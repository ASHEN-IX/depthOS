pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

FocusScope {
    id: palette

    property var backend: null
    property bool isOpen: false
    property bool motionOff: false
    property bool highContrast: false

    signal navigate(var item)
    signal closed

    visible: isOpen || opacity > 0
    enabled: isOpen
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(palette.motionOff, NeonTheme.modalMs) }
    }

    onIsOpenChanged: {
        if (isOpen) {
            searchInput.clear();
            selectedIndex = 0;
            focusInput();
        }
    }

    function dismiss() {
        if (!isOpen)
            return;
        isOpen = false;
        closed();
    }

    function focusInput() {
        searchInput.forceActiveFocus();
    }

    function moveSelection(direction) {
        if (searchResults.length)
            selectedIndex = (selectedIndex + direction + searchResults.length) % searchResults.length;
    }

    function activateSelection() {
        if (selectedIndex >= 0 && selectedIndex < searchResults.length)
            navigate(searchResults[selectedIndex]);
    }

    function selectTone(kind) {
        return kind === "world" ? NeonTheme.lavender :
            kind === "workspace" ? NeonTheme.emerald : NeonTheme.cyan;
    }

    // The markup is fixed. Escape hierarchy names before adding a match span.
    function highlightName(name) {
        const value = Display.safeText(name);
        const query = searchInput.text.trim().replace(/^[@#!]/, "").trim();
        const index = query.length ? value.toLowerCase().indexOf(query.toLowerCase()) : -1;
        const escape = text => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
        if (index < 0)
            return escape(value);
        const end = index + query.length;
        return escape(value.slice(0, index)) + "<font color='" + NeonTheme.cyan + "'>"
            + escape(value.slice(index, end)) + "</font>" + escape(value.slice(end));
    }

    property var searchResults: backend && isOpen ? backend.search(searchInput.text) : []
    property int selectedIndex: 0
    onSearchResultsChanged: selectedIndex = Math.max(0, Math.min(selectedIndex, searchResults.length - 1))

    Rectangle {
        anchors.fill: parent
        color: NeonTheme.scrim
        MouseArea {
            anchors.fill: parent
            onClicked: palette.dismiss()
        }
    }

    Rectangle {
        id: panel
        anchors.centerIn: parent
        width: Math.min(670, parent.width - 32)
        height: Math.min(446, parent.height - 44)
        radius: NeonTheme.cardRadius
        color: palette.highContrast ? NeonTheme.contrastPanel : NeonTheme.panel
        border.color: NeonTheme.cyan
        border.width: 1
        clip: true

        MouseArea { anchors.fill: parent }

        Column {
            anchors.fill: parent
            anchors.margins: 17
            spacing: 11

            Item {
                width: parent.width
                height: 30
                NeonLabel {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    text: "NAVIGATE HIERARCHY"
                    color: NeonTheme.cyan
                }
                NeonButton {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    compact: true
                    text: "Close · Esc"
                    onClicked: palette.dismiss()
                    Keys.onEscapePressed: palette.dismiss()
                }
            }

            Rectangle {
                width: parent.width
                height: 48
                radius: NeonTheme.buttonRadius
                color: NeonTheme.background
                border.width: searchInput.activeFocus ? 2 : 1
                border.color: searchInput.activeFocus ? NeonTheme.cyan : NeonTheme.border

                NeonText {
                    id: searchGlyph
                    x: 14
                    anchors.verticalCenter: parent.verticalCenter
                    text: "⌕"
                    color: NeonTheme.cyan
                    font.pixelSize: 21
                }

                TextInput {
                    id: searchInput
                    objectName: "searchInput"
                    anchors.left: searchGlyph.right
                    anchors.leftMargin: 10
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    height: parent.height
                    verticalAlignment: TextInput.AlignVCenter
                    color: NeonTheme.text
                    selectionColor: NeonTheme.selected
                    selectedTextColor: NeonTheme.text
                    font.pixelSize: NeonTheme.bodySize
                    focus: palette.isOpen
                    clip: true
                    selectByMouse: true
                    Accessible.name: "Search worlds, workspaces and desktops"

                    NeonText {
                        anchors.fill: parent
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                        text: "Search worlds, workspaces, and desktops"
                        color: NeonTheme.muted
                        visible: !searchInput.text && !searchInput.inputMethodComposing
                    }

                    onTextChanged: palette.selectedIndex = 0
                    Keys.onPressed: event => {
                        switch (event.key) {
                        case Qt.Key_Escape: palette.dismiss(); break;
                        case Qt.Key_Down:
                        case Qt.Key_Tab: palette.moveSelection(1); break;
                        case Qt.Key_Up:
                        case Qt.Key_Backtab: palette.moveSelection(-1); break;
                        case Qt.Key_Return:
                        case Qt.Key_Enter: palette.activateSelection(); break;
                        default: return;
                        }
                        event.accepted = true;
                    }
                }
            }

            ListView {
                id: results
                objectName: "searchResults"
                width: parent.width
                height: Math.max(24, panel.height - 156)
                spacing: 3
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: palette.searchResults
                currentIndex: palette.selectedIndex

                onCurrentIndexChanged: if (currentIndex >= 0) positionViewAtIndex(currentIndex, ListView.Contain)

                delegate: Rectangle {
                    id: result
                    required property var modelData
                    required property int index
                    readonly property bool selected: palette.selectedIndex === index
                    width: results.width
                    height: 53
                    radius: NeonTheme.buttonRadius
                    color: selected ? NeonTheme.selected : NeonTheme.transparent
                    border.color: selected ? (palette.highContrast ? NeonTheme.focus : NeonTheme.cyan) : NeonTheme.transparent
                    border.width: selected ? 2 : 0
                    Accessible.role: Accessible.ListItem
                    Accessible.name: Display.safeText(modelData.name)
                    Accessible.description: Display.safeText(modelData.breadcrumb || modelData.detail)
                    Accessible.selected: selected
                    Accessible.onPressAction: palette.navigate(modelData)

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 9

                        NeonBadge {
                            id: category
                            anchors.verticalCenter: parent.verticalCenter
                            width: 92
                            label: result.modelData.kind.toUpperCase()
                            tone: palette.selectTone(result.modelData.kind)
                        }

                        Column {
                            width: Math.max(0, parent.width - category.width - jump.width - 18)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 3

                            NeonText {
                                width: parent.width
                                textFormat: Text.StyledText
                                text: palette.highlightName(result.modelData.name)
                                elide: Text.ElideRight
                                font.weight: Font.DemiBold
                            }

                            NeonText {
                                width: parent.width
                                text: Display.safeText(result.modelData.breadcrumb || result.modelData.detail)
                                elide: Text.ElideRight
                                color: NeonTheme.muted
                                font.pixelSize: NeonTheme.detailSize
                            }
                        }

                        NeonText {
                            id: jump
                            width: 15
                            anchors.verticalCenter: parent.verticalCenter
                            text: result.selected ? "↵" : ""
                            color: NeonTheme.emerald
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onEntered: palette.selectedIndex = result.index
                        onClicked: palette.navigate(result.modelData)
                    }
                }

                NeonText {
                    anchors.centerIn: parent
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                    visible: palette.searchResults.length === 0
                    text: searchInput.text.trim() ? "No matching items" : "Type to find an item in your hierarchy."
                    color: NeonTheme.muted
                }
            }

            NeonText {
                width: parent.width
                text: "↑ / ↓   Select    ·    ↵   Open    ·    Esc   Close"
                color: NeonTheme.precision
                font.pixelSize: NeonTheme.detailSize
                elide: Text.ElideRight
            }
        }
    }
}
