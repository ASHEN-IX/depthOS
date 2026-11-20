import QtQuick
import "theme"

Canvas {
    id: mark
    property color ink: NeonTheme.cyan
    property int variant: 0
    property bool simple: false
    onInkChanged: requestPaint()
    onVariantChanged: requestPaint()
    onSimpleChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onVisibleChanged: if (visible) requestPaint()

    onPaint: {
        const ctx = getContext("2d");
        ctx.reset();
        ctx.clearRect(0, 0, width, height);
        if (width < 1 || height < 1)
            return;
        const unit = Math.min(width, height) / 200;
        ctx.translate(width / 2, height / 2);
        ctx.scale(unit, unit);
        function polygon(radius, sides, angle, yScale) {
            ctx.beginPath();
            for (let i = 0; i < sides; ++i) {
                const a = angle + i * Math.PI * 2 / sides;
                const x = Math.cos(a) * radius;
                const y = Math.sin(a) * radius * yScale;
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }
            ctx.closePath();
        }

        if (!simple) {
            ctx.strokeStyle = NeonTheme.tint(ink, 0.27);
            ctx.lineWidth = 1;
            ctx.setLineDash([3, 7]);
            for (let r = 56; r <= 92; r += 18) {
                ctx.beginPath();
                ctx.arc(0, 0, r, 0, 2 * Math.PI);
                ctx.stroke();
            }
            ctx.setLineDash([]);
            ctx.beginPath();
            ctx.moveTo(-132, 0);
            ctx.lineTo(132, 0);
            ctx.moveTo(0, -95);
            ctx.lineTo(0, 95);
            ctx.stroke();
        }

        const sides = [6, 4, 3, 8][variant % 4];
        const angle = [-Math.PI / 6, 0, -Math.PI / 2, Math.PI / 8][variant % 4];
        ctx.lineWidth = 1.6;
        ctx.strokeStyle = ink;
        ctx.fillStyle = NeonTheme.tint(ink, 0.13);
        polygon(49, sides, angle, 0.87);
        ctx.fill();
        ctx.stroke();
        ctx.strokeStyle = NeonTheme.tint(ink, 0.53);
        polygon(35, sides, angle + Math.PI / sides, 0.87);
        ctx.stroke();
        ctx.fillStyle = ink;
        polygon(14, sides, angle, 0.87);
        ctx.fill();
        if (!simple) {
            for (let n = 0; n < 5; ++n) {
                const a = angle + n * Math.PI * 2 / 5;
                ctx.fillRect(Math.cos(a) * 83 - 1.5, Math.sin(a) * 83 - 1.5, 3, 3);
            }
        }
    }
}
