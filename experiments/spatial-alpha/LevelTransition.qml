import QtQuick
import "theme"

Item {
    id: transitionLayer
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
        transitionLayer.scale = 1;
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
            duration: NeonTheme.ms(transitionLayer.motionOff, NeonTheme.zoomMs)
            easing.type: Easing.OutQuad
        }
    }

    NumberAnimation {
        id: entrance
        target: transitionLayer
        property: "scale"
        from: transitionLayer.direction > 0 ? 0.91 : 1.065
        to: 1
        duration: NeonTheme.zoomMs
        easing.type: Easing.OutQuad
    }
}
