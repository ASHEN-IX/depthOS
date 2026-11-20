// DepthOS Spatial Workspace Alpha - Realistic Mock Hierarchy & Observation Data
// Used for standalone development, design previews, and offscreen test execution.

const MOCK_WORLDS = [
    {
        id: "world-engineering",
        artCategory: "projects",
        name: "Core Engineering",
        icon: "💻",
        description: "DepthOS kernel, spatial compositor, and platform daemons",
        color: "#6366F1",
        accent: "#818CF8"
    },
    {
        id: "world-security",
        artCategory: "research",
        name: "Security Research",
        icon: "🛡️",
        description: "Vulnerability analysis, action leases, and verification audits",
        color: "#EC4899",
        accent: "#F472B6"
    },
    {
        id: "world-design",
        artCategory: "social",
        name: "Creative & Design",
        icon: "🎨",
        description: "Spatial interaction design, UI shaders, and animation tokens",
        color: "#06B6D4",
        accent: "#22D3EE"
    },
    {
        id: "world-personal",
        artCategory: "focus",
        name: "Personal & Admin",
        icon: "☕",
        description: "Communication, daily planning, and task management",
        color: "#10B981",
        accent: "#34D399"
    }
];

const MOCK_WORKSPACES = [
    // Under Core Engineering
    {
        id: "ws-depthos-kernel",
        worldId: "world-engineering",
        name: "DepthOS-Kernel",
        description: "Rust workspace core and SQLite durability engine",
        rootDirectory: "~/Projects/DepthOS",
        gitBranch: "codex/phase-4-1"
    },
    {
        id: "ws-spatial-ux",
        worldId: "world-engineering",
        name: "Spatial-UX",
        description: "QtQuick 3D canvas and spatial navigation alpha",
        rootDirectory: "~/Projects/DepthOS/experiments/spatial-alpha",
        gitBranch: "feature/spatial-alpha"
    },
    {
        id: "ws-kwin-observer",
        worldId: "world-engineering",
        name: "KWin-Observer",
        description: "Observer replica and private IPC protocol",
        rootDirectory: "~/Projects/DepthOS/crates/depthos-observer",
        gitBranch: "main"
    },

    // Under Security Research
    {
        id: "ws-adversarial-tests",
        worldId: "world-security",
        name: "Adversarial-Tests",
        description: "Boundary checks, memory safety, and fuzzing harness",
        rootDirectory: "~/Projects/DepthOS/tests/adversarial",
        gitBranch: "audit/phase-3-10-1"
    },
    {
        id: "ws-protocol-formal",
        worldId: "world-security",
        name: "Protocol-Formal",
        description: "Durable journal invariant proofs and state machine models",
        rootDirectory: "~/Projects/DepthOS/docs/security",
        gitBranch: "main"
    },

    // Under Creative & Design
    {
        id: "ws-design-tokens",
        worldId: "world-design",
        name: "Design-Tokens",
        description: "Color tokens, spring curves, and typography scales",
        rootDirectory: "~/Design/DepthOS",
        gitBranch: "main"
    },

    // Under Personal & Admin
    {
        id: "ws-communications",
        worldId: "world-personal",
        name: "Communications",
        description: "Async team chat, email, and issue tracking",
        rootDirectory: "~/Personal",
        gitBranch: "main"
    }
];

const MOCK_DESKTOPS = [
    // Under ws-depthos-kernel
    {
        id: "desk-kernel-code",
        workspaceId: "ws-depthos-kernel",
        name: "Editor & Spec",
        role: "Primary coding surface"
    },
    {
        id: "desk-kernel-build",
        workspaceId: "ws-depthos-kernel",
        name: "Build & Test",
        role: "Cargo test compilation & terminal"
    },
    {
        id: "desk-kernel-docs",
        workspaceId: "ws-depthos-kernel",
        name: "Architecture Docs",
        role: "Technical specification & notes"
    },

    // Under ws-spatial-ux
    {
        id: "desk-spatial-scene",
        workspaceId: "ws-spatial-ux",
        name: "Scene Graph",
        role: "QML component modeling"
    },
    {
        id: "desk-spatial-preview",
        workspaceId: "ws-spatial-ux",
        name: "Live Preview",
        role: "Interactive viewport testing"
    },

    // Under ws-kwin-observer
    {
        id: "desk-observer-replica",
        workspaceId: "ws-kwin-observer",
        name: "Replica Monitor",
        role: "Observer socket trace viewer"
    },

    // Under ws-adversarial-tests
    {
        id: "desk-security-fuzz",
        workspaceId: "ws-adversarial-tests",
        name: "Fuzzer Shell",
        role: "Crash consistency suite"
    },

    // Under ws-protocol-formal
    {
        id: "desk-formal-proofs",
        workspaceId: "ws-protocol-formal",
        name: "Proof Workbench",
        role: "Theorem prover & journal verification"
    },

    // Under ws-design-tokens
    {
        id: "desk-design-canvas",
        workspaceId: "ws-design-tokens",
        name: "Figma & Icons",
        role: "Visual asset pipeline"
    },

    // Under ws-communications
    {
        id: "desk-comm-chat",
        workspaceId: "ws-communications",
        name: "Chat & Feed",
        role: "Team messaging"
    }
];

const MOCK_WINDOWS = [
    // Windows on desk-kernel-code
    {
        id: "win-101",
        desktopId: "desk-kernel-code",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000101",
        appId: "org.kde.kate",
        appName: "Kate",
        title: "controller.rs — DepthOS Core",
        minimized: false,
        fullscreen: false,
        focused: true,
        statusBadge: "Live", // Live observation
        statusReason: "Verified launch claim and confirmed PID match"
    },
    {
        id: "win-102",
        desktopId: "desk-kernel-code",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000102",
        appId: "alacritty",
        appName: "Alacritty",
        title: "ashenix@host: ~/Projects/DepthOS",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Live", // Live observation
        statusReason: "Single terminal instance matching root directory"
    },

    // Windows on desk-kernel-build
    {
        id: "win-103",
        desktopId: "desk-kernel-build",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000103",
        appId: "alacritty",
        appName: "Alacritty",
        title: "cargo test --all-targets",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Stale", // Stale observation (demonstrating stale warning)
        statusReason: "Observation sequence gap; awaiting next snapshot"
    },

    // Windows on desk-kernel-docs
    {
        id: "win-104",
        desktopId: "desk-kernel-docs",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000104",
        appId: "firefox",
        appName: "Firefox",
        title: "DepthOS Phase 4 Architecture — Wayland Protocols",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Live",
        statusReason: "Browser window associated with research target"
    },

    // Windows on desk-spatial-scene
    {
        id: "win-105",
        desktopId: "desk-spatial-scene",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000105",
        appId: "org.qt-project.qtcreator",
        appName: "Qt Creator",
        title: "SpatialScene.qml [Design Mode]",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Live",
        statusReason: "Active QML IDE instance"
    },
    {
        id: "win-106",
        desktopId: "desk-spatial-scene",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000106",
        appId: "alacritty",
        appName: "Alacritty",
        title: "qmltestrunner -input tst_spatial_alpha.qml",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Reconnecting", // Reconnecting observation
        statusReason: "KWin observer connection resetting"
    },

    // Windows on desk-spatial-preview
    {
        id: "win-107",
        desktopId: "desk-spatial-preview",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000107",
        appId: "org.depthos.spatial-alpha",
        appName: "DepthOS Spatial Viewport",
        title: "DepthOS Spatial Alpha [Wayland]",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Live",
        statusReason: "Self viewport observation"
    },

    // Windows on desk-observer-replica
    {
        id: "win-108",
        desktopId: "desk-observer-replica",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000108",
        appId: "org.kde.dolphin",
        appName: "Dolphin",
        title: "~/Projects/DepthOS/crates/depthos-observer",
        minimized: true,
        fullscreen: false,
        focused: false,
        statusBadge: "Unavailable", // Unavailable observation
        statusReason: "Window minimized with capture exclusion flag"
    },

    // Windows on desk-security-fuzz
    {
        id: "win-109",
        desktopId: "desk-security-fuzz",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000109",
        appId: "alacritty",
        appName: "Alacritty",
        title: "python3 tests/adversarial/run_all_adversarial.py",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Live",
        statusReason: "Security test execution"
    },

    // Windows on desk-comm-chat
    {
        id: "win-110",
        desktopId: "desk-comm-chat",
        nativeId: "a1b2c3d4-0001-4000-8000-000000000110",
        appId: "thunderbird",
        appName: "Thunderbird",
        title: "Inbox — DepthOS Engineering Updates",
        minimized: false,
        fullscreen: false,
        focused: false,
        statusBadge: "Unknown", // Unknown observation
        statusReason: "Unclassified multi-profile email client"
    }
];

function getMockData() {
    return {
        worlds: MOCK_WORLDS.slice(),
        workspaces: MOCK_WORKSPACES.slice(),
        desktops: MOCK_DESKTOPS.slice(),
        windows: MOCK_WINDOWS.slice()
    };
}

if (typeof module !== "undefined") {
    module.exports = {
        MOCK_WORLDS,
        MOCK_WORKSPACES,
        MOCK_DESKTOPS,
        MOCK_WINDOWS,
        getMockData
    };
}
