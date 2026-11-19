//! Authenticated private cross-process protocol for DepthOS action security.
//!
//! Connects:
//! Rust durable coordinator <-> Authenticated private IPC <-> Private patched KWin authority
//! <-> Internal synthetic counting sink <-> Authenticated acknowledgment <-> Rust durable journal.
//!
//! # Invariants & Security Guarantees
//! 1. **Dedicated Unix Domain Socket**: Bound in a private `0700` directory with `0600` socket permissions.
//! 2. **Peer Credential Verification**: Enforces `SO_PEERCRED` checks via `rustix::net::sockopt::socket_peercred`.
//!    The peer UID must strictly match the current EUID (`geteuid()`), and PID must be strictly positive.
//! 3. **Bootstrap Token Handshake**: First message MUST be an authenticated `Hello` carrying an unguessable
//!    launcher-issued bootstrap token. Ordinary same-UID processes without this token are closed immediately.
//! 4. **Strict Wire Framing**: `[Magic: 4 bytes (0x44504F53)] [Version: 2 bytes (0x0100)] [MsgType: 2 bytes] [Seq: 8 bytes] [PayloadLen: 4 bytes] [Payload: N bytes]`.
//!    Payload length is strictly bounded to `<= 65,536` bytes (64 KiB). Any oversize packet causes immediate disconnect
//!    without allocating payload buffer.
//! 5. **Strict Monotonic Sequence**: Packets in each stream direction must have incrementing sequence numbers.
//! 6. **Finite-State Protocol Parser**: Disallowed state transitions trigger connection abort.
//! 7. **Linearized Intent Before Wire Transmission**: Dispatch intent is committed to SQLite WAL before any
//!    dispatch frame is sent over the wire.
//! 8. **Uncertainty Quarantine**: Lost ACK, timeout, or disconnect immediately transitions the operation into `Quarantined`,
//!    locking the window against concurrent mutations until authoritative reconciliation occurs.

use super::{
    Operation, RequestId, Tick, Uncertainty, WindowKey,
    durable::{
        DurableCoordinatorSession, DurableError, DurableOperationState, ReconciliationEvidence,
    },
    lease::{LeaseError, LeaseId, LeaseRequest, TerminalOutcome},
};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    time::timeout,
};
use uuid::Uuid;

pub const PROTOCOL_MAGIC: [u8; 4] = [0x44, 0x50, 0x4F, 0x53]; // "DPOS"
pub const PROTOCOL_VERSION: u16 = 0x0100; // v1.0
pub const MAX_PAYLOAD_LEN: usize = 65536; // 64 KiB
pub const HEADER_LEN: usize = 20;
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_millis(2000);

#[derive(thiserror::Error, Debug)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic: expected [0x44, 0x50, 0x4F, 0x53], got {0:?}")]
    InvalidMagic([u8; 4]),
    #[error("Unsupported protocol version: {0:#06x}")]
    UnsupportedVersion(u16),
    #[error("Unknown message type code: {0}")]
    UnknownMessageType(u16),
    #[error("Oversize payload: {0} bytes (max {MAX_PAYLOAD_LEN})")]
    OversizePayload(usize),
    #[error("Packet sequence mismatch: expected {expected}, got {got}")]
    SequenceMismatch { expected: u64, got: u64 },
    #[error("Invalid session state transition from {from:?} on message {msg_type:?}")]
    InvalidTransition {
        from: SessionState,
        msg_type: MessageType,
    },
    #[error("Peer credential check failed: {0}")]
    PeerCredFailed(String),
    #[error("Bootstrap token authentication failed")]
    AuthenticationFailed,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Protocol timeout reached")]
    Timeout,
    #[error("Connection closed by peer")]
    ConnectionClosed,
    #[error("Identity mismatch: {0}")]
    IdentityMismatch(String),
    #[error("Durable journal error: {0}")]
    Durable(#[from] DurableError),
    #[error("Coordinator lease error: {0:?}")]
    Lease(LeaseError),
    #[error("Operation quarantined: {0}")]
    Quarantined(String),
    #[error("Unsafe socket permissions or directory: {0}")]
    UnsafePermissions(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl From<LeaseError> for IpcError {
    fn from(err: LeaseError) -> Self {
        IpcError::Lease(err)
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MessageType {
    Hello = 1,
    HelloAccepted = 2,
    Reserve = 3,
    Prepared = 4,
    DispatchIntent = 5,
    SyntheticAttempt = 6,
    SyntheticResult = 7,
    Acknowledge = 8,
    Cancel = 9,
    Revoke = 10,
    QuarantineNotice = 11,
    ReconcileEvidence = 12,
    TerminalReceipt = 13,
}

impl MessageType {
    pub fn from_u16(val: u16) -> Option<Self> {
        match val {
            1 => Some(Self::Hello),
            2 => Some(Self::HelloAccepted),
            3 => Some(Self::Reserve),
            4 => Some(Self::Prepared),
            5 => Some(Self::DispatchIntent),
            6 => Some(Self::SyntheticAttempt),
            7 => Some(Self::SyntheticResult),
            8 => Some(Self::Acknowledge),
            9 => Some(Self::Cancel),
            10 => Some(Self::Revoke),
            11 => Some(Self::QuarantineNotice),
            12 => Some(Self::ReconcileEvidence),
            13 => Some(Self::TerminalReceipt),
            _ => None,
        }
    }

    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Unauthenticated,
    Active,
    Quarantined,
    Terminated,
}

// ---------------------------------------------------------------------------
// Typed Protocol Messages
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub bootstrap_token: String,
    pub client_pid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAcceptedMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub compositor_incarnation: Uuid,
    pub accepted_epoch: u64,
    pub server_pid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub window_key: WindowKey,
    pub native_uuid: Uuid,
    pub expiry_tick: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub compositor_incarnation: Uuid,
    pub window_generation: u64,
    pub ready: bool,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchIntentMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub intent_committed: bool,
    pub operation: Operation,
    pub operation_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticAttemptMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub attempt: u64,
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub compositor_incarnation: Uuid,
    pub window_key: WindowKey,
    pub native_uuid: Uuid,
    pub operation: Operation,
    pub params: String,
    pub fixture_authority_token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticResultMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub attempt: u64,
    pub coordinator_incarnation: Uuid,
    pub compositor_incarnation: Uuid,
    pub consumed_count: u64,
    pub outcome: String, // "Counted", "Denied", "Uncertain"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AckOutcome {
    Applied,
    Rejected,
    IndeterminateError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcknowledgeMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub attempt: u64,
    pub coordinator_incarnation: Uuid,
    pub compositor_incarnation: Uuid,
    pub outcome: AckOutcome,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub coordinator_incarnation: Uuid,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub lease_id: LeaseId,
    pub coordinator_incarnation: Uuid,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineNoticeMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub coordinator_incarnation: Uuid,
    pub window_key: WindowKey,
    pub native_uuid: Uuid,
    pub reason: String,
    pub quarantine_seq: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileEvidenceMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub evidence: ReconciliationEvidence,
    pub provenance: String, // "depthos.observer.authoritative"
    pub authority_token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalReceiptMessage {
    pub protocol_version: u16,
    pub session_id: Uuid,
    pub operation_id: RequestId,
    pub lease_id: LeaseId,
    pub final_state: DurableOperationState,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "msg_type", content = "payload")]
pub enum ProtocolMessage {
    Hello(HelloMessage),
    HelloAccepted(HelloAcceptedMessage),
    Reserve(ReserveMessage),
    Prepared(PreparedMessage),
    DispatchIntent(DispatchIntentMessage),
    SyntheticAttempt(SyntheticAttemptMessage),
    SyntheticResult(SyntheticResultMessage),
    Acknowledge(AcknowledgeMessage),
    Cancel(CancelMessage),
    Revoke(RevokeMessage),
    QuarantineNotice(QuarantineNoticeMessage),
    ReconcileEvidence(ReconcileEvidenceMessage),
    TerminalReceipt(TerminalReceiptMessage),
}

impl ProtocolMessage {
    pub fn message_type(&self) -> MessageType {
        match self {
            Self::Hello(_) => MessageType::Hello,
            Self::HelloAccepted(_) => MessageType::HelloAccepted,
            Self::Reserve(_) => MessageType::Reserve,
            Self::Prepared(_) => MessageType::Prepared,
            Self::DispatchIntent(_) => MessageType::DispatchIntent,
            Self::SyntheticAttempt(_) => MessageType::SyntheticAttempt,
            Self::SyntheticResult(_) => MessageType::SyntheticResult,
            Self::Acknowledge(_) => MessageType::Acknowledge,
            Self::Cancel(_) => MessageType::Cancel,
            Self::Revoke(_) => MessageType::Revoke,
            Self::QuarantineNotice(_) => MessageType::QuarantineNotice,
            Self::ReconcileEvidence(_) => MessageType::ReconcileEvidence,
            Self::TerminalReceipt(_) => MessageType::TerminalReceipt,
        }
    }

    pub fn session_id(&self) -> Uuid {
        match self {
            Self::Hello(m) => m.session_id,
            Self::HelloAccepted(m) => m.session_id,
            Self::Reserve(m) => m.session_id,
            Self::Prepared(m) => m.session_id,
            Self::DispatchIntent(m) => m.session_id,
            Self::SyntheticAttempt(m) => m.session_id,
            Self::SyntheticResult(m) => m.session_id,
            Self::Acknowledge(m) => m.session_id,
            Self::Cancel(m) => m.session_id,
            Self::Revoke(m) => m.session_id,
            Self::QuarantineNotice(m) => m.session_id,
            Self::ReconcileEvidence(m) => m.session_id,
            Self::TerminalReceipt(m) => m.session_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Framing Codec & Stream
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub msg_type: MessageType,
    pub sequence: u64,
    pub payload_len: u32,
}

pub struct IpcStream {
    stream: UnixStream,
    send_seq: u64,
    recv_seq: u64,
    session_state: SessionState,
    read_timeout: Duration,
}

impl IpcStream {
    pub fn new(stream: UnixStream, initial_state: SessionState) -> Self {
        Self {
            stream,
            send_seq: 1,
            recv_seq: 1,
            session_state: initial_state,
            read_timeout: DEFAULT_READ_TIMEOUT,
        }
    }

    pub fn session_state(&self) -> SessionState {
        self.session_state
    }

    pub fn set_session_state(&mut self, state: SessionState) {
        self.session_state = state;
    }

    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.read_timeout = timeout;
    }

    pub fn inner_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.stream.as_fd()
    }

    /// Send a protocol message with sequence number and strict framing.
    pub async fn send_message(&mut self, msg: &ProtocolMessage) -> Result<(), IpcError> {
        let payload = serde_json::to_vec(msg)?;
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(IpcError::OversizePayload(payload.len()));
        }

        let seq = self.send_seq;
        self.send_seq += 1;

        let mut buf = Vec::with_capacity(HEADER_LEN + payload.len());
        buf.extend_from_slice(&PROTOCOL_MAGIC);
        buf.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        buf.extend_from_slice(&msg.message_type().to_u16().to_be_bytes());
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&payload);

        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// Receive a protocol message with strict validation of magic, version, length, and sequence.
    pub async fn recv_message(&mut self) -> Result<ProtocolMessage, IpcError> {
        let mut header_buf = [0u8; HEADER_LEN];
        let read_res = timeout(self.read_timeout, self.stream.read_exact(&mut header_buf)).await;
        match read_res {
            Err(_) => return Err(IpcError::Timeout),
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(IpcError::ConnectionClosed);
            }
            Ok(Err(e)) => return Err(IpcError::Io(e)),
            Ok(Ok(_)) => {}
        }

        let magic: [u8; 4] = header_buf[0..4].try_into().unwrap();
        if magic != PROTOCOL_MAGIC {
            return Err(IpcError::InvalidMagic(magic));
        }

        let version = u16::from_be_bytes(header_buf[4..6].try_into().unwrap());
        if version != PROTOCOL_VERSION {
            return Err(IpcError::UnsupportedVersion(version));
        }

        let msg_type_raw = u16::from_be_bytes(header_buf[6..8].try_into().unwrap());
        let msg_type = MessageType::from_u16(msg_type_raw)
            .ok_or(IpcError::UnknownMessageType(msg_type_raw))?;

        let seq = u64::from_be_bytes(header_buf[8..16].try_into().unwrap());
        if seq != self.recv_seq {
            return Err(IpcError::SequenceMismatch {
                expected: self.recv_seq,
                got: seq,
            });
        }
        self.recv_seq += 1;

        let payload_len = u32::from_be_bytes(header_buf[16..20].try_into().unwrap()) as usize;
        if payload_len > MAX_PAYLOAD_LEN {
            // Immediate reject without allocating buffer
            return Err(IpcError::OversizePayload(payload_len));
        }

        let mut payload_buf = vec![0u8; payload_len];
        let payload_res =
            timeout(self.read_timeout, self.stream.read_exact(&mut payload_buf)).await;
        match payload_res {
            Err(_) => return Err(IpcError::Timeout),
            Ok(Err(e)) => return Err(IpcError::Io(e)),
            Ok(Ok(_)) => {}
        }

        // Validate state machine transitions
        self.validate_state_transition(msg_type)?;

        let msg: ProtocolMessage = serde_json::from_slice(&payload_buf)?;
        if msg.message_type() != msg_type {
            return Err(IpcError::Protocol(format!(
                "Header msg_type {msg_type:?} does not match deserialized msg {:?}",
                msg.message_type()
            )));
        }

        Ok(msg)
    }

    fn validate_state_transition(&mut self, msg_type: MessageType) -> Result<(), IpcError> {
        let res = match self.session_state {
            SessionState::Unauthenticated => match msg_type {
                MessageType::Hello | MessageType::HelloAccepted => Ok(()),
                _ => Err(IpcError::InvalidTransition {
                    from: self.session_state,
                    msg_type,
                }),
            },
            SessionState::Active => match msg_type {
                MessageType::Hello | MessageType::HelloAccepted => {
                    Err(IpcError::InvalidTransition {
                        from: self.session_state,
                        msg_type,
                    })
                }
                _ => Ok(()),
            },
            SessionState::Quarantined => match msg_type {
                MessageType::ReconcileEvidence
                | MessageType::TerminalReceipt
                | MessageType::QuarantineNotice => Ok(()),
                _ => Err(IpcError::InvalidTransition {
                    from: self.session_state,
                    msg_type,
                }),
            },
            SessionState::Terminated => Err(IpcError::InvalidTransition {
                from: self.session_state,
                msg_type,
            }),
        };
        if res.is_err() {
            self.session_state = SessionState::Terminated;
        }
        res
    }
}

// ---------------------------------------------------------------------------
// Security & Peer Credential Checks
// ---------------------------------------------------------------------------

pub fn verify_peer_credentials<F: std::os::fd::AsFd>(
    fd: F,
    expected_pid: Option<u32>,
) -> Result<(u32, u32, u32), IpcError> {
    let cred = rustix::net::sockopt::socket_peercred(fd)
        .map_err(|e| IpcError::PeerCredFailed(format!("Failed getsockopt SO_PEERCRED: {e}")))?;
    let peer_uid = cred.uid.as_raw();
    let peer_gid = cred.gid.as_raw();
    let raw_pid = cred.pid.as_raw_pid();

    let my_uid = rustix::process::geteuid().as_raw();
    if peer_uid != my_uid {
        return Err(IpcError::PeerCredFailed(format!(
            "Peer UID {peer_uid} does not match current user {my_uid}"
        )));
    }
    if raw_pid <= 0 {
        return Err(IpcError::PeerCredFailed(
            "Peer PID is zero or invalid".into(),
        ));
    }
    let peer_pid = raw_pid as u32;
    if let Some(exp_pid) = expected_pid
        && peer_pid != exp_pid
    {
        return Err(IpcError::PeerCredFailed(format!(
            "Peer PID {peer_pid} does not match expected PID {exp_pid}"
        )));
    }
    Ok((peer_uid, peer_gid, peer_pid))
}

pub fn verify_private_directory<P: AsRef<Path>>(dir: P) -> Result<(), IpcError> {
    let meta = std::fs::symlink_metadata(dir.as_ref())
        .map_err(|e| IpcError::UnsafePermissions(format!("Cannot inspect directory: {e}")))?;
    if meta.file_type().is_symlink() {
        return Err(IpcError::UnsafePermissions(
            "Directory must not be a symlink".into(),
        ));
    }
    if !meta.is_dir() {
        return Err(IpcError::UnsafePermissions(
            "Path is not a directory".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let my_uid = rustix::process::geteuid().as_raw();
        if meta.uid() != my_uid {
            return Err(IpcError::UnsafePermissions(format!(
                "Directory owner UID {} does not match current EUID {}",
                meta.uid(),
                my_uid
            )));
        }
        let mode = meta.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(IpcError::UnsafePermissions(format!(
                "Directory permissions {mode:#o} must be 0700 (no group or other access)"
            )));
        }
    }
    Ok(())
}

pub fn verify_private_socket<P: AsRef<Path>>(path: P) -> Result<(), IpcError> {
    let meta = std::fs::symlink_metadata(path.as_ref())
        .map_err(|e| IpcError::UnsafePermissions(format!("Cannot inspect socket: {e}")))?;
    if meta.file_type().is_symlink() {
        return Err(IpcError::UnsafePermissions(
            "Socket path must not be a symlink".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
        if !meta.file_type().is_socket() {
            return Err(IpcError::UnsafePermissions(
                "Path is not a unix domain socket".into(),
            ));
        }
        let my_uid = rustix::process::geteuid().as_raw();
        if meta.uid() != my_uid {
            return Err(IpcError::UnsafePermissions(format!(
                "Socket owner UID {} does not match current EUID {}",
                meta.uid(),
                my_uid
            )));
        }
        let mode = meta.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(IpcError::UnsafePermissions(format!(
                "Socket permissions {mode:#o} must be 0600 (no group or other access)"
            )));
        }
    }
    Ok(())
}

pub fn bind_private_listener<P: AsRef<Path>>(
    path: P,
) -> Result<tokio::net::UnixListener, IpcError> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        verify_private_directory(parent)?;
    }
    if p.exists() {
        let _ = std::fs::remove_file(p);
    }
    let listener = tokio::net::UnixListener::bind(p)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600))?;
    }
    verify_private_socket(p)?;
    Ok(listener)
}

// ---------------------------------------------------------------------------
// Cross-Process Coordinator Integration
// ---------------------------------------------------------------------------

pub struct CrossProcessCoordinator {
    pub session: DurableCoordinatorSession,
    pub session_id: Uuid,
    pub coordinator_incarnation: Uuid,
    pub coordinator_epoch: u64,
    pub compositor_incarnation: Option<Uuid>,
    pub bootstrap_token: String,
    pub attempt_counter: u64,
}

impl CrossProcessCoordinator {
    pub fn new(
        session: DurableCoordinatorSession,
        bootstrap_token: String,
    ) -> Result<Self, IpcError> {
        let coord_inc = session.lease_manager().coordinator_incarnation;
        let coord_epoch = session.lease_manager().coordinator_epoch;
        Ok(Self {
            session,
            session_id: Uuid::new_v4(),
            coordinator_incarnation: coord_inc,
            coordinator_epoch: coord_epoch,
            compositor_incarnation: None,
            bootstrap_token,
            attempt_counter: 0,
        })
    }

    /// Perform initial Hello handshake with the compositor over the provided IpcStream.
    pub async fn perform_handshake(
        &mut self,
        stream: &mut IpcStream,
        expected_compositor: Option<Uuid>,
    ) -> Result<HelloAcceptedMessage, IpcError> {
        verify_peer_credentials(stream.inner_fd(), None)?;

        let my_pid = std::process::id();
        let hello = HelloMessage {
            protocol_version: PROTOCOL_VERSION,
            session_id: self.session_id,
            coordinator_incarnation: self.coordinator_incarnation,
            coordinator_epoch: self.coordinator_epoch,
            bootstrap_token: self.bootstrap_token.clone(),
            client_pid: my_pid,
        };

        stream.send_message(&ProtocolMessage::Hello(hello)).await?;

        let response = stream.recv_message().await?;
        match response {
            ProtocolMessage::HelloAccepted(accepted) => {
                if accepted.session_id != self.session_id {
                    return Err(IpcError::IdentityMismatch("Session ID mismatch".into()));
                }
                if let Some(exp_comp) = expected_compositor
                    && accepted.compositor_incarnation != exp_comp
                {
                    return Err(IpcError::IdentityMismatch(
                        "Compositor incarnation mismatch".into(),
                    ));
                }
                self.compositor_incarnation = Some(accepted.compositor_incarnation);
                stream.set_session_state(SessionState::Active);
                Ok(accepted)
            }
            other => Err(IpcError::Protocol(format!(
                "Expected HelloAccepted, got {other:?}"
            ))),
        }
    }

    /// Dispatches an operation across the IPC boundary with strict durable intent persistence,
    /// linearized point-of-no-return commitment, at-most-once delivery, and uncertainty quarantine.
    pub async fn dispatch_operation(
        &mut self,
        stream: &mut IpcStream,
        request: LeaseRequest,
        operation: Operation,
        fixture_token: String,
        now: Tick,
    ) -> Result<DurableOperationState, IpcError> {
        let op_id = request.operation_id;
        let window_key = request.window.clone();
        let native_uuid = request.native_uuid;
        let comp_inc = self
            .compositor_incarnation
            .ok_or_else(|| IpcError::Protocol("Coordinator handshake not completed".into()))?;

        // 1. Acquire durable lease reservation (LP-1 & LP-2)
        let lease_id = self.session.acquire(request, now, comp_inc)?;

        // 2. Wire message: Reserve
        let reservation_state = self
            .session
            .lease_manager()
            .get_reservation_state(&lease_id)
            .ok_or_else(|| IpcError::Protocol("Reservation state not found".into()))?;
        let expires_at = match reservation_state {
            crate::action_security::lease::LeaseState::Active(r) => r.expires_at,
            _ => return Err(IpcError::Protocol("Reservation not active".into())),
        };

        let reserve_msg = ReserveMessage {
            protocol_version: PROTOCOL_VERSION,
            session_id: self.session_id,
            operation_id: op_id,
            lease_id,
            coordinator_incarnation: self.coordinator_incarnation,
            coordinator_epoch: self.coordinator_epoch,
            window_key: window_key.clone(),
            native_uuid,
            expiry_tick: expires_at,
        };

        if let Err(e) = stream
            .send_message(&ProtocolMessage::Reserve(reserve_msg))
            .await
        {
            self.session.abort_lease(
                lease_id,
                op_id,
                &format!("Failed to send Reserve: {e}"),
                now,
            )?;
            return Ok(DurableOperationState::Aborted);
        }

        // 3. Await Prepared
        let prepared_resp = stream.recv_message().await;
        let prepared = match prepared_resp {
            Ok(ProtocolMessage::Prepared(p)) => {
                if p.operation_id != op_id
                    || p.lease_id != lease_id
                    || p.compositor_incarnation != comp_inc
                {
                    self.session.abort_lease(
                        lease_id,
                        op_id,
                        "Prepared message correlation mismatch",
                        now,
                    )?;
                    return Ok(DurableOperationState::Aborted);
                }
                if !p.ready {
                    self.session.abort_lease(
                        lease_id,
                        op_id,
                        &format!("Compositor not ready: {:?}", p.detail),
                        now,
                    )?;
                    return Ok(DurableOperationState::Aborted);
                }
                p
            }
            Ok(other) => {
                self.session.abort_lease(
                    lease_id,
                    op_id,
                    &format!("Expected Prepared, got {other:?}"),
                    now,
                )?;
                return Ok(DurableOperationState::Aborted);
            }
            Err(e) => {
                self.session.abort_lease(
                    lease_id,
                    op_id,
                    &format!("Transport error awaiting Prepared: {e}"),
                    now,
                )?;
                return Ok(DurableOperationState::Aborted);
            }
        };

        // 4. POINT OF NO RETURN: Commit Dispatch Intent to SQLite WAL with PRAGMA synchronous = FULL (LP-3)
        self.session.journal_mut().prepare_operation(
            op_id,
            now,
            self.coordinator_incarnation,
            self.coordinator_epoch,
            comp_inc,
        )?;
        self.session
            .journal_mut()
            .persist_dispatch_intent(op_id, now)?;

        // 5. Send DispatchIntent over wire
        let intent_msg = DispatchIntentMessage {
            protocol_version: PROTOCOL_VERSION,
            session_id: self.session_id,
            operation_id: op_id,
            lease_id,
            coordinator_incarnation: self.coordinator_incarnation,
            coordinator_epoch: self.coordinator_epoch,
            intent_committed: true,
            operation: operation.clone(),
            operation_digest: format!("{:?}", operation),
        };
        if let Err(e) = stream
            .send_message(&ProtocolMessage::DispatchIntent(intent_msg))
            .await
        {
            // Post-intent transport failure: quarantine!
            self.session.quarantine_lease(
                lease_id,
                op_id,
                &format!("Send DispatchIntent failed: {e}"),
                Uncertainty::NoAcknowledgement,
                now,
            )?;
            stream.set_session_state(SessionState::Quarantined);
            return Ok(DurableOperationState::Quarantined);
        }

        // 6. Transmit SyntheticAttempt (LP-4)
        self.attempt_counter += 1;
        let attempt = self.attempt_counter;
        let attempt_msg = SyntheticAttemptMessage {
            protocol_version: PROTOCOL_VERSION,
            session_id: self.session_id,
            operation_id: op_id,
            lease_id,
            attempt,
            coordinator_incarnation: self.coordinator_incarnation,
            coordinator_epoch: self.coordinator_epoch,
            compositor_incarnation: comp_inc,
            window_key: window_key.clone(),
            native_uuid,
            operation: operation.clone(),
            params: format!("gen={}", prepared.window_generation),
            fixture_authority_token: fixture_token,
        };

        // Record attempt in SQLite journal
        self.session
            .journal_mut()
            .mark_synthetic_dispatch_attempted(op_id)?;

        if let Err(e) = stream
            .send_message(&ProtocolMessage::SyntheticAttempt(attempt_msg))
            .await
        {
            self.session.quarantine_lease(
                lease_id,
                op_id,
                &format!("Send SyntheticAttempt failed: {e}"),
                Uncertainty::NoAcknowledgement,
                now,
            )?;
            stream.set_session_state(SessionState::Quarantined);
            return Ok(DurableOperationState::Quarantined);
        }

        // 7. Await Correlated Acknowledgment (LP-5)
        let ack_res = stream.recv_message().await;
        match ack_res {
            Ok(ProtocolMessage::Acknowledge(ack)) => {
                if ack.operation_id != op_id
                    || ack.lease_id != lease_id
                    || ack.attempt != attempt
                    || ack.coordinator_incarnation != self.coordinator_incarnation
                    || ack.compositor_incarnation != comp_inc
                {
                    // Corrupted or mismatched ACK fails closed into quarantine
                    self.session.quarantine_lease(
                        lease_id,
                        op_id,
                        "Mismatched acknowledgment identities",
                        Uncertainty::PostconditionMismatch,
                        now,
                    )?;
                    stream.set_session_state(SessionState::Quarantined);
                    return Ok(DurableOperationState::Quarantined);
                }

                match ack.outcome {
                    AckOutcome::Applied => {
                        self.session.complete_lease(
                            lease_id,
                            op_id,
                            TerminalOutcome::Applied,
                            comp_inc,
                            now,
                        )?;
                        let receipt = TerminalReceiptMessage {
                            protocol_version: PROTOCOL_VERSION,
                            session_id: self.session_id,
                            operation_id: op_id,
                            lease_id,
                            final_state: DurableOperationState::Completed,
                            timestamp_ms: now.0,
                        };
                        let _ = stream
                            .send_message(&ProtocolMessage::TerminalReceipt(receipt))
                            .await;
                        Ok(DurableOperationState::Completed)
                    }
                    AckOutcome::Rejected => {
                        self.session.complete_lease(
                            lease_id,
                            op_id,
                            TerminalOutcome::Rejected,
                            comp_inc,
                            now,
                        )?;
                        let receipt = TerminalReceiptMessage {
                            protocol_version: PROTOCOL_VERSION,
                            session_id: self.session_id,
                            operation_id: op_id,
                            lease_id,
                            final_state: DurableOperationState::Completed,
                            timestamp_ms: now.0,
                        };
                        let _ = stream
                            .send_message(&ProtocolMessage::TerminalReceipt(receipt))
                            .await;
                        Ok(DurableOperationState::Completed)
                    }
                    AckOutcome::IndeterminateError => {
                        self.session.quarantine_lease(
                            lease_id,
                            op_id,
                            &format!("Compositor IndeterminateError: {:?}", ack.reason),
                            Uncertainty::PartialExecution,
                            now,
                        )?;
                        stream.set_session_state(SessionState::Quarantined);
                        Ok(DurableOperationState::Quarantined)
                    }
                }
            }
            Ok(ProtocolMessage::SyntheticResult(res)) => {
                if res.outcome == "Counted" {
                    self.session.complete_lease(
                        lease_id,
                        op_id,
                        TerminalOutcome::Applied,
                        comp_inc,
                        now,
                    )?;
                    Ok(DurableOperationState::Completed)
                } else {
                    self.session.complete_lease(
                        lease_id,
                        op_id,
                        TerminalOutcome::Rejected,
                        comp_inc,
                        now,
                    )?;
                    Ok(DurableOperationState::Completed)
                }
            }
            Err(e) => {
                // LP-6: Lost ACK / Timeout triggers Uncertainty Quarantine
                self.session.quarantine_lease(
                    lease_id,
                    op_id,
                    &format!("Lost acknowledgment or transport error: {e}"),
                    Uncertainty::NoAcknowledgement,
                    now,
                )?;
                stream.set_session_state(SessionState::Quarantined);
                let notice = QuarantineNoticeMessage {
                    protocol_version: PROTOCOL_VERSION,
                    session_id: self.session_id,
                    operation_id: op_id,
                    lease_id,
                    coordinator_incarnation: self.coordinator_incarnation,
                    window_key,
                    native_uuid,
                    reason: format!("Transport error or lost ACK: {e}"),
                    quarantine_seq: 1,
                };
                let _ = stream
                    .send_message(&ProtocolMessage::QuarantineNotice(notice))
                    .await;
                Ok(DurableOperationState::Quarantined)
            }
            Ok(other) => {
                self.session.quarantine_lease(
                    lease_id,
                    op_id,
                    &format!("Unexpected message instead of Acknowledge: {other:?}"),
                    Uncertainty::PostconditionMismatch,
                    now,
                )?;
                stream.set_session_state(SessionState::Quarantined);
                Ok(DurableOperationState::Quarantined)
            }
        }
    }
}
