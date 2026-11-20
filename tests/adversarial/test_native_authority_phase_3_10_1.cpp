// Phase 3.10.1 Independent Native Authority Adversarial Test Suite
// Rigorously verifies all Target 2 Attack Vectors under Qt6Core + ASan + UBSan.

#include "authority.h"
#include <QCoreApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <cassert>
#include <chrono>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <sys/socket.h>
#include <unistd.h>

using namespace depthos::experiment;
using namespace depthos::experiment::synthetic;

static void check(bool condition, const char *msg) {
    if (!condition) {
        std::cerr << "FAIL ASSERTION: " << msg << std::endl;
        throw std::runtime_error(msg);
    }
}

struct TestFixture {
    std::unique_ptr<QObject> windowObj = std::make_unique<QObject>();
    QObject promptObj;
    World world;
    Kernel::Clock::time_point now = Kernel::Clock::now();

    TestFixture() {
        world.windows.push_back({windowObj.get(), "uuid-target-win", 1, "desktop-1", "act-1", true});
        world.focus = windowObj.get();
        world.prompt = &promptObj;
        world.promptOwned = true;
        world.promptRendered = true;
        world.promptVisible = true;
        world.desktops = {"desktop-1", "desktop-2"};
        world.currentDesktop = "desktop-1";
        world.activity = "act-1";
        world.seat = "seat0";
        world.activeSeat = "seat0";
        world.deviceSeat = "seat0";
        world.compositor = 1001;
        world.coordinator = 2001;
        world.observer = 3001;
        world.topology = 1;
        world.lockPhase = LockPhase::Unlocked;
    }
};

static void helperHandshake(SyntheticAuthorityServer &server, int clientFd, int serverFd,
                            const std::string &token = "secret-token") {
    QJsonObject content;
    content["protocol_version"] = 0x0100;
    content["session_id"] = "sess-test";
    content["coordinator_incarnation"] = "coord-inc-1";
    content["coordinator_epoch"] = 1;
    content["bootstrap_token"] = QString::fromStdString(token);
    content["client_pid"] = static_cast<qint64>(getpid());

    QJsonObject root;
    root["msg_type"] = "Hello";
    root["payload"] = content;
    QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);

    check(server.sendFrame(clientFd, SyntheticAuthorityServer::MessageType::Hello,
                           std::string(bytes.constData(), bytes.size())), "send Hello");
    check(server.processMessage(serverFd, Kernel::Clock::now()), "process Hello");

    SyntheticAuthorityServer::MessageType repType;
    std::uint64_t seq = 0;
    std::string reply;
    check(server.readFrame(clientFd, repType, seq, reply), "read HelloAccepted");
    check(repType == SyntheticAuthorityServer::MessageType::HelloAccepted, "must be HelloAccepted");
}

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);
    std::cout << "=== Phase 3.10.1 Native Authority Adversarial Test Suite ===" << std::endl;

    // Vector 2.1: SyntheticAttempt without native proposal
    {
        std::cout << "[Test 2.1] SyntheticAttempt without native proposal..." << std::endl;
        TestFixture f;
        Kernel k;
        SyntheticAuthorityServer server(k, f.world, "secret-token", "auth-token-xyz", "comp-inc-1001");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair");
        helperHandshake(server, fds[1], fds[0]);

        QJsonObject content;
        content["protocol_version"] = 0x0100;
        content["session_id"] = "sess-test";
        content["operation_id"] = "op-unprepared";
        content["lease_id"] = "lease-1";
        content["attempt"] = 1;
        content["fixture_authority_token"] = "auth-token-xyz";
        content["compositor_incarnation"] = "comp-inc-1001";
        content["coordinator_incarnation"] = "coord-inc-1";

        QJsonObject root;
        root["msg_type"] = "SyntheticAttempt";
        root["payload"] = content;
        QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);

        check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::SyntheticAttempt,
                               std::string(bytes.constData(), bytes.size())), "send attempt");
        check(server.processMessage(fds[0], f.now), "process attempt");

        SyntheticAuthorityServer::MessageType repType;
        std::uint64_t seq = 0;
        std::string reply;
        check(server.readFrame(fds[1], repType, seq, reply), "read Ack");
        QJsonDocument doc = QJsonDocument::fromJson(QByteArray::fromRawData(reply.data(), reply.size()));
        check(doc.object()["payload"].toObject()["outcome"].toString() == "Rejected", "outcome must be Rejected");
        check(server.count() == 0, "count must remain 0");

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: SyntheticAttempt without native proposal rejected." << std::endl;
    }

    // Vector 2.2: Focus loss race between prepare and attempt
    {
        std::cout << "[Test 2.2] Focus loss between prepare and attempt..." << std::endl;
        TestFixture f;
        Kernel k;
        SyntheticAuthorityServer server(k, f.world, "secret-token", "auth-token-xyz", "comp-inc-1001");
        int fds[2];
        check(socketpair(AF_UNIX, SOCK_STREAM, 0, fds) == 0, "socketpair");
        helperHandshake(server, fds[1], fds[0]);

        // Prepare proposal
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-focus-loss"), "prepare");

        // Simulate focus loss: prompt window hidden / dismissed
        f.world.promptVisible = false;

        QJsonObject content;
        content["protocol_version"] = 0x0100;
        content["session_id"] = "sess-test";
        content["operation_id"] = "op-focus-loss";
        content["lease_id"] = "lease-1";
        content["attempt"] = 1;
        content["fixture_authority_token"] = "auth-token-xyz";
        content["compositor_incarnation"] = "comp-inc-1001";
        content["coordinator_incarnation"] = "coord-inc-1";

        QJsonObject root;
        root["msg_type"] = "SyntheticAttempt";
        root["payload"] = content;
        QByteArray bytes = QJsonDocument(root).toJson(QJsonDocument::Compact);

        check(server.sendFrame(fds[1], SyntheticAuthorityServer::MessageType::SyntheticAttempt,
                               std::string(bytes.constData(), bytes.size())), "send attempt");
        check(server.processMessage(fds[0], f.now), "process attempt");

        SyntheticAuthorityServer::MessageType repType;
        std::uint64_t seq = 0;
        std::string reply;
        check(server.readFrame(fds[1], repType, seq, reply), "read Ack");
        QJsonDocument doc = QJsonDocument::fromJson(QByteArray::fromRawData(reply.data(), reply.size()));
        check(doc.object()["payload"].toObject()["outcome"].toString() == "Rejected", "outcome must be Rejected");
        check(server.count() == 0, "count must remain 0");

        close(fds[0]);
        close(fds[1]);
        std::cout << "PASS: Attempt after focus loss strictly rejected." << std::endl;
    }

    // Vector 2.3: Replayed proposal execution
    {
        std::cout << "[Test 2.3] Replay of already executed/aborted proposal..." << std::endl;
        TestFixture f;
        Kernel k;
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-replay"), "prepare");
        check(k.syntheticConfirm(f.world, k.serial(), false, f.now) == Result::Counted, "confirm 1");
        check(k.count() == 1, "count == 1");

        // Replay attempt must be Denied
        check(k.syntheticConfirm(f.world, k.serial(), false, f.now) == Result::Denied, "replay confirm");
        check(k.count() == 1, "count must still be 1 (no double increment)");
        std::cout << "PASS: Replayed proposal execution strictly denied." << std::endl;
    }

    // Vector 2.4: Destroyed native window
    {
        std::cout << "[Test 2.4] Target native window destroyed in-flight..." << std::endl;
        TestFixture f;
        Kernel k;
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-destroy"), "prepare");

        // Destroy target window object
        f.windowObj.reset();

        // Attempt confirm
        check(k.syntheticConfirm(f.world, k.serial(), false, f.now) == Result::Denied, "confirm destroyed win");
        check(k.count() == 0, "count must remain 0");
        std::cout << "PASS: Destroyed window handled gracefully without use-after-free." << std::endl;
    }

    // Vector 2.5: Recreated window with new generation
    {
        std::cout << "[Test 2.5] Recreated window with new generation / UUID..." << std::endl;
        TestFixture f;
        Kernel k;
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-recreated"), "prepare");

        // Advance generation on window
        f.world.windows[0].generation = 2;

        check(k.syntheticConfirm(f.world, k.serial(), false, f.now) == Result::Denied, "confirm new gen");
        check(k.count() == 0, "count must remain 0");
        std::cout << "PASS: Recreated window generation mismatch rejected." << std::endl;
    }

    // Vector 2.6: Invalid destination geometry / desktop
    {
        std::cout << "[Test 2.6] Non-existent destination desktop..." << std::endl;
        TestFixture f;
        Kernel k;
        check(!k.prepare(f.world, f.windowObj.get(), "desktop-nonexistent", f.now, "op-bad-dest"),
              "prepare on non-existent desktop must fail");
        check(!k.isPending(), "must not be pending");
        std::cout << "PASS: Invalid destination rejected during prepare." << std::endl;
    }

    // Vector 2.7: Expired proposal TTL
    {
        std::cout << "[Test 2.7] Expired proposal TTL..." << std::endl;
        TestFixture f;
        Kernel k;
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-expired"), "prepare");

        // Advance clock past 5-second deadline
        auto futureTime = f.now + std::chrono::seconds(6);
        check(k.syntheticConfirm(f.world, k.serial(), false, futureTime) == Result::Denied, "expired confirm");
        check(k.count() == 0, "count must remain 0");
        std::cout << "PASS: Expired proposal strictly rejected." << std::endl;
    }

    // Vector 2.8: Wrong seat / seat lock transition
    {
        std::cout << "[Test 2.8] Input seat stolen / mismatch..." << std::endl;
        TestFixture f;
        Kernel k;
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-seat"), "prepare");

        // Change active seat
        f.world.activeSeat = "seat-attacker";
        check(k.syntheticConfirm(f.world, k.serial(), false, f.now) == Result::Denied, "wrong seat confirm");
        check(k.count() == 0, "count must remain 0");
        std::cout << "PASS: Seat mismatch strictly rejected." << std::endl;
    }

    // Vector 2.9: Compositor restart / incarnation mismatch
    {
        std::cout << "[Test 2.9] Compositor restart / incarnation mismatch..." << std::endl;
        TestFixture f;
        Kernel k;
        check(k.prepare(f.world, f.windowObj.get(), "desktop-2", f.now, "op-comp-restart"), "prepare");

        // Simulate compositor reboot: new compositor incarnation ID
        f.world.compositor = 9999;
        check(k.syntheticConfirm(f.world, k.serial(), false, f.now) == Result::Denied, "stale compositor confirm");
        check(k.count() == 0, "count must remain 0");
        std::cout << "PASS: Stale compositor incarnation strictly rejected." << std::endl;
    }

    std::cout << "\n=== ALL 9 NATIVE AUTHORITY ADVERSARIAL TESTS PASSED ===" << std::endl;
    return 0;
}
