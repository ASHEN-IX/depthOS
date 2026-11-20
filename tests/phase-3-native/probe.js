// Appended only to the private fixture's script. No test approval path.
(function () {
    const report = payload => print("DEPTHOS_AUTHORITY " + JSON.stringify(payload));
    const authority = createDenyOnlyAuthority(workspace, compositorIncarnation, (reason, proposal) =>
        report({ event: "invalidated", reason, generation: proposal.generation,
            staleRefused: !authority.revalidate(proposal) }));
    const seen = new Set();

    function captureFixture() {
        const window = workspace.activeWindow;
        if (!window || !fixturePids.includes(window.pid) || seen.has(window.pid)) return;
        const destinations = Array.from(workspace.desktops).filter(d => d !== workspace.currentDesktop);
        if (destinations.length !== 1 || workspace.activities.length !== 1) return;
        const proposal = authority.capture(window, destinations[0].id);
        if (!proposal) return;
        seen.add(window.pid);
        report({
            event: "captured",
            liveRevalidated: authority.revalidate(proposal),
            foreignProposalRefused: !authority.revalidate(Object.assign({}, proposal)),
            pendingConflictRefused: authority.capture(window, destinations[0].id) === null,
            immutable: Object.isFrozen(proposal),
            distinctDesktops: proposal.sourceDesktop !== proposal.destinationDesktop,
            pid: window.pid,
            generation: proposal.generation,
            incarnation: proposal.compositorIncarnation,
            runtimeId: proposal.trackerRuntimeId,
            preview: {
                sourceDesktop: proposal.sourceDesktop,
                destinationDesktop: proposal.destinationDesktop,
                activity: proposal.activity
            }
        });
    }

    authority.start();
    workspace.windowActivated.connect(captureFixture);
    workspace.windowRemoved.connect(window => {
        if (fixturePids.includes(window.pid)) report({ event: "fixture-removed", pid: window.pid });
    });
    // Startup/readiness polling is bounded by the launcher. It never grants consent.
    const readiness = new QTimer();
    readiness.interval = 50;
    readiness.timeout.connect(captureFixture);
    readiness.start();
    report({ event: "ready" });
})();
