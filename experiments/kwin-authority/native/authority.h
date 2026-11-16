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
// These values are owned by the test, never observations received over IPC.
struct Window {
    QPointer<QObject> object;
    std::string uuid;
    std::uint64_t generation = 1;
    std::string desktop = "source";
    std::string activity = "activity";
    bool eligible = true;
};
struct World {
    std::vector<Window> windows;
    QPointer<QObject> focus;
    QPointer<QObject> prompt;
    std::set<std::string> desktops{"source", "destination"};
    std::string currentDesktop = "source";
    std::string activity = "activity";
    std::string seat = "seat0";
    std::uint64_t compositor = 1, coordinator = 1, observer = 1, topology = 1;
    std::optional<bool> locked = false;
    bool promptOwned = true;
    bool alive = true;
};
struct Proposal {
    const QPointer<QObject> object;
    const QPointer<QObject> prompt;
    const std::string uuid, source, destination, activity, seat;
    const std::uint64_t generation, compositor, coordinator, observer, topology, serial;
    const std::chrono::steady_clock::time_point issued, deadline;
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
                 Clock::time_point now = Clock::now()) {
        if (!onThread() || pending_ || retired_ || serial_ == std::numeric_limits<std::uint64_t>::max())
            return false;
        const Window *window = unique(world, object);
        if (!window || !live(world, *window, destination)) return false;
        pending_.emplace(Proposal{window->object, world.prompt, window->uuid,
            window->desktop, destination, window->activity, world.seat,
            window->generation, world.compositor, world.coordinator, world.observer,
            world.topology, ++serial_, now, now + std::chrono::seconds(5)});
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
    std::uint64_t serial() const { return serial_; }
    std::uint64_t count() const { return count_; }

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
    static bool live(const World &world, const Window &window, const std::string &destination) {
        return world.alive && world.locked.has_value() && !*world.locked &&
            window.object && window.eligible && world.focus == window.object &&
            world.prompt && world.promptOwned && !world.seat.empty() &&
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
} // namespace synthetic
#endif
} // namespace depthos::experiment
