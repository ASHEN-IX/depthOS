// Experimental deny-only stub. NOT a KWin plugin, proof issuer or native adapter.
// The positive kernel exists only in the explicitly compiled unit-test binary.
#pragma once

#include <QObject>
#include <QPointer>
#include <QThread>
#include <cstdint>
#include <chrono>
#include <limits>
#include <optional>
#include <set>
#include <string>
#include <vector>

#ifdef DEPTHOS_AUTHORITY_UNIT_TEST
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <endian.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <cstring>
#endif

namespace depthos::experiment {

enum class Origin { Unknown, ClaimedPhysical, Qt, Qml, Eis, Remote, Accessibility };
enum class Result { Denied, Counted, Uncertain };

// No caller-supplied origin, sender name, Qt event or boolean can authorize.
class DenyOnlyAuthority final {
public:
    Result confirm(Origin) const noexcept { return Result::Denied; }
};

#ifdef DEPTHOS_AUTHORITY_UNIT_TEST
namespace synthetic {

enum class LockPhase { Unlocked, AboutToLock, Locked };

enum class InputProvenance {
    Unknown,
    LocalCandidate,
    Eis,
    Portal,
    FakeOrVirtual,
    Accessibility,
    Remote
};

// These values are owned by the test, never observations received over IPC.
struct Window {
    QPointer<QObject> object;
    std::string uuid;
    std::uint64_t generation = 1;
    std::string desktop = "source";
    std::string activity = "activity";
    bool eligible = true;
};

// Phase 3.8: Compositor-owned prompt representation
struct CompositorPrompt {
    const std::string proposalId;
    const QPointer<QObject> window;
    const std::string windowUuid;
    const std::uint64_t windowGeneration;
    const std::string sourceDesktop;
    const std::string destinationDesktop;
    const std::string activity;
    const std::uint64_t compositorIncarnation;
    const std::chrono::steady_clock::time_point deadline;
    bool rendered = false;
    bool visible = false;
    bool dismissed = false;
    std::string dismissReason;

    CompositorPrompt(std::string propId, QObject *win, std::string wUuid,
                     std::uint64_t wGen, std::string src, std::string dst,
                     std::string act, std::uint64_t compInc,
                     std::chrono::steady_clock::time_point dl)
        : proposalId(std::move(propId))
        , window(win)
        , windowUuid(std::move(wUuid))
        , windowGeneration(wGen)
        , sourceDesktop(std::move(src))
        , destinationDesktop(std::move(dst))
        , activity(std::move(act))
        , compositorIncarnation(compInc)
        , deadline(dl)
    {}

    void markRendered() {
        rendered = true;
        visible = true;
    }
    void dismiss(const std::string &reason) {
        visible = false;
        dismissed = true;
        dismissReason = reason;
    }
};

struct World {
    std::vector<Window> windows;
    QPointer<QObject> focus;
    QPointer<QObject> prompt;
    std::set<std::string> desktops{"source", "destination"};
    std::string currentDesktop = "source";
    std::string activity = "activity";
    std::string seat = "seat0";
    std::string activeSeat = "seat0";
    std::string deviceSeat = "seat0";
    std::uint64_t compositor = 1, coordinator = 1, observer = 1, topology = 1;
    std::optional<bool> locked = false;
    LockPhase lockPhase = LockPhase::Unlocked;
    bool promptOwned = true;
    bool promptRendered = true;
    bool promptVisible = true;
    bool alive = true;
    bool multiSeatQualified = false; // Phase 3.8: native multi-seat qualification unsupported
};

struct Proposal {
    const QPointer<QObject> object;
    const QPointer<QObject> prompt;
    const std::string uuid, source, destination, activity, seat;
    const std::uint64_t generation, compositor, coordinator, observer, topology, serial;
    const std::chrono::steady_clock::time_point issued, deadline;
    const std::string proposalId;
};

// All methods and all World mutation must run on the creating thread. No
// callbacks, setters, IPC or event-loop entry occur in check/consume/count.
// QObject destruction is observed with QPointer; this is NOT KWin::Window.
class Kernel final {
public:
    using Clock = std::chrono::steady_clock;
    Kernel() : thread_(QThread::currentThread()) {}
    Kernel(const Kernel &) = delete;
    Kernel &operator=(const Kernel &) = delete;

    bool prepare(const World &world, QObject *object, const std::string &destination,
                 Clock::time_point now = Clock::now(), const std::string &proposalId = "prop-1") {
        if (!onThread() || pending_ || retired_ || serial_ == std::numeric_limits<std::uint64_t>::max() ||
            now > Clock::time_point::max() - std::chrono::seconds(5))
            return false;
        const Window *window = unique(world, object);
        if (!window || !live(world, *window, destination)) return false;
        pending_.emplace(Proposal{window->object, world.prompt, window->uuid,
            window->desktop, destination, window->activity, world.seat,
            window->generation, world.compositor, world.coordinator, world.observer,
            world.topology, ++serial_, now, now + std::chrono::seconds(5), proposalId});
        return true;
    }

    // A compile-time-only token comparison models protected input; it DOES NOT
    // authenticate a human, native input, a physical device or a compositor.
    Result syntheticConfirm(const World &world, std::uint64_t serial,
                            bool loseAcknowledgement = false,
                            Clock::time_point now = Clock::now()) {
        if (!onThread() || !pending_ || serial != pending_->serial) return Result::Denied;
        const Proposal proposal = *pending_;
        pending_.reset(); // Terminal even if validation fails or receipt is lost.
        retired_ = true; // One proposal per instance; no retry after uncertainty.
        const Window *window = unique(world, proposal.object.data());
        if (now < proposal.issued || now >= proposal.deadline ||
            !window || !live(world, *window, proposal.destination) ||
            window->uuid != proposal.uuid || window->generation != proposal.generation ||
            window->desktop != proposal.source || window->activity != proposal.activity ||
            world.prompt != proposal.prompt || world.seat != proposal.seat ||
            world.seat != world.activeSeat || world.deviceSeat != world.activeSeat ||
            !world.promptRendered || !world.promptVisible ||
            world.compositor != proposal.compositor || world.coordinator != proposal.coordinator ||
            world.observer != proposal.observer || world.topology != proposal.topology)
            return Result::Denied;
        ++count_; // Redacted in-memory count only; no native or persistent setter.
        return loseAcknowledgement ? Result::Uncertain : Result::Counted;
    }

    bool cancel() {
        if (!onThread()) return false;
        const bool hadPending = pending_.has_value();
        pending_.reset();
        retired_ = true;
        return hadPending;
    }
    void teardown() {
        if (!onThread()) return;
        pending_.reset();
        retired_ = true;
    }
    std::uint64_t serial() const { return onThread() ? serial_ : 0; }
    std::uint64_t count() const { return onThread() ? count_ : 0; }
    bool isPending() const { return onThread() && pending_.has_value(); }
    const std::optional<Proposal> &pending() const { return pending_; }

    // Phase 3.C: Input provenance classifier test method
    // Invariant: source classification is diagnostic evidence only, NEVER human consent.
    static Result classifyInputOrigin(InputProvenance origin) {
        (void)origin;
        return Result::Denied;
    }

private:
    bool onThread() const { return QThread::currentThread() == thread_; }
    static const Window *unique(const World &world, QObject *object) {
        if (!object) return nullptr;
        const Window *found = nullptr;
        for (const auto &window : world.windows) {
            if (window.object.data() == object) {
                if (found) return nullptr;
                found = &window;
            }
        }
        if (!found || found->uuid.empty() || found->generation == 0) return nullptr;
        for (const auto &other : world.windows) {
            if (&other != found && other.object && other.uuid == found->uuid) return nullptr;
        }
        return found;
    }
    bool live(const World &world, const Window &window, const std::string &destination) const {
        return world.alive && world.locked.has_value() && !*world.locked &&
            world.lockPhase == LockPhase::Unlocked &&
            window.object && window.object->thread() == thread_ &&
            window.eligible && world.focus == window.object &&
            world.prompt && world.prompt->thread() == thread_ &&
            world.promptOwned && world.promptRendered && world.promptVisible &&
            !world.seat.empty() && world.seat == world.activeSeat &&
            world.deviceSeat == world.activeSeat &&
            world.compositor && world.coordinator && world.observer && world.topology &&
            window.activity == world.activity && !world.activity.empty() &&
            window.desktop == world.currentDesktop &&
            world.desktops.contains(window.desktop) && world.desktops.contains(destination) &&
            destination != window.desktop;
    }
    QThread *const thread_;
    std::optional<Proposal> pending_;
    std::uint64_t serial_ = 0, count_ = 0;
    bool retired_ = false;
};

// Phase 3.10: Authenticated private synthetic IPC endpoint.
// Connects Rust durable coordinator to KWin synthetic counting sink.
class SyntheticAuthorityServer final {
public:
    static constexpr std::uint32_t PROTOCOL_MAGIC = 0x44504F53;
    static constexpr std::uint16_t PROTOCOL_VERSION = 0x0100;
    static constexpr std::size_t MAX_PAYLOAD_LEN = 65536;
    static constexpr std::size_t HEADER_LEN = 20;

    enum class MessageType : std::uint16_t {
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
        TerminalReceipt = 13
    };

    SyntheticAuthorityServer(Kernel &kernel,
                             World &world,
                             std::string bootstrapToken,
                             std::string fixtureAuthorityToken,
                             std::string compositorIncarnation)
        : kernel_(kernel)
        , world_(world)
        , bootstrapToken_(std::move(bootstrapToken))
        , fixtureAuthorityToken_(std::move(fixtureAuthorityToken))
        , compositorIncarnation_(std::move(compositorIncarnation))
    {}

    static bool verifyPeerCredentials(int fd) {
        struct ucred cred;
        socklen_t len = sizeof(cred);
        if (getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &cred, &len) != 0) {
            return false;
        }
        return cred.uid == geteuid() && cred.pid > 0;
    }

    bool readFrame(int fd, MessageType &msgType, std::uint64_t &seq, std::string &payload) {
        char headerBuf[HEADER_LEN];
        ssize_t n = readAll(fd, headerBuf, HEADER_LEN);
        if (n != static_cast<ssize_t>(HEADER_LEN)) {
            return false;
        }

        uint32_t magic = 0;
        uint16_t ver = 0;
        uint16_t typeCode = 0;
        uint64_t s = 0;
        uint32_t payloadLen = 0;

        std::memcpy(&magic, &headerBuf[0], 4);
        std::memcpy(&ver, &headerBuf[4], 2);
        std::memcpy(&typeCode, &headerBuf[6], 2);
        std::memcpy(&s, &headerBuf[8], 8);
        std::memcpy(&payloadLen, &headerBuf[16], 4);

        if (be32toh(magic) != PROTOCOL_MAGIC || be16toh(ver) != PROTOCOL_VERSION) {
            return false;
        }

        uint32_t hostLen = be32toh(payloadLen);
        if (hostLen > MAX_PAYLOAD_LEN) {
            return false; // Reject oversize immediately without allocating
        }

        uint64_t hostSeq = be64toh(s);
        if (hostSeq != expectedRecvSeq_) {
            return false; // Packet replay or sequence gap
        }
        expectedRecvSeq_++;

        msgType = static_cast<MessageType>(be16toh(typeCode));
        seq = hostSeq;

        if (hostLen > 0) {
            payload.resize(hostLen);
            n = readAll(fd, payload.data(), hostLen);
            if (n != static_cast<ssize_t>(hostLen)) {
                return false;
            }
        } else {
            payload.clear();
        }
        return true;
    }

    bool sendFrame(int fd, MessageType msgType, const std::string &payload) {
        if (payload.size() > MAX_PAYLOAD_LEN) {
            return false;
        }

        char headerBuf[HEADER_LEN];
        uint32_t magic = htobe32(PROTOCOL_MAGIC);
        uint16_t ver = htobe16(PROTOCOL_VERSION);
        uint16_t typeCode = htobe16(static_cast<uint16_t>(msgType));
        uint64_t s = htobe64(sendSeq_++);
        uint32_t len = htobe32(static_cast<uint32_t>(payload.size()));

        std::memcpy(&headerBuf[0], &magic, 4);
        std::memcpy(&headerBuf[4], &ver, 2);
        std::memcpy(&headerBuf[6], &typeCode, 2);
        std::memcpy(&headerBuf[8], &s, 8);
        std::memcpy(&headerBuf[16], &len, 4);

        if (writeAll(fd, headerBuf, HEADER_LEN) != static_cast<ssize_t>(HEADER_LEN)) {
            return false;
        }
        if (!payload.empty()) {
            if (writeAll(fd, payload.data(), payload.size()) != static_cast<ssize_t>(payload.size())) {
                return false;
            }
        }
        return true;
    }

    bool processMessage(int fd, Kernel::Clock::time_point now = Kernel::Clock::now()) {
        MessageType msgType;
        std::uint64_t seq = 0;
        std::string payload;

        if (!readFrame(fd, msgType, seq, payload)) {
            return false;
        }

        QJsonParseError err;
        QJsonDocument doc = QJsonDocument::fromJson(QByteArray::fromRawData(payload.data(), static_cast<int>(payload.size())), &err);
        if (err.error != QJsonParseError::NoError || !doc.isObject()) {
            return false;
        }

        QJsonObject root = doc.object();
        QJsonObject content = root["payload"].toObject();

        if (!authenticated_) {
            if (msgType != MessageType::Hello) {
                return false;
            }
            std::string token = content["bootstrap_token"].toString().toStdString();
            if (token != bootstrapToken_) {
                return false; // Authentication failed: disconnect
            }
            activeSessionId_ = content["session_id"].toString().toStdString();
            coordinatorIncarnation_ = content["coordinator_incarnation"].toString().toStdString();
            coordinatorEpoch_ = static_cast<std::uint64_t>(content["coordinator_epoch"].toInteger());
            authenticated_ = true;

            // Reply with HelloAccepted
            QJsonObject replyPayload;
            replyPayload["protocol_version"] = static_cast<int>(PROTOCOL_VERSION);
            replyPayload["session_id"] = QString::fromStdString(activeSessionId_);
            replyPayload["compositor_incarnation"] = QString::fromStdString(compositorIncarnation_);
            replyPayload["accepted_epoch"] = static_cast<qint64>(coordinatorEpoch_);
            replyPayload["server_pid"] = static_cast<qint64>(getpid());

            QJsonObject replyRoot;
            replyRoot["msg_type"] = "HelloAccepted";
            replyRoot["payload"] = replyPayload;

            QByteArray outBytes = QJsonDocument(replyRoot).toJson(QJsonDocument::Compact);
            return sendFrame(fd, MessageType::HelloAccepted, std::string(outBytes.constData(), outBytes.size()));
        }

        switch (msgType) {
        case MessageType::Reserve: {
            std::string opId = content["operation_id"].toObject()["0"].toString().toStdString();
            if (opId.empty()) opId = content["operation_id"].toString().toStdString();
            std::string nativeUuid = content["native_uuid"].toString().toStdString();

            // Find matching window in world
            const Window *matchedWindow = nullptr;
            for (const auto &w : world_.windows) {
                if (w.uuid == nativeUuid) {
                    matchedWindow = &w;
                    break;
                }
            }

            bool ready = (matchedWindow != nullptr && matchedWindow->eligible && !kernel_.isPending());
            if (ready) {
                std::string dest = "destination";
                kernel_.prepare(world_, matchedWindow->object.data(), dest, now, opId);
            }

            QJsonObject replyPayload;
            replyPayload["protocol_version"] = static_cast<int>(PROTOCOL_VERSION);
            replyPayload["session_id"] = QString::fromStdString(activeSessionId_);
            replyPayload["operation_id"] = content["operation_id"];
            replyPayload["lease_id"] = content["lease_id"];
            replyPayload["compositor_incarnation"] = QString::fromStdString(compositorIncarnation_);
            replyPayload["window_generation"] = static_cast<qint64>(matchedWindow ? matchedWindow->generation : 0);
            replyPayload["ready"] = ready;
            if (!ready) {
                replyPayload["detail"] = "Window not found or ineligible";
            }

            QJsonObject replyRoot;
            replyRoot["msg_type"] = "Prepared";
            replyRoot["payload"] = replyPayload;

            QByteArray outBytes = QJsonDocument(replyRoot).toJson(QJsonDocument::Compact);
            return sendFrame(fd, MessageType::Prepared, std::string(outBytes.constData(), outBytes.size()));
        }
        case MessageType::DispatchIntent: {
            return true;
        }
        case MessageType::SyntheticAttempt: {
            std::string fixtureToken = content["fixture_authority_token"].toString().toStdString();
            std::string compInc = content["compositor_incarnation"].toString().toStdString();
            std::string coordInc = content["coordinator_incarnation"].toString().toStdString();

            bool tokensValid = (fixtureToken == fixtureAuthorityToken_ &&
                                compInc == compositorIncarnation_ &&
                                coordInc == coordinatorIncarnation_);

            Result outcomeResult = Result::Denied;
            if (tokensValid) {
                outcomeResult = kernel_.syntheticConfirm(world_, kernel_.serial(), false, now);
            }

            QJsonObject replyPayload;
            replyPayload["protocol_version"] = static_cast<int>(PROTOCOL_VERSION);
            replyPayload["session_id"] = QString::fromStdString(activeSessionId_);
            replyPayload["operation_id"] = content["operation_id"];
            replyPayload["lease_id"] = content["lease_id"];
            replyPayload["attempt"] = content["attempt"];
            replyPayload["coordinator_incarnation"] = QString::fromStdString(coordinatorIncarnation_);
            replyPayload["compositor_incarnation"] = QString::fromStdString(compositorIncarnation_);

            if (outcomeResult == Result::Counted) {
                replyPayload["outcome"] = "Applied";
            } else if (outcomeResult == Result::Uncertain) {
                replyPayload["outcome"] = "IndeterminateError";
                replyPayload["reason"] = "Uncertain execution";
            } else {
                replyPayload["outcome"] = "Rejected";
                replyPayload["reason"] = tokensValid ? "Denied by synthetic kernel" : "Invalid tokens or incarnation";
            }

            QJsonObject replyRoot;
            replyRoot["msg_type"] = "Acknowledge";
            replyRoot["payload"] = replyPayload;

            QByteArray outBytes = QJsonDocument(replyRoot).toJson(QJsonDocument::Compact);
            return sendFrame(fd, MessageType::Acknowledge, std::string(outBytes.constData(), outBytes.size()));
        }
        case MessageType::Cancel:
        case MessageType::QuarantineNotice: {
            kernel_.cancel();
            return true;
        }
        default:
            return true;
        }
    }

    bool isAuthenticated() const { return authenticated_; }
    std::uint64_t count() const { return kernel_.count(); }

private:
    static ssize_t readAll(int fd, char *buf, size_t len) {
        size_t total = 0;
        while (total < len) {
            ssize_t r = read(fd, buf + total, len - total);
            if (r <= 0) return (total == 0) ? r : static_cast<ssize_t>(total);
            total += static_cast<size_t>(r);
        }
        return static_cast<ssize_t>(total);
    }

    static ssize_t writeAll(int fd, const char *buf, size_t len) {
        size_t total = 0;
        while (total < len) {
            ssize_t w = write(fd, buf + total, len - total);
            if (w <= 0) return (total == 0) ? w : static_cast<ssize_t>(total);
            total += static_cast<size_t>(w);
        }
        return static_cast<ssize_t>(total);
    }

    Kernel &kernel_;
    World &world_;
    const std::string bootstrapToken_;
    const std::string fixtureAuthorityToken_;
    const std::string compositorIncarnation_;
    std::string coordinatorIncarnation_;
    std::uint64_t coordinatorEpoch_ = 0;
    std::string activeSessionId_;
    std::uint64_t expectedRecvSeq_ = 1;
    std::uint64_t sendSeq_ = 1;
    bool authenticated_ = false;
};
} // namespace synthetic
#endif
} // namespace depthos::experiment
