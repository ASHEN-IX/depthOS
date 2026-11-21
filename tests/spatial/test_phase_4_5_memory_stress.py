#!/usr/bin/env python3
"""Phase 4.5 Memory Stress and Leak Qualification Test.

Runs a 100-cycle intensive spatial interaction workload:
- World carousel navigation (moveHorizontal, selectWorldFromPointer)
- Level transitions (enterDeeper, exitLevel)
- Workspace selection and scrolling
- Desktop switching and tab selection
- Search palette open, query typing, dismissal
- Transactional management CRUD (create, archive, restore)

Records RSS, PSS, and Private Dirty memory every 25 cycles and asserts
that memory stabilizes without unbounded growth (< 15 MiB growth from cycle 25 to 100).
Outputs results to artifacts/phase-4-5/memory_profile.json.
"""
import argparse
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/support"))
from fixture_generator import generate_fixture

LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
ARTIFACTS = ROOT / "artifacts/phase-4-5"


def read_memory_stats(pid=None):
    pid_str = str(pid) if pid else "self"
    stats = {"rssKiB": 0, "pssKiB": 0, "privateDirtyKiB": 0, "vmHwmKiB": 0}
    try:
        with open(f"/proc/{pid_str}/smaps_rollup", "r") as f:
            for line in f:
                parts = line.split()
                if not parts:
                    continue
                if parts[0] == "Rss:":
                    stats["rssKiB"] = int(parts[1])
                elif parts[0] == "Pss:":
                    stats["pssKiB"] = int(parts[1])
                elif parts[0] == "Private_Dirty:":
                    stats["privateDirtyKiB"] = int(parts[1])
    except Exception:
        pass

    try:
        with open(f"/proc/{pid_str}/status", "r") as f:
            for line in f:
                parts = line.split()
                if not parts:
                    continue
                if parts[0] == "VmHWM:":
                    stats["vmHwmKiB"] = int(parts[1])
                elif parts[0] == "VmRSS:" and not stats["rssKiB"]:
                    stats["rssKiB"] = int(parts[1])
    except Exception:
        pass
    return stats


def run_memory_stress(total_cycles=100):
    ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o755)
    temp_dir = Path(tempfile.mkdtemp(prefix="depthos-memstress-"))
    os.chmod(temp_dir, 0o700)
    db_path = temp_dir / "medium.sqlite"

    try:
        generate_fixture(db_path, "medium")
        print(f"Generated medium fixture at {db_path}", flush=True)

        from PyQt6 import QtQml
        from PyQt6.QtCore import QTimer
        from PyQt6.QtGui import QGuiApplication
        from PyQt6.QtQuick import QQuickWindow

        loader = importlib.machinery.SourceFileLoader("spatial_alpha_module", str(LAUNCHER))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        launcher = importlib.util.module_from_spec(spec)
        loader.exec_module(launcher)

        snapshots = []
        cycle_count = 0
        initial_mem = read_memory_stats()
        snapshots.append({"cycle": 0, "stage": "initial", **initial_mem})
        print(f"[Cycle 0] Initial RSS: {initial_mem['rssKiB']} KiB, PSS: {initial_mem['pssKiB']} KiB", flush=True)

        class StressEngine(QtQml.QQmlApplicationEngine):
            def load(self, url):
                super().load(url)
                window = self.rootObjects()[0]
                assert isinstance(window, QQuickWindow)
                scene = window.property("scene")
                backend = window.property("backend")
                bridge = window.property("hostBridge")

                def run_cycle():
                    nonlocal cycle_count
                    if cycle_count >= total_cycles:
                        finish()
                        return

                    cycle_count += 1
                    # 1. World navigation
                    scene.moveHorizontal(1 if cycle_count % 2 == 0 else -1)
                    worlds = backend.property("worlds") or []
                    if worlds:
                        target_w = cycle_count % len(worlds)
                        scene.selectWorldFromPointer(target_w)

                    # 2. Level transition into Workspaces
                    scene.enterDeeper()
                    workspaces = backend.property("workspaces") or []
                    if workspaces:
                        ws_idx = cycle_count % len(workspaces)
                        backend.selectWorkspace(workspaces[ws_idx]["id"])

                    # 3. Level transition into Desktops
                    scene.enterDeeper()
                    desktops = backend.property("desktops") or []
                    if desktops:
                        dt_idx = cycle_count % len(desktops)
                        scene.selectDesktopIndex(dt_idx)

                    # 4. Search interaction
                    search_item = scene.findNamed("searchPalette")
                    if search_item:
                        search_item.setProperty("isOpen", True)
                        input_field = scene.findNamed("searchInput")
                        if input_field:
                            input_field.setProperty("text", f"test-query-{cycle_count}")
                        search_item.dismiss()

                    # 5. Return to top level
                    scene.setLevel(1)

                    # 6. Periodic management CRUD mutation (every 10 cycles)
                    if cycle_count % 10 == 0 and bridge:
                        current_ws = backend.property("selectedWorkspaceId") or (workspaces[0]["id"] if workspaces else "")
                        if current_ws:
                            c_req = json.dumps({
                                "action": "create",
                                "entity": "desktop",
                                "parentId": current_ws,
                                "payload": {
                                    "name": f"Stress Desktop {cycle_count}",
                                    "role": "testing",
                                    "visualIdentity": "neon-cyan"
                                }
                            })
                            res_raw = bridge.executeManagement(c_req)
                            res = json.loads(res_raw)
                            if res.get("success"):
                                dt_id = res.get("data", {}).get("id")
                                a_req = json.dumps({
                                    "action": "archive",
                                    "entity": "desktop",
                                    "id": dt_id,
                                    "expectedVersion": 1
                                })
                                bridge.executeManagement(a_req)
                                r_req = json.dumps({
                                    "action": "restore",
                                    "entity": "desktop",
                                    "id": dt_id,
                                    "expectedVersion": 2
                                })
                                bridge.executeManagement(r_req)

                    # 7. Record checkpoint memory
                    if cycle_count in (1, 10, 25, 50, 75, 100):
                        mem = read_memory_stats()
                        snapshots.append({"cycle": cycle_count, "stage": f"cycle_{cycle_count}", **mem})
                        print(f"[Cycle {cycle_count:3d}] RSS: {mem['rssKiB']:6d} KiB | PSS: {mem['pssKiB']:6d} KiB | Dirty: {mem['privateDirtyKiB']:6d} KiB", flush=True)

                    QTimer.singleShot(5, run_cycle)

                def finish():
                    final_mem = read_memory_stats()
                    mem_25 = next((s for s in snapshots if s["cycle"] == 25), snapshots[0])
                    mem_100 = next((s for s in snapshots if s["cycle"] == 100), final_mem)

                    delta_rss_kib = mem_100["rssKiB"] - mem_25["rssKiB"]
                    delta_pss_kib = mem_100["pssKiB"] - mem_25["pssKiB"]

                    report = {
                        "totalCycles": total_cycles,
                        "initialMemory": initial_mem,
                        "finalMemory": final_mem,
                        "snapshots": snapshots,
                        "growthFromCycle25To100KiB": {
                            "rss": delta_rss_kib,
                            "pss": delta_pss_kib
                        },
                        "stabilized": delta_rss_kib < 15 * 1024,
                        "verdict": "PASS" if delta_rss_kib < 15 * 1024 else "FAIL"
                    }

                    out_path = ARTIFACTS / "memory_profile.json"
                    out_path.write_text(json.dumps(report, indent=2))
                    print(f"Memory stress report saved to {out_path}", flush=True)
                    print(f"Growth from Cycle 25 to 100: RSS={delta_rss_kib} KiB ({delta_rss_kib/1024:.2f} MiB), PSS={delta_pss_kib} KiB ({delta_pss_kib/1024:.2f} MiB)", flush=True)
                    assert report["stabilized"], f"Memory failed to stabilize: grew by {delta_rss_kib/1024:.2f} MiB"
                    QGuiApplication.exit(0)

                QTimer.singleShot(100, run_cycle)

        QtQml.QQmlApplicationEngine = StressEngine

        os.environ["QT_QPA_PLATFORM"] = "offscreen"
        sys.argv = [
            str(LAUNCHER),
            "--db", str(db_path),
            "--window-size", "1366x768"
        ]

        try:
            launcher.main()
        except SystemExit as ex:
            if ex.code != 0:
                raise RuntimeError(f"launcher exited with code {ex.code}")

    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Memory stress qualification")
    parser.add_argument("--cycles", type=int, default=100)
    args = parser.parse_args()
    run_memory_stress(args.cycles)
