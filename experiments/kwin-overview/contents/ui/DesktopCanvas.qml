import QtQuick
import org.kde.kwin
import "Policy.js" as Policy

// Public native surfaces in their output-relative positions. Desktop/dock
// surfaces are included only when exposed and permitted; nothing is synthesized.
Item {
    id: canvas
    required property var system
    required property var source
    required property var output
    property bool previewsEnabled: true
    readonly property var outputGeometry: Policy.read(output, "geometry")
    WindowFilterModel {
        id: activeWindows
        windowModel: canvas.source
        desktop: Policy.currentDesktopForOutput(canvas.system, canvas.output)
        activity: canvas.system ? canvas.system.currentActivity : ""
        screenName: Policy.read(canvas.output, "name") || ""
        windowType: WindowFilterModel.Normal | WindowFilterModel.Dialog | WindowFilterModel.Desktop | WindowFilterModel.Dock
    }
    Repeater {
        model: canvas.system && canvas.source && canvas.output ? activeWindows : null
        delegate: Loader {
            id: surface
            required property var window
            required property int index
            active: Policy.previewReason(window, canvas.system, canvas.previewsEnabled, window === (canvas.system ? canvas.system.activeWindow : null) ? 0 : index + 1, 6) === ""
            readonly property var frame: Policy.read(window, "frameGeometry")
            x: frame && canvas.outputGeometry ? frame.x - canvas.outputGeometry.x : 0
            y: frame && canvas.outputGeometry ? frame.y - canvas.outputGeometry.y : 0
            width: frame ? Math.max(1, frame.width) : 1
            height: frame ? Math.max(1, frame.height) : 1
            z: Policy.read(window, "stackingOrder") || 0
            sourceComponent: WindowThumbnail { client: surface.window }
        }
    }
}
