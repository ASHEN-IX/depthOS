pragma ComponentBehavior: Bound
import QtQuick

FocusScope {
    id: searchScope

    property var backend: null
    property bool isOpen: false
    property bool motionOff: false

    signal navigate(var item)
    signal closed

    visible: opacity > 0
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation {
            duration: searchScope.motionOff ? 0 : 180
            easing.type: Easing.OutCubic
        }
    }

    onIsOpenChanged: {
        if (isOpen) {
            searchInput.text = "";
            selectedIndex = 0;
            searchInput.forceActiveFocus();
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

    property var searchResults: (backend && isOpen) ? backend.search(searchInput.text) : []
    property int selectedIndex: 0

    // Backdrop click dismisses
    Rectangle {
        anchors.fill: parent
        color: "#00000088"
        MouseArea {
            anchors.fill: parent
            onClicked: searchScope.dismiss()
        }
    }

    // Modal Box centered near top
    Rectangle {
        id: modalBox
        width: Math.min(680, searchScope.width - 48)
        height: Math.min(480, 72 + resultsList.contentHeight)
        anchors.horizontalCenter: parent.horizontalCenter
        y: searchScope.isOpen ? 80 : 50
        radius: 16
        color: "#0F172A"
        border.color: "#38BDF8"
        border.width: 1.5
        clip: true

        Behavior on y {
            NumberAnimation {
                duration: searchScope.motionOff ? 0 : 200
                easing.type: Easing.OutCubic
            }
        }

        Column {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 10

            // Search Input Row
            Row {
                width: parent.width
                height: 40
                spacing: 12

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "🔍"
                    font.pixelSize: 18
                }

                TextInput {
                    id: searchInput
                    objectName: "searchInput"
                    width: parent.width - 120
                    height: parent.height
                    verticalAlignment: TextInput.AlignVCenter
                    color: "#F8FAFC"
                    font.pixelSize: 15
                    focus: searchScope.isOpen
                    clip: true
                    selectByMouse: true

                    Text {
                        anchors.fill: parent
                        verticalAlignment: Text.AlignVCenter
                        text: "Search Worlds, Workspaces, and Desktops... (type to filter)"
                        color: "#64748B"
                        font.pixelSize: 14
                        visible: !searchInput.text && !searchInput.inputMethodComposing
                    }

                    onTextChanged: {
                        searchScope.selectedIndex = 0;
                    }

                    Keys.onPressed: event => {
                        if (event.key === Qt.Key_Escape) {
                            searchScope.dismiss();
                            event.accepted = true;
                        } else if (event.key === Qt.Key_Down || event.key === Qt.Key_Tab) {
                            if (searchScope.searchResults.length > 0) {
                                searchScope.selectedIndex = (searchScope.selectedIndex + 1) % searchScope.searchResults.length;
                            }
                            event.accepted = true;
                        } else if (event.key === Qt.Key_Up || event.key === Qt.Key_Backtab) {
                            if (searchScope.searchResults.length > 0) {
                                searchScope.selectedIndex = (searchScope.selectedIndex - 1 + searchScope.searchResults.length) % searchScope.searchResults.length;
                            }
                            event.accepted = true;
                        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                            if (searchScope.searchResults.length > 0 && searchScope.selectedIndex >= 0 && searchScope.selectedIndex < searchScope.searchResults.length) {
                                searchScope.navigate(searchScope.searchResults[searchScope.selectedIndex]);
                            }
                            event.accepted = true;
                        }
                    }
                }

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    height: 22
                    width: escBadge.implicitWidth + 12
                    radius: 4
                    color: "#1E293B"
                    border.color: "#334155"
                    border.width: 1

                    Text {
                        id: escBadge
                        anchors.centerIn: parent
                        textFormat: Text.PlainText
                        text: "Esc to close"
                        color: "#94A3B8"
                        font.pixelSize: 10
                    }
                }
            }

            Rectangle {
                width: parent.width
                height: 1
                color: "#1E293B"
            }

            // Results List
            ListView {
                id: resultsList
                width: parent.width
                height: Math.min(380, count * 52)
                clip: true
                model: searchScope.searchResults
                currentIndex: searchScope.selectedIndex

                delegate: Rectangle {
                    id: resultDelegate
                    required property var modelData
                    required property int index

                    width: resultsList.width
                    height: 48
                    radius: 8
                    color: searchScope.selectedIndex === resultDelegate.index ? "#1E293B" : "transparent"
                    border.color: searchScope.selectedIndex === resultDelegate.index ? "#38BDF8" : "transparent"
                    border.width: 1

                    Row {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 12

                        // Kind Badge
                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            height: 22
                            width: kindLabel.implicitWidth + 12
                            radius: 4
                            color: {
                                switch (resultDelegate.modelData.kind) {
                                case "world":
                                    return "#312E81";
                                case "workspace":
                                    return "#064E3B";
                                case "desktop":
                                    return "#1E3A8A";
                                default:
                                    return "#334155";
                                }
                            }

                            Text {
                                id: kindLabel
                                anchors.centerIn: parent
                                textFormat: Text.PlainText
                                text: resultDelegate.modelData.kind.toUpperCase()
                                color: {
                                    switch (resultDelegate.modelData.kind) {
                                    case "world":
                                        return "#C7D2FE";
                                    case "workspace":
                                        return "#A7F3D0";
                                    case "desktop":
                                        return "#BFDBFE";
                                    default:
                                        return "#E2E8F0";
                                    }
                                }
                                font.pixelSize: 9
                                font.weight: Font.Bold
                            }
                        }

                        // Title & Detail
                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            width: parent.width - 120
                            spacing: 2

                            Text {
                                textFormat: Text.PlainText
                                text: resultDelegate.modelData.name
                                color: "#F8FAFC"
                                font.pixelSize: 13
                                font.weight: Font.Medium
                                elide: Text.ElideRight
                                width: parent.width
                            }

                            Text {
                                textFormat: Text.PlainText
                                text: resultDelegate.modelData.detail
                                color: "#94A3B8"
                                font.pixelSize: 11
                                elide: Text.ElideRight
                                width: parent.width
                            }
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            textFormat: Text.PlainText
                            text: searchScope.selectedIndex === resultDelegate.index ? "Jump ↵" : ""
                            color: "#38BDF8"
                            font.pixelSize: 11
                            font.weight: Font.DemiBold
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: searchScope.selectedIndex = resultDelegate.index
                        onClicked: searchScope.navigate(resultDelegate.modelData)
                    }
                }

                // Empty result text
                Text {
                    anchors.centerIn: parent
                    visible: searchScope.searchResults.length === 0 && searchInput.text.trim() !== ""
                    textFormat: Text.PlainText
                    text: "No matching worlds, workspaces, or desktops found"
                    color: "#64748B"
                    font.pixelSize: 13
                }
            }
        }
    }
}
