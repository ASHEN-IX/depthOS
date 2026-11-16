"""An owned Wayland fixture with a one-way, display-only preview input."""
import json
import os
from pathlib import Path
import signal
import sys

from PyQt6.QtCore import QObject, QSocketNotifier, QUrl
from PyQt6.QtGui import QGuiApplication
from PyQt6.QtQuick import QQuickView


def report(**values):
    print(json.dumps(values), flush=True)


if sys.stdin.readline().strip() != "show":
    raise SystemExit("Missing private fixture readiness signal")

app = QGuiApplication(sys.argv)
view = QQuickView()
view.setSource(QUrl.fromLocalFile(str(Path(__file__).with_name("Fixture.qml"))))
root = view.rootObject()
if root is None:
    raise SystemExit("Missing private fixture QML root")


def cancelled():
    report(event="cancelled")
    app.quit()


def rendered():
    labels = [root.findChild(QObject, name) for name in ("source", "destination", "activity")]
    if all(label is not None for label in labels):
        view.frameSwapped.disconnect(rendered)
        report(event="presented", labels=[label.property("text") for label in labels])


def handle(command):
    if "preview" in command:
        view.frameSwapped.connect(rendered)
        root.showPreview(command["preview"])
    elif command == {"dismiss": True}:
        root.dismissPreview()
    elif command == {"invalidate": True}:
        root.invalidatePreview()
        report(event="hidden", empty=not root.property("previewVisible"))
    else:
        raise ValueError("Unknown private fixture command")


buffer = bytearray()


def read():
    chunk = os.read(sys.stdin.fileno(), 4096)
    if not chunk:
        app.quit()
        return
    buffer.extend(chunk)
    while b"\n" in buffer:
        line, _, rest = buffer.partition(b"\n")
        buffer[:] = rest
        handle(json.loads(line))


notifier = QSocketNotifier(sys.stdin.fileno(), QSocketNotifier.Type.Read)
notifier.activated.connect(read)
root.cancelled.connect(cancelled)
view.show()
signal.signal(signal.SIGTERM, lambda *_: app.quit())
raise SystemExit(app.exec())
