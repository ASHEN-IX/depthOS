import QtQuick
import "theme"

Rectangle {
    id: sky
    property bool lowEffects: false
    property bool highContrast: false
    property int level: 1
    property real drift: 0

    color: highContrast ? NeonTheme.contrastPanel : NeonTheme.background

    Canvas {
        visible: !sky.lowEffects && !sky.highContrast
        anchors.fill: parent
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onVisibleChanged: if (visible) requestPaint()

        onPaint: {
            const ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            function light(x, y, extent, tint) {
                const g = ctx.createRadialGradient(x, y, 0, x, y, extent);
                g.addColorStop(0, tint);
                g.addColorStop(1, NeonTheme.transparent);
                ctx.fillStyle = g;
                ctx.fillRect(0, 0, width, height);
            }
            light(width * 0.16, height * 0.5, width * 0.47, NeonTheme.violetHaze);
            light(width * 0.92, height * 0.47, width * 0.42, NeonTheme.cyanHaze);
            light(width * 0.53, height * 0.64, width * 0.34, NeonTheme.emeraldHaze);

            const horizon = height * 0.57;
            ctx.lineWidth = 1;
            ctx.strokeStyle = NeonTheme.grid;
            ctx.beginPath();
            for (let col = -13; col <= 13; ++col) {
                ctx.moveTo(width / 2 + col * width * 0.0105, horizon);
                ctx.lineTo(width / 2 + col * width * 0.11, height);
            }
            for (let row = 1; row <= 13; ++row) {
                const y = horizon + (height - horizon) * Math.pow(row / 13, 2.05);
                ctx.moveTo(0, y);
                ctx.lineTo(width, y);
            }
            ctx.stroke();

            const line = ctx.createLinearGradient(0, 0, width, 0);
            line.addColorStop(0, NeonTheme.transparent);
            line.addColorStop(0.5, NeonTheme.gridHorizon);
            line.addColorStop(1, NeonTheme.transparent);
            ctx.strokeStyle = line;
            ctx.beginPath();
            ctx.moveTo(0, horizon);
            ctx.lineTo(width, horizon);
            ctx.stroke();

            ctx.fillStyle = NeonTheme.gridHorizon;
            for (let i = 0; i < 38; ++i) {
                const x = ((i * 1297 + 101) % 991) * width / 991;
                const y = ((i * 937 + 211) % 571) * horizon / 571;
                ctx.fillRect(x, y, i % 5 === 0 ? 2 : 1, 1);
            }
        }
    }

    Rectangle {
        visible: !sky.lowEffects && !sky.highContrast
        width: Math.min(sky.width / 3, 420)
        height: 1
        x: (sky.width - width) / 2 + sky.drift
        y: sky.height * (0.57 + (sky.level - 1) * 0.04)
        opacity: 0.38
        color: NeonTheme.emerald
    }
}
