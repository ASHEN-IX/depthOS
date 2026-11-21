import QtQuick
import "theme"

FocusScope {
    id: root
    property bool isOpen: false
    property var hostBridge: null
    property int currentStep: 1
    property int totalSteps: 6
    property string errorMessage: ""
    property bool motionOff: false

    // Form state across wizard steps
    property string worldName: "Primary World"
    property string worldDescription: "General work, projects, and personal exploration"
    property string worldArtwork: "cyber-grid"
    property string worldTone: "neon-cyan"

    property string workspaceName: "Development"
    property string workspaceDescription: "Core development projects and tools"
    property string projectRoot: ""

    property string desktopName: "Main Desktop"
    property string desktopRole: "development"

    property bool prefReducedMotion: false
    property bool prefLowEffects: false
    property bool prefHighContrast: false
    property bool prefShowTitles: false
    property string prefLandingView: "worlds"

    signal completed()
    signal quickSetupRequested()
    signal demoModeRequested()

    visible: opacity > 0 || isOpen
    enabled: isOpen
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(root.motionOff, NeonTheme.modalMs) }
    }

    onIsOpenChanged: {
        if (isOpen) {
            root.currentStep = 1;
            root.errorMessage = "";
        }
    }

    function finishWizard() {
        if (!root.hostBridge) {
            root.completed();
            return;
        }

        // 1. Save user preferences
        const prefs = {
            reducedMotion: root.prefReducedMotion,
            lowEffects: root.prefLowEffects,
            highContrast: root.prefHighContrast,
            showTitles: root.prefShowTitles,
            accentIntensity: "standard",
            defaultLandingView: root.prefLandingView,
            lastWorldId: null,
            lastWorkspaceId: null,
            lastDesktopId: null
        };
        root.hostBridge.executeManagement(JSON.stringify({
            action: "saveUserPreferences",
            payload: prefs
        }));

        // 2. Create World
        const wResp = JSON.parse(root.hostBridge.executeManagement(JSON.stringify({
            action: "create",
            entity: "world",
            payload: {
                name: root.worldName,
                description: root.worldDescription,
                visualIdentity: root.worldTone,
                artwork: root.worldArtwork
            }
        })));
        if (!wResp.success) {
            root.errorMessage = "Failed creating world: " + (wResp.error || "unknown error");
            return;
        }
        const worldId = wResp.id;

        // 3. Create Workspace
        const wsResp = JSON.parse(root.hostBridge.executeManagement(JSON.stringify({
            action: "create",
            entity: "workspace",
            parentId: worldId,
            payload: {
                name: root.workspaceName,
                description: root.workspaceDescription,
                rootDirectory: root.projectRoot.trim() || undefined,
                visualIdentity: root.worldTone
            }
        })));
        if (!wsResp.success) {
            root.errorMessage = "Failed creating workspace: " + (wsResp.error || "unknown error");
            return;
        }
        const wsId = wsResp.id;

        // 4. Create Desktop
        const dtResp = JSON.parse(root.hostBridge.executeManagement(JSON.stringify({
            action: "create",
            entity: "desktop",
            parentId: wsId,
            payload: {
                name: root.desktopName,
                role: root.desktopRole,
                visualIdentity: root.worldTone
            }
        })));
        if (!dtResp.success) {
            root.errorMessage = "Failed creating desktop: " + (dtResp.error || "unknown error");
            return;
        }

        root.hostBridge.reloadHierarchy();
        root.completed();
    }

    Rectangle {
        anchors.fill: parent
        color: NeonTheme.scrim
        MouseArea { anchors.fill: parent }
    }

    SpatialGlass {
        width: Math.min(680, parent.width - 40)
        height: Math.min(600, parent.height - 40)
        anchors.centerIn: parent
        radius: 20
        rim: NeonTheme.cyan
        border.color: NeonTheme.cyan
        border.width: 1

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Column {
            anchors.fill: parent
            anchors.margins: 26
            spacing: 16

            // Header & Step indicator
            Row {
                width: parent.width
                spacing: 12

                Column {
                    width: parent.width - quickBtn.implicitWidth - demoBtn.implicitWidth - 24
                    spacing: 2
                    NeonLabel { text: "SETUP WIZARD  ·  STEP " + root.currentStep + " OF " + root.totalSteps; color: NeonTheme.cyan }
                    NeonText {
                        text: {
                            switch (root.currentStep) {
                            case 1: return "Welcome to DepthOS";
                            case 2: return "1. Define First World";
                            case 3: return "2. Configure Workspace";
                            case 4: return "3. Initial Logical Desktop";
                            case 5: return "4. Personalize Preferences";
                            case 6: return "5. Verify & Initialize";
                            default: return "Onboarding";
                            }
                        }
                        font.pixelSize: 22
                        font.weight: Font.DemiBold
                    }
                }

                NeonButton {
                    id: demoBtn
                    text: "Demo Mode"
                    compact: true
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: root.demoModeRequested()
                }

                NeonButton {
                    id: quickBtn
                    text: "Quick Setup"
                    compact: true
                    highlighted: true
                    accent: NeonTheme.emerald
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: root.quickSetupRequested()
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            // Step Progress Bar
            Row {
                width: parent.width
                spacing: 6
                Repeater {
                    model: root.totalSteps
                    delegate: Rectangle {
                        required property int index
                        width: (parent.width - (root.totalSteps - 1) * 6) / root.totalSteps
                        height: 4
                        radius: 2
                        color: index + 1 <= root.currentStep ? NeonTheme.cyan : NeonTheme.border
                    }
                }
            }

            NeonText {
                visible: root.errorMessage !== ""
                text: root.errorMessage
                color: NeonTheme.danger
                font.pixelSize: 12
            }

            // Step Content container
            Item {
                width: parent.width
                height: 380

                // STEP 1: Overview
                Column {
                    visible: root.currentStep === 1
                    anchors.fill: parent
                    spacing: 14

                    NeonText {
                        width: parent.width
                        wrapMode: Text.WordWrap
                        text: "DepthOS organizes your workspace into a unified, cyber-spatial hierarchy. Each layer gives structure to your real tasks without moving native windows automatically."
                        font.pixelSize: 14
                        color: NeonTheme.text
                    }

                    Rectangle {
                        width: parent.width
                        height: 180
                        radius: 12
                        color: "#0B1424"
                        border.color: NeonTheme.border
                        border.width: 1

                        Column {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 10

                            Row {
                                spacing: 10
                                NeonBadge { tone: NeonTheme.cyan; label: "WORLD" }
                                NeonText { text: "Top-level domain (Work, Personal, Research, Media)"; color: NeonTheme.text; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
                            }
                            Row {
                                spacing: 10
                                NeonBadge { tone: NeonTheme.lavender; label: "WORKSPACE" }
                                NeonText { text: "Project boundary (Git repo root, backend, frontend)"; color: NeonTheme.text; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
                            }
                            Row {
                                spacing: 10
                                NeonBadge { tone: NeonTheme.emerald; label: "LOGICAL DESKTOP" }
                                NeonText { text: "Task focus context (e.g. editor, terminal, browser)"; color: NeonTheme.text; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
                            }
                            Row {
                                spacing: 10
                                NeonBadge { tone: NeonTheme.precision; label: "OBSERVED WINDOWS" }
                                NeonText { text: "Read-only native KDE Wayland windows (DENY-ONLY native mutators)"; color: NeonTheme.muted; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
                            }
                        }
                    }

                    NeonText {
                        width: parent.width
                        wrapMode: Text.WordWrap
                        text: "Click 'Next' to customize your initial layout, or 'Quick Setup' to generate standard defaults immediately."
                        font.pixelSize: 13
                        color: NeonTheme.precision
                    }
                }

                // STEP 2: World
                Column {
                    visible: root.currentStep === 2
                    anchors.fill: parent
                    spacing: 12

                    NeonText {
                        text: "Define your first top-level World. You can create more Worlds anytime."
                        color: NeonTheme.precision
                        font.pixelSize: 13
                    }

                    CyberTextField {
                        width: parent.width
                        label: "WORLD NAME"
                        text: root.worldName
                        onTextModified: root.worldName = text
                    }

                    CyberTextField {
                        width: parent.width
                        label: "DESCRIPTION"
                        text: root.worldDescription
                        onTextModified: root.worldDescription = text
                    }

                    Column {
                        width: parent.width
                        spacing: 6
                        NeonLabel { text: "ARTWORK IDENTITY" }
                        Row {
                            spacing: 8
                            Repeater {
                                model: ["cyber-grid", "cyber-city", "neural-net", "deep-space"]
                                delegate: Rectangle {
                                    id: artPill
                                    required property var modelData
                                    width: 110
                                    height: 28
                                    radius: 6
                                    color: root.worldArtwork === modelData ? NeonTheme.tint(NeonTheme.cyan, 0.3) : NeonTheme.panel
                                    border.color: root.worldArtwork === modelData ? NeonTheme.cyan : NeonTheme.border
                                    border.width: 1

                                    NeonText {
                                        anchors.centerIn: parent
                                        text: artPill.modelData
                                        font.pixelSize: 11
                                        color: root.worldArtwork === artPill.modelData ? NeonTheme.cyan : NeonTheme.precision
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.worldArtwork = artPill.modelData
                                    }
                                }
                            }
                        }
                    }
                }

                // STEP 3: Workspace
                Column {
                    visible: root.currentStep === 3
                    anchors.fill: parent
                    spacing: 12

                    NeonText {
                        text: "Workspaces group related tasks and optionally map to a local project directory."
                        color: NeonTheme.precision
                        font.pixelSize: 13
                    }

                    CyberTextField {
                        width: parent.width
                        label: "WORKSPACE NAME"
                        text: root.workspaceName
                        onTextModified: root.workspaceName = text
                    }

                    CyberTextField {
                        width: parent.width
                        label: "DESCRIPTION"
                        text: root.workspaceDescription
                        onTextModified: root.workspaceDescription = text
                    }

                    ProjectRootPicker {
                        width: parent.width
                        hostBridge: root.hostBridge
                        path: root.projectRoot
                        onPathChanged: root.projectRoot = path
                    }
                }

                // STEP 4: Desktop
                Column {
                    visible: root.currentStep === 4
                    anchors.fill: parent
                    spacing: 12

                    NeonText {
                        text: "Logical Desktops partition observed windows within this Workspace."
                        color: NeonTheme.precision
                        font.pixelSize: 13
                    }

                    CyberTextField {
                        width: parent.width
                        label: "LOGICAL DESKTOP NAME"
                        text: root.desktopName
                        onTextModified: root.desktopName = text
                    }

                    CyberTextField {
                        width: parent.width
                        label: "ROLE (OPTIONAL)"
                        text: root.desktopRole
                        placeholderText: "development, browser, chat"
                        onTextModified: root.desktopRole = text
                    }
                }

                // STEP 5: Preferences
                Column {
                    visible: root.currentStep === 5
                    anchors.fill: parent
                    spacing: 12

                    NeonText {
                        text: "Configure your interface rendering and privacy preferences."
                        color: NeonTheme.precision
                        font.pixelSize: 13
                    }

                    NeonButton {
                        width: parent.width
                        checkable: true
                        checked: root.prefReducedMotion
                        highlighted: checked
                        text: "Reduced Motion · " + (checked ? "Enabled (Instant animations)" : "Disabled")
                        onClicked: root.prefReducedMotion = checked
                    }

                    NeonButton {
                        width: parent.width
                        checkable: true
                        checked: root.prefLowEffects
                        highlighted: checked
                        text: "Low Effects · " + (checked ? "Enabled (Optimized for low-end GPUs)" : "Disabled")
                        onClicked: root.prefLowEffects = checked
                    }

                    NeonButton {
                        width: parent.width
                        checkable: true
                        checked: root.prefHighContrast
                        highlighted: checked
                        text: "High Contrast · " + (checked ? "Enabled" : "Disabled")
                        onClicked: root.prefHighContrast = checked
                    }

                    NeonButton {
                        width: parent.width
                        checkable: true
                        checked: root.prefShowTitles
                        highlighted: checked
                        text: "Window Titles · " + (checked ? "Revealed" : "Redacted (Privacy Safe)")
                        onClicked: root.prefShowTitles = checked
                    }
                }

                // STEP 6: Confirmation
                Column {
                    visible: root.currentStep === 6
                    anchors.fill: parent
                    spacing: 14

                    NeonText {
                        text: "Ready to initialize your personal DepthOS workspace:"
                        color: NeonTheme.text
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                    }

                    Rectangle {
                        width: parent.width
                        height: 180
                        radius: 12
                        color: "#0B1424"
                        border.color: NeonTheme.border
                        border.width: 1

                        Column {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 8

                            NeonText { text: "World: " + root.worldName + " (" + root.worldArtwork + ")"; color: NeonTheme.cyan; font.weight: Font.DemiBold }
                            NeonText { text: "Workspace: " + root.workspaceName + (root.projectRoot ? " @ " + root.projectRoot : ""); color: NeonTheme.lavender; font.weight: Font.DemiBold }
                            NeonText { text: "Desktop: " + root.desktopName + " (" + root.desktopRole + ")"; color: NeonTheme.emerald; font.weight: Font.DemiBold }
                            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }
                            NeonText {
                                text: "Preferences: " + (root.prefReducedMotion ? "Reduced Motion · " : "") + (root.prefHighContrast ? "High Contrast · " : "") + (root.prefShowTitles ? "Titles Shown" : "Titles Redacted")
                                color: NeonTheme.precision
                                font.pixelSize: 12
                            }
                        }
                    }

                    NeonText {
                        text: "Click 'Initialize & Launch' to commit this hierarchy to SQLite."
                        color: NeonTheme.precision
                        font.pixelSize: 13
                    }
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            // Navigation footer
            Item {
                width: parent.width
                height: 36

                NeonButton {
                    visible: root.currentStep > 1
                    text: "← Back"
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: root.currentStep -= 1
                }

                NeonButton {
                    id: nextBtn
                    objectName: root.currentStep === root.totalSteps ? "finishWizardButton" : "nextWizardButton"
                    text: root.currentStep === root.totalSteps ? "Initialize & Launch  ↵" : "Next  →"
                    highlighted: true
                    accent: root.currentStep === root.totalSteps ? NeonTheme.emerald : NeonTheme.cyan
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: {
                        if (root.currentStep < root.totalSteps) {
                            root.currentStep += 1;
                        } else {
                            root.finishWizard();
                        }
                    }
                }
            }
        }
    }
}
