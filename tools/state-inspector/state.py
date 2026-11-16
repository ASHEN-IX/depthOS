"""In-memory observer replica. No D-Bus, shell, I/O, or persistence here."""
import copy
import json
import math
import unicodedata

SCHEMA = 1
MAX_BYTES = 4 * 1024 * 1024
WINDOW_EVENTS = {
    "window.added", "window.metadata.changed", "window.desktop.changed",
    "window.activity.changed", "window.geometry.changed", "window.state.changed",
}
GLOBAL_FIELDS = {
    "window.focus.changed": ("activeWindowId",),
    "desktop.list.changed": ("desktops",),
    "desktop.active.changed": ("activeDesktopId", "activeDesktopByScreen"),
    "activity.list.changed": ("activities",),
    "activity.active.changed": ("activeActivityId",),
    "screen.list.changed": ("screens",),
}


def safe_terminal(value, limit=160):
    """Never interpret titles as terminal control sequences or multiline output."""
    if value is None:
        return "?"
    return "".join(c for c in str(value) if not unicodedata.category(c).startswith("C"))[:limit]


def redacted(state):
    result = copy.deepcopy(state)
    if result is not None:
        for window in result.get("windows", []):
            window["title"] = "<redacted>" if window.get("title") is not None else None
    return result


def validate_window(window):
    if not isinstance(window, dict) or not isinstance(window.get("runtimeId"), str):
        raise ValueError("invalid window")
    for key in ("title", "applicationId", "resourceClass", "resourceName", "nativeId", "parentRuntimeId", "screenId", "windowingSystem"):
        if window.get(key) is not None and not isinstance(window[key], str):
            raise ValueError("invalid window metadata")
    for key in ("desktopIds", "activityIds"):
        if window.get(key) is not None and (not isinstance(window[key], list) or any(not isinstance(i, str) for i in window[key])):
            raise ValueError("invalid membership")
    for key in ("onAllDesktops", "onAllActivities", "minimized", "fullscreen", "skipTaskbar", "excludeFromCapture"):
        if window.get(key) is not None and type(window[key]) is not bool:
            raise ValueError("invalid window flag")
    if window.get("classification") not in ("application", "dialog", "utility", "system", "unknown"):
        raise ValueError("invalid classification")
    geometry = window.get("geometry")
    if geometry is not None and (not isinstance(geometry, dict) or any(
            type(geometry.get(k)) not in (int, float) or not math.isfinite(geometry[k]) for k in ("x", "y", "width", "height"))):
        raise ValueError("invalid geometry")


def validate_global(field, value):
    if field in ("activeWindowId", "activeDesktopId", "activeActivityId"):
        valid = value is None or isinstance(value, str)
    elif field == "activeDesktopByScreen":
        valid = value is None or (isinstance(value, dict) and all(v is None or isinstance(v, str) for v in value.values()))
    elif field == "activities":
        valid = value is None or (isinstance(value, list) and all(isinstance(v, str) for v in value))
    else:  # desktops/screens
        valid = value is None or (isinstance(value, list) and all(isinstance(v, dict) and isinstance(v.get("id"), str) for v in value))
    if not valid:
        raise ValueError("invalid global state")


class Replica:
    def __init__(self):
        self.session_id = None
        self.sequence = 0
        self.revision = 0
        self.state = None
        self.need_snapshot = True
        self.last_event = None
        self.resyncs = 0
        self.status = None

    def reset(self):
        self.__init__()

    def apply(self, frame):
        if not isinstance(frame, dict) or frame.get("schemaVersion") != SCHEMA:
            raise ValueError("unsupported frame")
        session = frame.get("sessionId")
        sequence, revision = frame.get("sequence"), frame.get("revision")
        if not isinstance(session, str) or not session or len(session) > 256:
            raise ValueError("invalid session")
        if type(sequence) is not int or sequence < 1 or type(revision) is not int or revision < 1:
            raise ValueError("invalid position")
        kind = frame.get("type")
        if self.session_id == session and sequence <= self.sequence:
            return  # Duplicate/replayed delivery cannot roll the replica backwards.
        if kind == "snapshot":
            if session == self.session_id and revision < self.revision:
                raise ValueError("snapshot revision regressed")
            state = frame.get("state")
            if not isinstance(state, dict) or not isinstance(state.get("windows"), list):
                raise ValueError("invalid snapshot")
            ids = [w.get("runtimeId") for w in state["windows"] if isinstance(w, dict)]
            if len(ids) != len(state["windows"]) or any(not isinstance(i, str) for i in ids) or len(set(ids)) != len(ids):
                raise ValueError("duplicate or invalid window ids")
            if not all(k in state for k in ("desktops", "activities", "screens", "activeWindowId", "activeDesktopId", "activeActivityId")):
                raise ValueError("incomplete snapshot")
            for window in state["windows"]:
                validate_window(window)
            for fields in GLOBAL_FIELDS.values():
                for field in fields:
                    validate_global(field, state.get(field))
            if not isinstance(state.get("applicationWindowIds"), list) or any(not isinstance(i, str) for i in state["applicationWindowIds"]):
                raise ValueError("invalid application collection")
            self.state = copy.deepcopy(state)
            self.session_id, self.sequence, self.revision = session, sequence, revision
            self.need_snapshot = False
            self.last_event = {"type": kind, "sequence": sequence, "reason": frame.get("reason")}
            return
        if self.need_snapshot or session != self.session_id or sequence != self.sequence + 1 or revision != self.revision + 1:
            self.need_snapshot = True
            self.resyncs += 1
            return
        state = copy.deepcopy(self.state)
        windows = {w["runtimeId"]: w for w in state["windows"]}
        if kind in WINDOW_EVENTS:
            window_id, window = frame.get("windowId"), frame.get("window")
            if not isinstance(window, dict) or window.get("runtimeId") != window_id:
                raise ValueError("invalid window event")
            validate_window(window)
            if kind != "window.added" and window_id not in windows:
                self.need_snapshot = True
                return
            windows[window_id] = copy.deepcopy(window)
        elif kind == "window.removed":
            windows.pop(frame.get("windowId"), None)
            if state["activeWindowId"] == frame.get("windowId"):
                state["activeWindowId"] = None
        elif kind in GLOBAL_FIELDS:
            for field in GLOBAL_FIELDS[kind]:
                if field not in frame:
                    raise ValueError("missing changed global field")
                validate_global(field, frame[field])
                state[field] = copy.deepcopy(frame[field])
        else:
            raise ValueError("unknown event type")
        state["windows"] = sorted(windows.values(), key=lambda w: w["runtimeId"])
        state["applicationWindowIds"] = [w["runtimeId"] for w in state["windows"]
            if w.get("classification") == "application" and w.get("skipTaskbar") is not True and w.get("excludeFromCapture") is not True]
        self.state, self.sequence, self.revision = state, sequence, revision
        self.last_event = {"type": kind, "sequence": sequence, "windowId": frame.get("windowId")}

    def accept(self, packet, consumer_id):
        if not isinstance(packet, dict) or packet.get("schemaVersion") != SCHEMA or packet.get("consumerId") != consumer_id:
            raise ValueError("invalid packet")
        self.status = packet.get("status") if packet.get("status") == "payload-too-large" else None
        if self.status:
            self.need_snapshot = True
        frames = packet.get("frames")
        if not isinstance(frames, list) or len(frames) > 32:
            raise ValueError("invalid batch")
        if any(not isinstance(f, dict) or f.get("sessionId") != packet.get("sessionId") for f in frames):
            raise ValueError("inconsistent session")
        for frame in frames:
            self.apply(frame)
        position = packet.get("position")
        if not isinstance(position, dict) or type(position.get("sequence")) is not int:
            raise ValueError("invalid heartbeat")
        # A packet can contain only the first 32 queued events. Ahead is normal
        # in that case; an empty heartbeat ahead means events have been lost.
        if not frames and (self.session_id != packet.get("sessionId") or self.sequence != position["sequence"]):
            self.need_snapshot = True
        return {"consumerId": consumer_id, "needSnapshot": self.need_snapshot}


def decode(raw):
    if not isinstance(raw, str) or len(raw.encode("utf-8")) > MAX_BYTES:
        raise ValueError("packet exceeds byte limit")
    def reject_constant(_):
        raise ValueError("non-finite JSON")
    return json.loads(raw, parse_constant=reject_constant)
