import QtQuick
import "theme"

Item {
    id: layer
    property bool activeLevel: false
    property bool motionOff: false
    property int direction: 1

    visible: activeLevel || opacity > 0
    enabled: activeLevel
    opacity: activeLevel ? 1 : 0
    scale: 1

    // Restart only this layer's entrance. Interaction and hierarchy state are
    // already final when the animation starts, including interrupted entrances.
    onActiveLevelChanged: {
        entrance.stop();
        layer.scale = 1;
        if (activeLevel && !motionOff)
            entrance.restart();
    }

    onMotionOffChanged: {
        if (motionOff) {
            entrance.stop();
            scale = 1;
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: NeonTheme.ms(layer.motionOff, NeonTheme.zoomMs)
            easing.type: Easing.OutQuad
        }
    }

    NumberAnimation {
        id: entrance
        target: layer
        property: "scale"
        from: layer.direction > 0 ? 0.91 : 1.065
        to: 1
        duration: NeonTheme.zoomMs
        easing.type: Easing.OutQuad
    }
}
