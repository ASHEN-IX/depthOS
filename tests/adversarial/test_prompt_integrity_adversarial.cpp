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

    std::cout << "\n=== ALL NATIVE AUTHORITY ADVERSARIAL TESTS PASSED ===" << std::endl;
#endif

    return 0;
}
