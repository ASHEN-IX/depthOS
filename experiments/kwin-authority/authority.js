/* Read-only KWin scripting prototype. The private fixture supplies its observer.
 * Neither a live snapshot nor the retained QObject establishes user consent.
 */
function createDenyOnlyAuthority(workspace, compositorIncarnation, onInvalidated) {
    const entries = new Map();
    const subscriptions = [];
    const lifetime = new QTimer();
    let pending = null;
    let nextGeneration = 0;
    let topologyRevision = 1;
    let started = false;
    lifetime.interval = 4000;
    lifetime.singleShot = true;
    lifetime.timeout.connect(() => invalidate("expired"));

    function connect(signal, callback, bag) {
        signal.connect(callback);
        bag.push([signal, callback]);
    }

    function disconnect(bag) {
        for (const [signal, callback] of bag.splice(0)) {
            try { signal.disconnect(callback); } catch (_) { /* QObject was destroyed. */ }
        }
    }

    function invalidate(reason) {
        if (!pending) return;
        const { proposal } = pending;
        pending = null;
        lifetime.stop();
        onInvalidated(reason, proposal);
    }

    function discard(window) {
        const entry = entries.get(window);
        if (!entry) return;
        entries.delete(window);
        disconnect(entry.subscriptions);
        if (pending && pending.window === window) invalidate("window-removed");
    }

    function nativeUuid(window) {
        const match = String(window.internalId).toLowerCase().match(
            /^(?:\{([0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12})\}|([0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}))$/
        );
        const id = match && (match[1] || match[2]);
        return id !== "00000000-0000-0000-0000-000000000000" ? id : null;
    }

    function track(window) {
        if (entries.has(window) || window.deleted !== false) return;
        const entry = { generation: ++nextGeneration, subscriptions: [] };
        try {
            connect(window.closed, () => discard(window), entry.subscriptions);
            for (const signal of ["desktopsChanged", "activitiesChanged", "skipTaskbarChanged",
                "minimizedChanged", "excludeFromCaptureChanged"]) {
                connect(window[signal], () => {
                    if (pending && pending.window === window) invalidate("window-state");
                }, entry.subscriptions);
            }
            entries.set(window, entry);
            if (pending && nativeUuid(window) === pending.proposal.nativeUuid) invalidate("ambiguous-window");
        } catch (_) {
            entries.delete(window);
            disconnect(entry.subscriptions);
            invalidate("window-state-unavailable");
        }
    }

    function topologyChanged() {
        ++topologyRevision;
        invalidate("topology");
    }

    function start() {
        if (started) return;
        try {
            connect(workspace.windowAdded, track, subscriptions);
            connect(workspace.windowRemoved, discard, subscriptions);
            for (const name of ["desktopsChanged", "screensChanged", "activitiesChanged"])
                connect(workspace[name], topologyChanged, subscriptions);
            connect(workspace.currentDesktopChanged, () => invalidate("desktop"), subscriptions);
            connect(workspace.currentActivityChanged, () => invalidate("activity"), subscriptions);
            connect(workspace.windowActivated, () => {
                if (pending && workspace.activeWindow !== pending.window) invalidate("focus");
            }, subscriptions);
            for (const window of workspace.windowList()) track(window);
            started = true;
        } catch (error) {
            stop();
            throw error;
        }
    }

    function inspect(window, destinationId) {
        try {
            const entry = entries.get(window);
            if (!started || !entry || window.deleted !== false || window.managed !== true ||
                window.normalWindow !== true || window.popupWindow !== false ||
                window.excludeFromCapture !== false || window.minimized !== false ||
                window.skipTaskbar !== false || workspace.activeWindow !== window) return null;
            const uuid = nativeUuid(window);
            if (!uuid) return null;
            const matches = Array.from(workspace.windowList()).filter(other =>
                other.deleted === false && nativeUuid(other) === uuid);
            if (matches.length !== 1 || matches[0] !== window) return null;

            const desktops = Array.from(workspace.desktops);
            const membership = Array.from(window.desktops);
            const ids = desktops.map(desktop => desktop.id);
            if (ids.some(id => typeof id !== "string" || !id) || new Set(ids).size !== ids.length ||
                membership.length !== 1 || !desktops.includes(membership[0]) ||
                workspace.currentDesktop !== membership[0] || !ids.includes(destinationId) ||
                membership[0].id === destinationId) return null;
            const activity = workspace.currentActivity;
            const windowActivities = Array.from(window.activities);
            if (typeof activity !== "string" || !activity ||
                Array.from(workspace.activities).filter(id => id === activity).length !== 1 ||
                (windowActivities.length && (windowActivities.length !== 1 ||
                    windowActivities[0] !== activity))) return null;

            return {
                nativeUuid: uuid,
                generation: entry.generation,
                trackerRuntimeId: "window-" + entry.generation,
                compositorIncarnation,
                topologyRevision,
                sourceDesktop: membership[0].id,
                destinationDesktop: destinationId,
                activity,
                activityMembership: windowActivities[0] || null
            };
        } catch (_) {
            return null;
        }
    }

    function capture(window, destinationId) {
        if (pending) return null;
        const state = inspect(window, destinationId);
        if (!state) return null;
        const proposal = Object.freeze(Object.assign({ operation: "desktop-placement-preview" }, state));
        pending = { window, proposal };
        lifetime.start();
        return proposal;
    }

    function revalidate(proposal) {
        if (!pending || proposal !== pending.proposal) return false;
        const current = inspect(pending.window, proposal.destinationDesktop);
        if (current && Object.keys(current).every(key => current[key] === proposal[key])) return true;
        invalidate("stale");
        return false;
    }

    function stop() {
        started = false;
        invalidate("shutdown");
        disconnect(subscriptions);
        for (const entry of entries.values()) disconnect(entry.subscriptions);
        entries.clear();
    }

    return Object.freeze({ start, capture, revalidate, stop });
}
