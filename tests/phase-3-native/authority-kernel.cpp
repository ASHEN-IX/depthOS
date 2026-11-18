#include "authority.h"
#include <QCoreApplication>
#include <QMetaObject>
#include <functional>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <thread>

using namespace depthos::experiment;
static void check(bool passed, const char *label) {
    if (!passed) throw std::runtime_error(label);
}

int main(int argc, char **argv) {
    QCoreApplication application(argc, argv);
    DenyOnlyAuthority authority;
    for (Origin origin : {Origin::Unknown, Origin::ClaimedPhysical, Origin::Qt, Origin::Qml,
                          Origin::Eis, Origin::Remote, Origin::Accessibility})
        check(authority.confirm(origin) == Result::Denied, "untrusted origin authorized");
    std::cout << "PASS all seven public origin claims denied\n";
#ifdef DEPTHOS_AUTHORITY_UNIT_TEST
    using namespace synthetic;
    struct Fixture {
        std::unique_ptr<QObject> window = std::make_unique<QObject>();
        QObject prompt, replacement;
        World world;
        Kernel kernel;
        const Kernel::Clock::time_point now = Kernel::Clock::now();
        Fixture() {
            world.windows.push_back(Window{window.get(), "uuid"});
            world.focus = window.get();
            world.prompt = &prompt;
            check(kernel.prepare(world, window.get(), "destination", now), "prepare");
        }
        Result confirm(bool lose = false) {
            return kernel.syntheticConfirm(world, kernel.serial(), lose, now);
        }
    };
    const std::vector<std::pair<const char *, std::function<void(Fixture &)>>> adversarial{
        {"focus loss", [](auto &f) { f.world.focus = nullptr; }},
        {"locked", [](auto &f) { f.world.locked = true; }},
        {"unknown lock", [](auto &f) { f.world.locked.reset(); }},
        {"wrong seat", [](auto &f) { f.world.seat = "seat1"; }},
        {"unknown seat", [](auto &f) { f.world.seat.clear(); }},
        {"prompt replaced", [](auto &f) { f.world.prompt = &f.replacement; }},
        {"prompt absent", [](auto &f) { f.world.prompt = nullptr; }},
        {"prompt unowned", [](auto &f) { f.world.promptOwned = false; }},
        {"source changed", [](auto &f) { f.world.windows[0].desktop = "destination"; }},
        {"activity changed", [](auto &f) { f.world.activity = "other"; }},
        {"ineligible", [](auto &f) { f.world.windows[0].eligible = false; }},
        {"destination removed", [](auto &f) { f.world.desktops.erase("destination"); }},
        {"topology drift", [](auto &f) { ++f.world.topology; }},
        {"observer replaced", [](auto &f) { ++f.world.observer; }},
        {"coordinator drift", [](auto &f) { ++f.world.coordinator; }},
        {"compositor replaced", [](auto &f) { ++f.world.compositor; }},
        {"generation drift", [](auto &f) { ++f.world.windows[0].generation; }},
        {"UUID changed", [](auto &f) { f.world.windows[0].uuid = "different"; }},
        {"duplicate UUID", [](auto &f) { f.world.windows.push_back(Window{&f.replacement, "uuid"}); }},
        {"duplicate object", [](auto &f) { f.world.windows.push_back(f.world.windows[0]); }},
        {"destruction", [](auto &f) { f.window.reset(); }},
        {"same UUID recreation", [](auto &f) {
            f.window.reset(); f.window = std::make_unique<QObject>();
            f.world.windows[0].object = f.window.get(); f.world.focus = f.window.get();
        }},
        {"compositor teardown", [](auto &f) { f.world.alive = false; }},
        {"authority teardown", [](auto &f) { f.kernel.teardown(); }},
        {"cancel wins", [](auto &f) { check(f.kernel.cancel(), "cancel failed"); }},
    };
    for (const auto &[label, attack] : adversarial) {
        Fixture fixture;
        attack(fixture);
        check(fixture.confirm() == Result::Denied, label);
        check(fixture.kernel.count() == 0, label);
        check(fixture.confirm() == Result::Denied, "replay after denial");
        std::cout << "PASS synthetic denial: " << label << '\n';
    }
    {
        Fixture f;
        check(!f.kernel.prepare(f.world, f.window.get(), "destination", f.now), "pending conflict");
        check(f.kernel.syntheticConfirm(f.world, f.kernel.serial() + 1, false, f.now) == Result::Denied,
              "foreign serial");
        check(f.confirm() == Result::Counted && f.kernel.count() == 1, "valid count");
        check(!f.kernel.cancel(), "cancel after count");
        check(f.confirm() == Result::Denied && f.kernel.count() == 1, "double consume");
        std::cout << "PASS synthetic single-use, foreign token, conflict, confirm wins\n";
    }
    {
        Fixture f;
        check(f.confirm(true) == Result::Uncertain, "lost ack");
        check(f.confirm() == Result::Denied && f.kernel.count() == 1, "uncertain replay");
        check(!f.kernel.prepare(f.world, f.window.get(), "destination", f.now), "uncertain reprepare");
        std::cout << "PASS synthetic uncertain completion never retries\n";
    }
    {
        Fixture f;
        check(f.kernel.syntheticConfirm(f.world, f.kernel.serial(), false,
              f.now - std::chrono::nanoseconds(1)) == Result::Denied, "clock reversal");
        check(f.kernel.count() == 0, "reversed count");
        std::cout << "PASS synthetic clock reversal denial\n";
    }
    {
        Fixture f;
        check(f.kernel.syntheticConfirm(f.world, f.kernel.serial(), false,
              f.now + std::chrono::seconds(5)) == Result::Denied, "deadline inclusive");
        check(f.kernel.count() == 0, "expired count");
        std::cout << "PASS synthetic steady-clock exact expiry\n";
    }
    {
        QObject window, prompt;
        World world;
        world.windows.push_back(Window{&window, "uuid"});
        world.focus = &window;
        world.prompt = &prompt;
        Kernel kernel;
        check(!kernel.prepare(world, &window, "destination", Kernel::Clock::time_point::max()),
              "overflowing deadline accepted");
        const auto latest = Kernel::Clock::time_point::max() - std::chrono::seconds(5);
        check(kernel.prepare(world, &window, "destination", latest), "latest safe deadline");
        check(kernel.syntheticConfirm(world, kernel.serial(), false, latest) == Result::Counted,
              "safe extreme time refused");
        std::cout << "PASS synthetic deadline overflow refused and safe boundary accepted\n";
    }
    for (bool movePrompt : {false, true}) {
        Fixture f;
        QThread other;
        other.start();
        QThread *owner = QThread::currentThread();
        QObject *moved = movePrompt ? &f.prompt : f.window.get();
        check(moved->moveToThread(&other), "move object to foreign thread");
        const Result result = f.confirm();
        Kernel fresh;
        const bool prepared = fresh.prepare(f.world, f.window.get(), "destination", f.now);
        check(QMetaObject::invokeMethod(moved, [&] {
            check(moved->moveToThread(owner), "restore object affinity");
        }, Qt::BlockingQueuedConnection), "restore invocation");
        other.quit();
        check(other.wait(3000), "foreign thread cleanup");
        check(result == Result::Denied && f.kernel.count() == 0 && !prepared,
              "foreign QObject affinity accepted");
        check(f.confirm() == Result::Denied, "affinity restoration revived consumed proposal");
        std::cout << "PASS synthetic foreign " << (movePrompt ? "prompt" : "window")
                  << " affinity denied at prepare and consume\n";
    }
    for (bool lockTransition : {false, true}) {
        Fixture f;
        // A future observer must call cancel on each transition, even when
        // the subsequent snapshot returns to identical values.
        if (lockTransition) f.world.locked = true;
        else f.world.focus = nullptr;
        check(f.kernel.cancel(), "transition invalidation");
        f.world.locked = false;
        f.world.focus = f.window.get();
        check(f.confirm() == Result::Denied && f.kernel.count() == 0,
              "restored state revived invalidated proposal");
        std::cout << "PASS synthetic explicit transition cancellation survives state restoration\n";
    }
    {
        Fixture f;
        Result result = Result::Counted;
        std::thread other([&] { result = f.confirm(); });
        other.join();
        check(result == Result::Denied && f.kernel.count() == 0, "wrong thread");
        check(f.confirm() == Result::Counted, "owner thread after foreign refusal");
        std::cout << "PASS synthetic wrong-thread denial\n";
    }
#endif
}
