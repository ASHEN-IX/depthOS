"""Owned Wayland fixture surface. It never requests focus or placement."""
import signal
import sys

from PyQt6.QtWidgets import QApplication, QLabel

if sys.stdin.readline().strip() != "show":
    raise SystemExit("Missing private fixture readiness signal")

app = QApplication(sys.argv)
window = QLabel("DepthOS private authority fixture")
window.resize(360, 140)
signal.signal(signal.SIGTERM, lambda *_: app.quit())
window.show()
raise SystemExit(app.exec())
