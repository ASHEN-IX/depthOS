"""Pure ownership decision model for Phase 2.3A synthetic fixtures."""
from dataclasses import dataclass
from enum import IntEnum
from typing import Mapping


@dataclass(frozen=True, order=True)
class WindowKey:
    epoch: str
    runtime_id: str


@dataclass(frozen=True)
class Window:
    eligible: bool
    parent: WindowKey | None = None
    dialog: bool = False


class Source(IntEnum):
    PARENT_HINT = 0
    HEURISTIC = 1
    EXPLICIT_POLICY = 2
    VERIFIED_LAUNCH = 3
    USER = 4


@dataclass(frozen=True)
class Claim:
    """Prepared by a typed trusted producer, never deserialized from the observer.

    An empty target is reserved for a user unassignment. Multiple targets form
    one explicit shared assignment, so independent claims are never merged.
    """
    window: WindowKey
    source: Source
    targets: frozenset[str]

    def __post_init__(self):
        if not self.targets and self.source != Source.USER:
            raise ValueError("only a user override can suppress ownership")
        if len(self.targets) > 1 and self.source not in {Source.USER, Source.EXPLICIT_POLICY}:
            raise ValueError("shared ownership requires an explicit choice")


@dataclass(frozen=True)
class Decision:
    status: str
    reason: str
    sources: tuple[str, ...] = ()
    targets: tuple[str, ...] = ()
    alternatives: tuple[tuple[str, ...], ...] = ()


def project_window(record: Mapping, epoch: str) -> tuple[WindowKey, Window]:
    """Take only reconciled identity, classification and parent fields."""
    runtime = record["runtimeId"]
    if not isinstance(runtime, str) or not runtime:
        raise ValueError("invalid runtime ID")
    parent = record.get("parentRuntimeId")
    eligible = (
        record.get("classification") in ("application", "dialog")
        and record.get("excludeFromCapture") is False
        and record.get("skipTaskbar") is False
    )
    parent_key = WindowKey(epoch, parent) if isinstance(parent, str) and parent else None
    return WindowKey(epoch, runtime), Window(eligible, parent_key, record.get("classification") == "dialog")


def parent_is_valid(key: WindowKey, windows: Mapping[WindowKey, Window]) -> bool:
    seen = {key}
    window = windows[key]
    current = window.parent
    if not window.dialog or current is None:
        return False
    while current is not None:
        if current.epoch != key.epoch or current in seen:
            return False
        parent = windows.get(current)
        if parent is None or not parent.eligible:
            return False
        seen.add(current)
        current = parent.parent
    return True


def decide(
    key: WindowKey,
    *,
    epoch: str,
    ready: bool,
    windows: Mapping[WindowKey, Window],
    workspaces: frozenset[str],
    claims: tuple[Claim, ...] = (),
) -> Decision:
    if not ready or key.epoch != epoch or key not in windows:
        return Decision("stale", "observation_unavailable_or_replaced")
    if not windows[key].eligible:
        return Decision("unknown", "window_outside_policy_scope")

    applicable = [claim for claim in claims if claim.window == key]
    if any(claim.source == Source.PARENT_HINT for claim in applicable):
        if not parent_is_valid(key, windows):
            applicable = [claim for claim in applicable if claim.source != Source.PARENT_HINT]
    if not applicable:
        return Decision("unknown", "no_applicable_evidence")

    # Both kinds of suggestions are peers. A disagreement remains visible.
    def rank(claim: Claim) -> int:
        return max(Source.HEURISTIC, claim.source)

    winning_rank = max(map(rank, applicable))
    winners = [claim for claim in applicable if rank(claim) == winning_rank]
    sources = tuple(sorted({claim.source.name.lower() for claim in winners}))
    if any(not claim.targets <= workspaces for claim in winners):
        return Decision("stale", "workspace_removed", sources)
    alternatives = tuple(sorted({tuple(sorted(claim.targets)) for claim in winners}))
    if len(alternatives) > 1:
        return Decision("conflicted", "equal_precedence_disagreement", sources, alternatives=alternatives)

    targets = alternatives[0]
    if not targets:
        return Decision("unknown", "user_unassigned", sources)
    if winning_rank == Source.HEURISTIC:
        return Decision("suggested", "review_required", sources, targets)
    reason = {
        Source.USER: "user_confirmed",
        Source.VERIFIED_LAUNCH: "verified_window_launch",
        Source.EXPLICIT_POLICY: "explicit_binding_matched",
    }[winning_rank]
    return Decision("confirmed", reason, sources, targets)
