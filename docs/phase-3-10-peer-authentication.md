# Phase 3.10 Transport Security & Peer Authentication Specification

**Status**: Implemented & Independently Verified  
**Date**: November 2026  
**Scope**: Transport Security, Peer Credential Verification, Bootstrap Token Handshake, and Incarnation Binding  

---

## 1. Executive Summary & Purpose

Phase 3.10 establishes the transport-level security and peer authentication mechanisms for the private IPC channel connecting the **DepthOS Coordinator**, **KWin Compositor Synthetic Endpoint**, and **DepthOS Observer**.

Because all components execute on the same local host under the same user UID, standard Unix file permissions alone are insufficient to isolate the channel from other unprivileged processes running under that same user account. This document formalizes the multi-layered defense-in-depth model that guarantees mutual peer verification and rejects unauthorized local actors.

---

## 2. Multi-Layered Transport Security Model

Transport security is established through five defense-in-depth boundaries:

```
[Private Filesystem Boundary]  ──> Directory: 0700 | Socket: 0600
         │
         ▼
[Kernel Credential Boundary]   ──> SO_PEERCRED: Peer UID == EUID & Peer PID > 0
         │
         ▼
[Bootstrap Token Boundary]     ──> 256-bit Ephemeral CSPRNG Launcher Token
         │
         ▼
[Process Incarnation Boundary] ──> Coordinator Incarnation + Compositor Incarnation
         │
         ▼
[Stream Monotonicity Boundary] ──> Frame Sequence Check (seq == expected_seq)
```

---

## 3. Detailed Boundary Specifications

### 3.1 Private Directory & Socket File Permissions
1. **Private Base Directory**: The Unix Domain Socket is created under a dedicated private directory (e.g. `/run/user/$UID/depthos/` or a secure temporary path).
   - Mode verification: `verify_private_directory()` enforces `mode & 0o077 == 0` (`0700`). Any permission granting group or others access is refused.
   - Symlink prevention: The directory path must NOT be a symlink (`file_type().is_symlink() == false`).
2. **Socket Node Permissions**:
   - On server bind, the socket node permissions are set to `0600` (`S_IRUSR | S_IWUSR`).
   - Group and world read/write/execute are strictly disabled.

### 3.2 Kernel Peer Credential Verification (`SO_PEERCRED`)
Upon connection acceptance (`accept()`), the receiving endpoint extracts kernel peer credentials:
- In Rust: `rustix::net::sockopt::socket_peercred(stream.as_fd())`.
- In C++: `getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &cred, &len)`.

**Validation Invariants**:
1. `peer_uid == geteuid()`: Peer user ID must strictly equal the current process effective UID. Connections from any other user (including root or foreign local accounts) fail immediately.
2. `peer_pid > 0`: Peer process ID must be non-zero and valid.
3. Optional executable path inspection: `/proc/$PID/exe` is inspected via `readlink` where available to ensure the binary path matches the expected process identity.

### 3.3 Ephemeral Bootstrap Token Handshake
Because malicious processes executing under the same UID have equal kernel UID credentials, UID checks alone cannot prevent same-user attacks. To defeat unauthorized local actors:
1. **Launcher Generation**: When spawning coordinator and compositor instances, the launcher generates high-entropy (256-bit) ephemeral bootstrap tokens using a CSPRNG.
2. **Private Injection**: The token is transferred exclusively via private inherited file descriptors or secure environment configurations.
3. **First-Frame Enforcement**: The very first message transmitted on a newly established connection MUST be a `Hello` packet containing the matching `bootstrap_token`.
4. **Immediate Fail-Closed Disconnect**: If any other message type arrives, if the token is missing, or if the token fails comparison:
   - The connection is closed immediately (`drop(stream)` / `close(fd)`).
   - No descriptive error packet is transmitted to the client, preventing information leakage or oracle attacks.

### 3.4 Multi-Process Incarnation & Epoch Fencing
1. **Coordinator Incarnation & Epoch**:
   - Each coordinator restart increments the durable epoch counter in SQLite and issues a fresh `coordinator_incarnation` UUID.
   - The coordinator attaches its incarnation and epoch to the `Hello` message and all subsequent requests.
2. **Compositor Incarnation**:
   - KWin creates a unique `compositor_incarnation` UUID at startup.
   - Upon successful handshake, the compositor returns `HelloAccepted` binding its incarnation UUID.
   - If KWin restarts, the new instance generates a fresh incarnation UUID. When the coordinator detects the mismatch, it immediately fences all in-flight leases from the prior incarnation into `Quarantined`.

### 3.5 Packet Sequence Monotonicity & Replay Immunity
1. Each stream direction maintains an incrementing `u64` sequence number starting at 1.
2. The receiver validates that `received_seq == expected_seq`, incrementing `expected_seq` on valid delivery.
3. Any skipped, duplicate, or regressing packet sequence immediately aborts the connection (`IpcError::SequenceMismatch`).
4. Operation-level replay immunity is guaranteed by primary key uniqueness constraints on `operation_id` within the durable SQLite journal.

---

## 4. Contract Boundaries & Architectural Limitations

1. **Shared Address Space Limitation**: In-process code within the monolithic KWin compositor shares memory with all loaded effects and plugins. True physical memory isolation between plugins does not exist in-process without OS-level process boundaries.
2. **Software Token Qualification**: Incarnation UUIDs and bootstrap tokens are software CSPRNG identifiers. They are not hardware-backed cryptographic signatures.
3. **Multi-Seat Hardware Qualification**: Multi-seat hardware configurations require bare-metal seat and device qualification; multi-seat operation is formally qualified as UNSUPPORTED in virtual testbeds.
4. **Power-Loss Safety Qualification**: Sudden hardware power cut (Class E) cannot be guaranteed in virtualized test environments and is formally qualified as UNSUPPORTED.
