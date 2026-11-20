pragma ComponentBehavior: Bound
import QtQuick
import "theme"

Item {
    id: card

    property bool isSelected: false
    property bool isFocused: false
    property bool motionOff: false
    property bool strongGlow: false
    property bool lowEffects: false
    property bool highContrast: false
    property bool available: true
    property color accent: NeonTheme.emerald
    property string accessibleLabel: ""
    property string accessibleDescription: ""
    property Item openTarget: null
    property bool pointerEnabled: true
    property int radius: NeonTheme.cardRadius
    default property alias content: contentLayer.data

    signal chosen
    signal activated

    Accessible.role: Accessible.ListItem
    Accessible.name: accessibleLabel
    Accessible.description: accessibleDescription
    Accessible.selectable: available
    Accessible.selected: isSelected
    Accessible.onPressAction: if (available && pointerEnabled) chosen()

    Rectangle {
        anchors.fill: parent
        anchors.margins: -6
        radius: card.radius + 6
        color: NeonTheme.transparent
        border.color: card.accent
        border.width: 3
        opacity: card.isFocused && !card.strongGlow && !card.lowEffects && !card.highContrast ? NeonTheme.glowOpacity : 0
    }

    Loader {
        anchors.fill: parent
        active: card.strongGlow && card.isSelected && !card.lowEffects && !card.highContrast
        sourceComponent: NeonRim {
            radius: card.radius
            tone: card.accent
            bright: true
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: card.radius
        color: card.highContrast ? NeonTheme.contrastSurface : card.isSelected
            ? NeonTheme.selected : pointer.containsMouse ? NeonTheme.hover : NeonTheme.surface
        border.width: card.isFocused || card.isSelected ? 2 : 1
        border.color: card.isFocused && card.highContrast ? NeonTheme.focus
            : card.isSelected || card.isFocused ? card.accent
            : card.highContrast ? NeonTheme.muted : NeonTheme.border
    }

    Item {
        id: contentLayer
        anchors.fill: parent
        clip: true
        opacity: card.available ? 1 : 0.62
    }

    Rectangle {
        visible: card.isFocused
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: 4
        height: 30
        radius: 2
        color: card.highContrast ? NeonTheme.focus : card.accent
    }

    MouseArea {
        id: pointer
        anchors.fill: parent
        enabled: card.pointerEnabled
        hoverEnabled: card.available
        cursorShape: card.available ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: mouse => {
            if (!card.available)
                return;
            const target = card.openTarget;
            if (target && target.visible && target.contains(target.mapFromItem(pointer, mouse.x, mouse.y)))
                card.activated();
            else
                card.chosen();
        }
        onDoubleClicked: if (card.available) card.activated()
    }
}
