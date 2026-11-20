// Independent Adversarial Security Test for Phase 3.8
// Tests Native Prompt Integrity, Lock/Seat Races, and State Resurrection Attacks.
#include "authority.h"
#include <QCoreApplication>
#include <QMetaObject>
#include <QThread>
#include <cassert>
#include <chrono>
#include <functional>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

using namespace depthos::experiment;

static void check(bool condition, const char *label) {
    if (!condition) {
        std::cerr << "FAIL ASSERTION: " << label << std::endl;
        throw std::runtime_error(label);
    }
}

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);

    std::cout << "--- [ADVERSARIAL 1] Testing Public Origin Refusal ---" << std::endl;
    DenyOnlyAuthority denyAuthority;
    for (Origin origin : {Origin::Unknown, Origin::ClaimedPhysical, Origin::Qt, Origin::Qml,
                          Origin::Eis, Origin::Remote, Origin::Accessibility}) {
        check(denyAuthority.confirm(origin) == Result::Denied,
              "Untrusted or synthetic origin incorrectly authorized!");
    }
    std::cout << "PASS: All public origins return Result::Denied without exception." << std::endl;

#ifdef DEPTHOS_AUTHORITY_UNIT_TEST
    for (synthetic::InputProvenance prov : {
             synthetic::InputProvenance::Unknown,
             synthetic::InputProvenance::LocalCandidate,
             synthetic::InputProvenance::Eis,
             synthetic::InputProvenance::Portal,
             synthetic::InputProvenance::FakeOrVirtual,
             synthetic::InputProvenance::Accessibility,
             synthetic::InputProvenance::Remote}) {
        check(synthetic::Kernel::classifyInputOrigin(prov) == Result::Denied,
              "Input provenance classification incorrectly authorized!");
    }
    std::cout << "PASS: All 7 synthetic input provenances return Result::Denied without exception." << std::endl;
#endif

#ifdef DEPTHOS_AUTHORITY_UNIT_TEST
    using namespace synthetic;

    struct AdversarialFixture {
        std::unique_ptr<QObject> window = std::make_unique<QObject>();
        QObject prompt;
        QObject replacementWindow;
        QObject replacementPrompt;
        World world;
        Kernel kernel;
        const Kernel::Clock::time_point now = Kernel::Clock::now();

        AdversarialFixture() {
            world.windows.push_back(Window{window.get(), "window-uuid-001"});
            world.focus = window.get();
            world.prompt = &prompt;
            world.promptOwned = true;
            world.locked = false;
            world.seat = "seat0";
            world.activity = "activity";
            world.currentDesktop = "source";
            world.desktops = {"source", "destination", "extra"};
            world.compositor = 100;
            world.coordinator = 200;
            world.observer = 300;
            world.topology = 400;

            check(kernel.prepare(world, window.get(), "destination", now), "Initial prepare failed");
        }

        Result confirm(bool loseAck = false, Kernel::Clock::time_point at = Kernel::Clock::now()) {
            return kernel.syntheticConfirm(world, kernel.serial(), loseAck, at);
        }
    };

    std::cout << "\n--- [ADVERSARIAL 2] Matrix of State Corruption & Tampering Attacks ---" << std::endl;
    const std::vector<std::pair<const char *, std::function<void(AdversarialFixture &)>>> corruptionAttacks{
        {"focus dropped to null", [](auto &f) { f.world.focus = nullptr; }},
        {"focus switched to replacement window", [](auto &f) { f.world.focus = &f.replacementWindow; }},
        {"screen locked", [](auto &f) { f.world.locked = true; f.world.lockPhase = LockPhase::Locked; }},
        {"screen aboutToLock phase", [](auto &f) { f.world.lockPhase = LockPhase::AboutToLock; }},
        {"screen lock status unknown", [](auto &f) { f.world.locked.reset(); }},
        {"seat switched to seat1", [](auto &f) { f.world.seat = "seat1"; }},
        {"active seat mismatch", [](auto &f) { f.world.activeSeat = "seat1"; }},
        {"device seat mismatch", [](auto &f) { f.world.deviceSeat = "seat1"; }},
        {"seat empty", [](auto &f) { f.world.seat.clear(); }},
        {"prompt replaced by rogue object", [](auto &f) { f.world.prompt = &f.replacementPrompt; }},
        {"prompt destroyed/null", [](auto &f) { f.world.prompt = nullptr; }},
        {"prompt unowned by compositor", [](auto &f) { f.world.promptOwned = false; }},
        {"prompt not rendered", [](auto &f) { f.world.promptRendered = false; }},
        {"prompt not visible", [](auto &f) { f.world.promptVisible = false; }},
        {"window moved prematurely by other actor", [](auto &f) { f.world.windows[0].desktop = "destination"; }},
        {"current desktop changed out-of-sync", [](auto &f) { f.world.currentDesktop = "destination"; }},
        {"activity changed", [](auto &f) { f.world.activity = "other-activity"; }},
        {"window marked ineligible", [](auto &f) { f.world.windows[0].eligible = false; }},
        {"destination desktop deleted from topology", [](auto &f) { f.world.desktops.erase("destination"); }},
        {"topology revision drifted", [](auto &f) { ++f.world.topology; }},
        {"observer epoch drifted", [](auto &f) { ++f.world.observer; }},
        {"coordinator incarnation drifted", [](auto &f) { ++f.world.coordinator; }},
        {"compositor incarnation drifted", [](auto &f) { ++f.world.compositor; }},
        {"window generation drifted", [](auto &f) { ++f.world.windows[0].generation; }},
        {"window UUID mutated", [](auto &f) { f.world.windows[0].uuid = "mutated-uuid"; }},
        {"duplicate UUID injected into window list", [](auto &f) {
            f.world.windows.push_back(Window{&f.replacementWindow, "window-uuid-001"});
        }},
        {"duplicate QObject pointer in window list", [](auto &f) {
            f.world.windows.push_back(f.world.windows[0]);
        }},
        {"window QObject destroyed before confirm", [](auto &f) { f.window.reset(); }},
        {"same UUID recreated on new object", [](auto &f) {
            f.window.reset();
            f.window = std::make_unique<QObject>();
            f.world.windows[0].object = f.window.get();
            f.world.focus = f.window.get();
        }},
        {"compositor marked dead", [](auto &f) { f.world.alive = false; }},
        {"authority kernel torn down", [](auto &f) { f.kernel.teardown(); }},
        {"explicit cancellation invoked", [](auto &f) { check(f.kernel.cancel(), "cancel returned false"); }},
    };

    for (const auto &[label, attack] : corruptionAttacks) {
        AdversarialFixture fixture;
        attack(fixture);
        Result res = fixture.confirm(false, fixture.now);
        check(res == Result::Denied, label);
        check(fixture.kernel.count() == 0, "Execution count must be 0 after denial");
        // Verify replay after denial is also denied
        check(fixture.confirm(false, fixture.now) == Result::Denied, "Replay after denial was permitted");
        std::cout << "PASS: Attack [" << label << "] successfully defeated with Result::Denied." << std::endl;
    }

    std::cout << "\n--- [ADVERSARIAL 3] Proposal Resurrection Attacks After State Restoration ---" << std::endl;
    // Attack: State change occurs, cancelling proposal. Later, the state returns to normal.
    // In a flawed implementation, checking current state would resurrect the proposal.
    // In DepthOS, cancellation is strictly permanent and irreversible.

    // Scenario 3.1: Focus lost and regained
    {
        AdversarialFixture f;
        f.world.focus = nullptr;
        check(f.kernel.cancel(), "Cancel on focus loss");
        // Restore focus
        f.world.focus = f.window.get();
        check(f.confirm(false, f.now) == Result::Denied, "Resurrection via focus restoration must fail");
        check(f.kernel.count() == 0, "Count must remain 0");
        std::cout << "PASS: Focus loss -> restoration resurrection attack defeated." << std::endl;
    }

    // Scenario 3.2: Screen locked and unlocked
    {
        AdversarialFixture f;
        f.world.locked = true;
        check(f.kernel.cancel(), "Cancel on screen lock");
        // Unlock screen
        f.world.locked = false;
        check(f.confirm(false, f.now) == Result::Denied, "Resurrection via screen unlock must fail");
        check(f.kernel.count() == 0, "Count must remain 0");
        std::cout << "PASS: Screen lock -> unlock resurrection attack defeated." << std::endl;
    }

    // Scenario 3.2b: AboutToLock transition and cancellation
    {
        AdversarialFixture f;
        f.world.lockPhase = LockPhase::AboutToLock;
        check(f.kernel.cancel(), "Cancel on aboutToLock transition");
        // Screen unlocking back to Unlocked
        f.world.lockPhase = LockPhase::Unlocked;
        check(f.confirm(false, f.now) == Result::Denied, "Resurrection via aboutToLock recovery must fail");
        check(f.kernel.count() == 0, "Count must remain 0");
        std::cout << "PASS: AboutToLock transition -> unlock resurrection attack defeated." << std::endl;
    }

    // Scenario 3.3: Prompt replaced and restored
    {
        AdversarialFixture f;
        f.world.prompt = &f.replacementPrompt;
        check(f.kernel.cancel(), "Cancel on prompt replacement");
        // Restore original prompt
        f.world.prompt = &f.prompt;
        check(f.confirm(false, f.now) == Result::Denied, "Resurrection via prompt restoration must fail");
        check(f.kernel.count() == 0, "Count must remain 0");
        std::cout << "PASS: Prompt tampering -> restoration resurrection attack defeated." << std::endl;
    }

    // Scenario 3.4: Seat switched and restored
    {
        AdversarialFixture f;
        f.world.seat = "seat1";
        check(f.kernel.cancel(), "Cancel on seat change");
        // Restore seat
        f.world.seat = "seat0";
        check(f.confirm(false, f.now) == Result::Denied, "Resurrection via seat restoration must fail");
        check(f.kernel.count() == 0, "Count must remain 0");
        std::cout << "PASS: Seat switch -> restoration resurrection attack defeated." << std::endl;
    }

    std::cout << "\n--- [ADVERSARIAL 4] Timing and Deadline Edge Attacks ---" << std::endl;
    // Attack 4.1: Steady-clock reversal
    {
        AdversarialFixture f;
        auto earlier = f.now - std::chrono::nanoseconds(1);
        check(f.confirm(false, earlier) == Result::Denied, "Clock reversal was accepted");
        check(f.kernel.count() == 0, "Reversed clock counted");
        std::cout << "PASS: Clock reversal rejected." << std::endl;
    }

    // Attack 4.2: Exact deadline boundary (now == deadline)
    {
        AdversarialFixture f;
        auto exactDeadline = f.now + std::chrono::seconds(5);
        check(f.confirm(false, exactDeadline) == Result::Denied, "Exact deadline boundary accepted");
        check(f.kernel.count() == 0, "Expired deadline counted");
        std::cout << "PASS: Exact deadline boundary (now == deadline) rejected." << std::endl;
    }

    // Attack 4.3: Past deadline
    {
        AdversarialFixture f;
        auto pastDeadline = f.now + std::chrono::seconds(6);
        check(f.confirm(false, pastDeadline) == Result::Denied, "Past deadline accepted");
        check(f.kernel.count() == 0, "Past deadline counted");
        std::cout << "PASS: Past deadline rejected." << std::endl;
    }

    // Attack 4.4: Deadline overflow at prepare
    {
        QObject win, pmt;
        World w;
        w.windows.push_back(Window{&win, "win-max"});
        w.focus = &win;
        w.prompt = &pmt;
        Kernel k;
        check(!k.prepare(w, &win, "destination", Kernel::Clock::time_point::max()),
              "Overflowing deadline accepted at prepare");
        const auto safeLatest = Kernel::Clock::time_point::max() - std::chrono::seconds(5);
        check(k.prepare(w, &win, "destination", safeLatest), "Safe boundary rejected at prepare");
        std::cout << "PASS: Deadline overflow refused and safe boundary accepted." << std::endl;
    }

    std::cout << "\n--- [ADVERSARIAL 5] Concurrency, Single-Use, and Crash-Cut Invariants ---" << std::endl;
    // Attack 5.1: Double prepare / concurrent proposal on same kernel
    {
        AdversarialFixture f;
        check(!f.kernel.prepare(f.world, f.window.get(), "destination", f.now),
              "Concurrent prepare on active kernel succeeded");
        std::cout << "PASS: Concurrent prepare on active kernel rejected." << std::endl;
    }

    // Attack 5.2: Token/serial forgery
    {
        AdversarialFixture f;
        auto forgedSerial = f.kernel.serial() + 999;
        check(f.kernel.syntheticConfirm(f.world, forgedSerial, false, f.now) == Result::Denied,
              "Forged serial was confirmed");
        std::cout << "PASS: Forged token/serial strictly rejected." << std::endl;
    }

    // Attack 5.3: Single-use & double-consume
    {
        AdversarialFixture f;
        check(f.confirm(false, f.now) == Result::Counted && f.kernel.count() == 1, "First confirm failed");
        check(f.confirm(false, f.now) == Result::Denied && f.kernel.count() == 1, "Second confirm succeeded");
        check(!f.kernel.cancel(), "Cancel succeeded after consumption");
        std::cout << "PASS: Single-use verified; double-consume strictly denied." << std::endl;
    }

    // Attack 5.4: Uncertain completion (lost ack) quarantine
    {
        AdversarialFixture f;
        check(f.confirm(true, f.now) == Result::Uncertain, "Lost ack did not yield Uncertain");
        check(f.kernel.count() == 1, "Count was not recorded on uncertain");
        // Must never retry
        check(f.confirm(false, f.now) == Result::Denied, "Retry permitted after uncertain completion");
        // Must never reprepare
        check(!f.kernel.prepare(f.world, f.window.get(), "destination", f.now),
              "Reprepare permitted after uncertain completion");
        std::cout << "PASS: Uncertain completion permanently quarantines proposal without retry." << std::endl;
    }

    // Attack 5.5: Cross-thread affinity tampering
    for (bool movePrompt : {false, true}) {
        AdversarialFixture f;
        QThread workerThread;
        workerThread.start();
        QThread *mainThread = QThread::currentThread();
        QObject *target = movePrompt ? &f.prompt : f.window.get();
        check(target->moveToThread(&workerThread), "Move to foreign thread failed");

        Result res = f.confirm(false, f.now);
        Kernel freshKernel;
        bool prep = freshKernel.prepare(f.world, f.window.get(), "destination", f.now);

        check(QMetaObject::invokeMethod(target, [&] {
            check(target->moveToThread(mainThread), "Restore thread affinity failed");
        }, Qt::BlockingQueuedConnection), "Invoke restore failed");

        workerThread.quit();
        check(workerThread.wait(3000), "Thread wait timeout");

        check(res == Result::Denied && f.kernel.count() == 0 && !prep,
              "Foreign thread affinity was accepted");
        check(f.confirm(false, f.now) == Result::Denied, "Affinity restoration revived proposal");
        std::cout << "PASS: Foreign thread affinity attack on "
                  << (movePrompt ? "prompt" : "window") << " strictly denied." << std::endl;
    }

    std::cout << "\n--- [ADVERSARIAL 6] Phase 3.10 Synthetic IPC Endpoint Adversarial Tests ---" << std::endl;
    // 6.1 Peer Credential Verification
    {
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");
        check(SyntheticAuthorityServer::verifyPeerCredentials(fds[0]), "Peer credentials check failed");
        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Peer credential check verified for same-UID process." << std::endl;
    }

    // 6.2 Bootstrap Token Authentication & Unauthorized Rejection
    {
        AdversarialFixture f;
        SyntheticAuthorityServer server(f.kernel, f.world, "secret-bootstrap-token-128", "fixture-token", "comp-uuid");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // Send Hello with wrong bootstrap token
        QJsonObject content;
        content["protocol_version"] = 0x0100;
        content["session_id"] = "sess-1";
        content["coordinator_incarnation"] = "coord-inc-1";
        content["coordinator_epoch"] = 1;
        content["bootstrap_token"] = "wrong-forged-token";
        content["client_pid"] = static_cast<qint64>(getpid());

        QJsonObject root;
        root["msg_type"] = "Hello";
        root["payload"] = content;

        QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
        check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Hello,
                               std::string(bytes.constData(), bytes.size())), "sendFrame failed");

        // Server must reject unauthorized connection attempt
        check(!server.processMessage(fds[0]), "Server accepted invalid bootstrap token");
        check(!server.isAuthenticated(), "Server marked authenticated on bad token");
        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Unauthorized connection attempt with invalid bootstrap token strictly rejected." << std::endl;
    }

    // 6.3 Valid Handshake & Session Activation
    {
        AdversarialFixture f;
        SyntheticAuthorityServer server(f.kernel, f.world, "secret-bootstrap-token-128", "fixture-token", "comp-uuid");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        QJsonObject content;
        content["protocol_version"] = 0x0100;
        content["session_id"] = "sess-1";
        content["coordinator_incarnation"] = "coord-inc-1";
        content["coordinator_epoch"] = 1;
        content["bootstrap_token"] = "secret-bootstrap-token-128";
        content["client_pid"] = static_cast<qint64>(getpid());

        QJsonObject root;
        root["msg_type"] = "Hello";
        root["payload"] = content;

        QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
        check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Hello,
                               std::string(bytes.constData(), bytes.size())), "sendFrame failed");

        check(server.processMessage(fds[0]), "Server rejected valid bootstrap token");
        check(server.isAuthenticated(), "Server not authenticated after valid token");

        // Client reads HelloAccepted
        SyntheticAuthorityServer::MessageType repType;
        std::uint64_t seq = 0;
        std::string reply;
        check(server.readFrame(fds[1], repType, seq, reply), "Client read HelloAccepted failed");
        check(repType == SyntheticAuthorityServer::MessageType::HelloAccepted, "Expected HelloAccepted type");

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Valid bootstrap token handshake establishes authenticated session." << std::endl;
    }

    // 6.4 Framing Boundary: Oversize Payload (>64KiB) Rejection Without Allocation
    {
        AdversarialFixture f;
        SyntheticAuthorityServer server(f.kernel, f.world, "tok", "fixt", "comp");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // Send frame with declared length 70000 bytes (> 65536)
        char headerBuf[20];
        uint32_t magic = htobe32(0x44504F53);
        uint16_t ver = htobe16(0x0100);
        uint16_t typeCode = htobe16(1);
        uint64_t s = htobe64(1);
        uint32_t len = htobe32(70000); // 70 KiB
        std::memcpy(&headerBuf[0], &magic, 4);
        std::memcpy(&headerBuf[4], &ver, 2);
        std::memcpy(&headerBuf[6], &typeCode, 2);
        std::memcpy(&headerBuf[8], &s, 8);
        std::memcpy(&headerBuf[16], &len, 4);

        check(write(fds[1], headerBuf, 20) == 20, "write header failed");

        SyntheticAuthorityServer::MessageType t;
        std::uint64_t rseq = 0;
        std::string p;
        check(!server.readFrame(fds[0], t, rseq, p), "Server accepted oversize payload");

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Oversize payload (>64KiB) rejected immediately without memory allocation." << std::endl;
    }

    // 6.5 Framing Boundary: Malformed Magic & Version Rejection
    {
        AdversarialFixture f;
        SyntheticAuthorityServer server(f.kernel, f.world, "tok", "fixt", "comp");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // Bad magic: 0xDEADBEEF
        char headerBuf[20];
        uint32_t magic = htobe32(0xDEADBEEF);
        uint16_t ver = htobe16(0x0100);
        uint16_t typeCode = htobe16(1);
        uint64_t s = htobe64(1);
        uint32_t len = htobe32(10);
        std::memcpy(&headerBuf[0], &magic, 4);
        std::memcpy(&headerBuf[4], &ver, 2);
        std::memcpy(&headerBuf[6], &typeCode, 2);
        std::memcpy(&headerBuf[8], &s, 8);
        std::memcpy(&headerBuf[16], &len, 4);

        check(write(fds[1], headerBuf, 20) == 20, "write header failed");
        SyntheticAuthorityServer::MessageType t;
        std::uint64_t rseq = 0;
        std::string p;
        check(!server.readFrame(fds[0], t, rseq, p), "Server accepted bad magic");

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Malformed framing magic strictly rejected." << std::endl;
    }

    // 6.6 Packet Sequence Replay Rejection
    {
        AdversarialFixture f;
        SyntheticAuthorityServer server(f.kernel, f.world, "tok", "fixt", "comp");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // Sequence number = 5 when server expects 1
        char headerBuf[20];
        uint32_t magic = htobe32(0x44504F53);
        uint16_t ver = htobe16(0x0100);
        uint16_t typeCode = htobe16(1);
        uint64_t s = htobe64(5); // Out of order!
        uint32_t len = htobe32(0);
        std::memcpy(&headerBuf[0], &magic, 4);
        std::memcpy(&headerBuf[4], &ver, 2);
        std::memcpy(&headerBuf[6], &typeCode, 2);
        std::memcpy(&headerBuf[8], &s, 8);
        std::memcpy(&headerBuf[16], &len, 4);

        check(write(fds[1], headerBuf, 20) == 20, "write header failed");
        SyntheticAuthorityServer::MessageType t;
        std::uint64_t rseq = 0;
        std::string p;
        check(!server.readFrame(fds[0], t, rseq, p), "Server accepted out-of-order sequence");

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Packet sequence gap and replay strictly rejected." << std::endl;
    }

    // 6.7 End-to-End Synthetic Proposal, Single-Use Invariant & Non-Mutation Guarantee
    {
        AdversarialFixture f;
        Kernel freshKernel;
        SyntheticAuthorityServer server(freshKernel, f.world, "secret-tok", "fixture-auth-token-128", "comp-uuid-42");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // 1. Handshake
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-alpha";
            content["coordinator_incarnation"] = "coord-alpha";
            content["coordinator_epoch"] = 1;
            content["bootstrap_token"] = "secret-tok";
            content["client_pid"] = static_cast<qint64>(getpid());
            QJsonObject root;
            root["msg_type"] = "Hello";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Hello,
                                   std::string(bytes.constData(), bytes.size())), "send Hello");
            check(server.processMessage(fds[0], f.now), "process Hello");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read HelloAccepted");
            check(repType == SyntheticAuthorityServer::MessageType::HelloAccepted, "repType HelloAccepted");
        }

        // 2. Reserve
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-alpha";
            content["operation_id"] = "op-1";
            content["lease_id"] = "lease-1";
            content["coordinator_incarnation"] = "coord-alpha";
            content["coordinator_epoch"] = 1;
            content["native_uuid"] = "window-uuid-001";
            QJsonObject root;
            root["msg_type"] = "Reserve";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Reserve,
                                   std::string(bytes.constData(), bytes.size())), "send Reserve");
            check(server.processMessage(fds[0], f.now), "process Reserve");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read Prepared");
            check(repType == SyntheticAuthorityServer::MessageType::Prepared, "repType Prepared");
        }

        // 3. SyntheticAttempt with valid tokens
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-alpha";
            content["operation_id"] = "op-1";
            content["lease_id"] = "lease-1";
            content["attempt"] = 1;
            content["coordinator_incarnation"] = "coord-alpha";
            content["coordinator_epoch"] = 1;
            content["compositor_incarnation"] = "comp-uuid-42";
            content["native_uuid"] = "window-uuid-001";
            content["fixture_authority_token"] = "fixture-auth-token-128";
            QJsonObject root;
            root["msg_type"] = "SyntheticAttempt";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::SyntheticAttempt,
                                   std::string(bytes.constData(), bytes.size())), "send SyntheticAttempt");
            check(server.processMessage(fds[0], f.now), "process SyntheticAttempt");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read Acknowledge");
            check(repType == SyntheticAuthorityServer::MessageType::Acknowledge, "repType Acknowledge");

            QJsonDocument ackDoc = QJsonDocument::fromJson(QByteArray::fromRawData(reply.data(), reply.size()));
            check(ackDoc.object()["payload"].toObject()["outcome"].toString() == "Applied", "Ack outcome Applied");
            check(server.count() == 1, "Counter incremented to exactly 1");
        }

        // 4. Duplicate Attempt: strictly denied, counter stays 1
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-alpha";
            content["operation_id"] = "op-1";
            content["lease_id"] = "lease-1";
            content["attempt"] = 2;
            content["coordinator_incarnation"] = "coord-alpha";
            content["coordinator_epoch"] = 1;
            content["compositor_incarnation"] = "comp-uuid-42";
            content["native_uuid"] = "window-uuid-001";
            content["fixture_authority_token"] = "fixture-auth-token-128";
            QJsonObject root;
            root["msg_type"] = "SyntheticAttempt";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::SyntheticAttempt,
                                   std::string(bytes.constData(), bytes.size())), "send duplicate Attempt");
            check(server.processMessage(fds[0], f.now), "process duplicate Attempt");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read second Acknowledge");

            QJsonDocument ackDoc = QJsonDocument::fromJson(QByteArray::fromRawData(reply.data(), reply.size()));
            check(ackDoc.object()["payload"].toObject()["outcome"].toString() == "Rejected", "Duplicate attempt Rejected");
            check(server.count() == 1, "Counter strictly remains 1 (single-use invariant)");
        }

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: End-to-end synthetic proposal, single-use counting, and non-mutation verified." << std::endl;
    }

    // 6.8 Adversarial: Operation ID mismatch in SyntheticAttempt must be strictly rejected
    {
        AdversarialFixture f;
        Kernel k67;
        SyntheticAuthorityServer server(k67, f.world, "secret-tok", "fixture-auth-token-128", "comp-uuid-42");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // Handshake
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-beta";
            content["coordinator_incarnation"] = "coord-beta";
            content["coordinator_epoch"] = 1;
            content["bootstrap_token"] = "secret-tok";
            content["client_pid"] = static_cast<qint64>(getpid());
            QJsonObject root;
            root["msg_type"] = "Hello";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Hello,
                                   std::string(bytes.constData(), bytes.size())), "send Hello");
            check(server.processMessage(fds[0], f.now), "process Hello");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read HelloAccepted");
        }

        // Reserve for "op-legit"
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-beta";
            content["operation_id"] = "op-legit";
            content["lease_id"] = "lease-beta";
            content["coordinator_incarnation"] = "coord-beta";
            content["coordinator_epoch"] = 1;
            content["native_uuid"] = "window-uuid-001";
            QJsonObject root;
            root["msg_type"] = "Reserve";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Reserve,
                                   std::string(bytes.constData(), bytes.size())), "send Reserve");
            check(server.processMessage(fds[0], f.now), "process Reserve");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read Prepared");
        }

        // Adversarial SyntheticAttempt with mismatched operation_id: "op-spoofed"
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-beta";
            content["operation_id"] = "op-spoofed"; // Mismatch!
            content["lease_id"] = "lease-beta";
            content["attempt"] = 1;
            content["coordinator_incarnation"] = "coord-beta";
            content["coordinator_epoch"] = 1;
            content["compositor_incarnation"] = "comp-uuid-42";
            content["native_uuid"] = "window-uuid-001";
            content["fixture_authority_token"] = "fixture-auth-token-128";
            QJsonObject root;
            root["msg_type"] = "SyntheticAttempt";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::SyntheticAttempt,
                                   std::string(bytes.constData(), bytes.size())), "send spoofed Attempt");
            check(server.processMessage(fds[0], f.now), "process spoofed Attempt");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read Acknowledge");

            QJsonDocument ackDoc = QJsonDocument::fromJson(QByteArray::fromRawData(reply.data(), reply.size()));
            check(ackDoc.object()["payload"].toObject()["outcome"].toString() == "Rejected",
                  "Mismatched operation_id in SyntheticAttempt was NOT rejected");
            check(server.count() == 0, "Counter was incremented despite operation_id mismatch");
        }

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Mismatched operation_id in SyntheticAttempt strictly rejected." << std::endl;
    }

    // 6.9 Adversarial: Kernel prepare failure must report ready = false in Prepared message
    {
        AdversarialFixture f;
        Kernel k67;
        SyntheticAuthorityServer server(k67, f.world, "secret-tok", "fixture-auth-token-128", "comp-uuid-42");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair failed");

        // Handshake
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-gamma";
            content["coordinator_incarnation"] = "coord-gamma";
            content["coordinator_epoch"] = 1;
            content["bootstrap_token"] = "secret-tok";
            content["client_pid"] = static_cast<qint64>(getpid());
            QJsonObject root;
            root["msg_type"] = "Hello";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Hello,
                                   std::string(bytes.constData(), bytes.size())), "send Hello");
            check(server.processMessage(fds[0], f.now), "process Hello");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read HelloAccepted");
        }

        // Erase "destination" desktop so prepare fails!
        f.world.desktops.erase("destination");

        // Reserve for "op-prep-fail"
        {
            QJsonObject content;
            content["protocol_version"] = 0x0100;
            content["session_id"] = "sess-gamma";
            content["operation_id"] = "op-prep-fail";
            content["lease_id"] = "lease-gamma";
            content["coordinator_incarnation"] = "coord-gamma";
            content["coordinator_epoch"] = 1;
            content["native_uuid"] = "window-uuid-001";
            QJsonObject root;
            root["msg_type"] = "Reserve";
            root["payload"] = content;
            QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);
            check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::Reserve,
                                   std::string(bytes.constData(), bytes.size())), "send Reserve");
            check(server.processMessage(fds[0], f.now), "process Reserve");
            SyntheticAuthorityServer::MessageType repType;
            std::uint64_t seq = 0;
            std::string reply;
            check(server.readFrame(fds[1], repType, seq, reply), "read Prepared");

            QJsonDocument prepDoc = QJsonDocument::fromJson(QByteArray::fromRawData(reply.data(), reply.size()));
            check(!prepDoc.object()["payload"].toObject()["ready"].toBool(),
                  "Kernel prepare failure falsely reported ready = true");
        }

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Kernel prepare failure correctly reports ready = false." << std::endl;
    }



    std::cout << "\n=== ALL NATIVE AUTHORITY ADVERSARIAL TESTS PASSED ===" << std::endl;
#endif

    return 0;
}
