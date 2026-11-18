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
} // namespace synthetic
#endif
} // namespace depthos::experiment
