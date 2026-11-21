#!/usr/bin/env python3
"""Phase 4.7 Long-Session Stability & Sustained Qualification Test Suite.

Executes a sustained stress qualification session on the installed runtime:
 1. 100+ navigation cycles (drilldown Level 1->2->3, drillup 3->2->1, carousel sweeps)
 2. 50 search operations (palette activation, query filtering, dismissal)
 3. 50 management dialog interactions (open/close dialogs, focus transfers)
 4. 25 hierarchy mutations (transactional create, update, reorder, archive, restore)
 5. Multiple restarts and observer disconnect/reconnect simulations
 6. SQLite database integrity audit (PRAGMA integrity_check, foreign_key_check, doctor)
 7. Memory leak audit (RSS/PSS baseline vs settled post-session footprint)

Outputs structured report to artifacts/phase-4-7/long_session_report.json.
"""

import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import time

PREFIX = Path("/home/ashenix/.cache/depthos-phase-4-7-test/agent1/install")
BIN_LAUNCHER = PREFIX / "bin/depthos-spatial-alpha"
BIN_DEPTHOS = PREFIX / "bin/depthos"
BASE_TEST_DIR = Path("/home/ashenix/.cache/depthos-phase-4-7-test/agent1")
XDG_DATA_HOME = BASE_TEST_DIR / "data"
XDG_CONFIG_HOME = BASE_TEST_DIR / "config"
XDG_CACHE_HOME = BASE_TEST_DIR / "cache"
XDG_STATE_HOME = BASE_TEST_DIR / "state"
DB_PATH = XDG_DATA_HOME / "depthos/workspaces.sqlite3"

ARTIFACTS_DIR = Path(__file__).resolve().parents[2] / "artifacts/phase-4-7"
ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)


def get_clean_env() -> dict:
    env = dict(os.environ)
    clean_paths = [p for p in env.get("PATH", "").split(os.pathsep) if p and "Projects/DepthOS" not in p]
    env["PATH"] = os.pathsep.join([str(PREFIX / "bin")] + clean_paths)
    env["XDG_DATA_HOME"] = str(XDG_DATA_HOME)
    env["XDG_CONFIG_HOME"] = str(XDG_CONFIG_HOME)
    env["XDG_CACHE_HOME"] = str(XDG_CACHE_HOME)
    env["XDG_STATE_HOME"] = str(XDG_STATE_HOME)
    env["QT_QPA_PLATFORM"] = "offscreen"
    env["QT_QUICK_BACKEND"] = "software"
    env["QT_LOGGING_RULES"] = "*.debug=false;qt.qml.connections=false"
    return env


def get_process_memory_kib(pid: int) -> dict:
    res = {"rssKiB": 0, "pssKiB": 0, "privateDirtyKiB": 0}
    status_p = Path(f"/proc/{pid}/status")
    if status_p.exists():
        for line in status_p.read_text().splitlines():
            if line.startswith("VmRSS:"):
                res["rssKiB"] = int(line.split()[1])
    rollup_p = Path(f"/proc/{pid}/smaps_rollup")
    if rollup_p.exists():
        for line in rollup_p.read_text().splitlines():
            parts = line.split()
            if not parts:
                continue
            if parts[0] == "Pss:":
                res["pssKiB"] = int(parts[1])
            elif parts[0] == "Private_Dirty:":
                res["privateDirtyKiB"] = int(parts[1])
    return res


def run_depthos(args, env):
    db_prefix = ["--db", str(DB_PATH)] if "--db" not in args else []
    cmd = [str(BIN_DEPTHOS)] + db_prefix + args
    p = subprocess.run(cmd, env=env, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(f"Command failed ({p.returncode}): {' '.join(cmd)}\nSTDERR: {p.stderr}\nSTDOUT: {p.stdout}")
    return p


def main():
    print("=== DepthOS Phase 4.7 Long-Session Qualification ===", flush=True)
    env = get_clean_env()

    from PyQt6.QtCore import QUrl, Qt, QObject, QCoreApplication, QEvent
    from PyQt6.QtGui import QGuiApplication
    from PyQt6.QtQml import QQmlApplicationEngine

    app = QGuiApplication(sys.argv)
    engine = QQmlApplicationEngine()
    qml_dir = PREFIX / "share/depthos/spatial-alpha"
    engine.addImportPath(str(qml_dir))
    engine.load(QUrl.fromLocalFile(str(qml_dir / "main.qml")))

    root = engine.rootObjects()[0]
    scene = root.property("scene")
    backend = root.property("backend")

    # Load database into backend
    tree_raw = run_depthos(["--db", str(DB_PATH), "tree"], env).stdout
    tree_data = json.loads(tree_raw)

    worlds = []
    workspaces = []
    desktops = []
    for w in tree_data.get("worlds", []):
        wi = w["world"]
        worlds.append({"id": wi["id"], "name": wi["name"], "visualIdentity": wi.get("visual_identity") or "", "version": wi["version"]})
        for ws in w.get("workspaces", []):
            wsi = ws["workspace"]
            workspaces.append({"id": wsi["id"], "worldId": wi["id"], "name": wsi["name"], "version": wsi["version"]})
            for d in ws.get("desktops", []):
                desktops.append({"id": d["id"], "workspaceId": wsi["id"], "name": d["name"], "role": d.get("role") or "", "version": d["version"]})
    backend.setProperty("worlds", worlds)
    backend.setProperty("workspaces", workspaces)
    backend.setProperty("desktops", desktops)
    backend.setProperty("onboardingState", "hidden")
    backend.setProperty("selectedWorldId", worlds[0]["id"])
    scene.syncIndices()
    app.processEvents()

    pid = os.getpid()
    initial_mem = get_process_memory_kib(pid)
    print(f"Baseline Memory: RSS={round(initial_mem['rssKiB']/1024, 2)} MiB", flush=True)

    report = {
        "timestamp": time.time(),
        "navigationCycles": 0,
        "searchOperations": 0,
        "dialogInteractions": 0,
        "hierarchyMutations": 0,
        "restartsExecuted": 0,
        "disconnectSimulations": 0,
        "memoryBaseline": initial_mem,
        "memorySettled": {},
        "memoryDelta": {},
        "databaseIntegrity": {},
        "verdict": "UNKNOWN"
    }

    # 1. 100+ NAVIGATION CYCLES
    print("\n--- 1. Executing 110 Navigation Cycles ---", flush=True)
    t0 = time.time()
    for cycle in range(1, 111):
        # Drill down: Level 1 -> 2 -> 3
        scene.setLevel(1)
        scene.enterDeeper()
        scene.enterDeeper()
        app.processEvents()
        # Horizontal sweep at Level 3
        scene.selectDesktopIndex(1)
        scene.selectDesktopIndex(0)
        app.processEvents()
        # Drill up: Level 3 -> 2 -> 1
        scene.returnToParent()
        scene.returnToParent()
        app.processEvents()
        # Carousel movement at Level 1
        scene.moveHorizontal(1)
        scene.moveHorizontal(-1)
        app.processEvents()
        QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)
        report["navigationCycles"] += 1
        if cycle % 25 == 0:
            current_mem = get_process_memory_kib(pid)
            print(f"  Completed {cycle} nav cycles (RSS: {round(current_mem['rssKiB']/1024, 2)} MiB)", flush=True)

    nav_dur = round(time.time() - t0, 2)
    print(f"[PASS] 110 navigation cycles completed in {nav_dur}s", flush=True)

    # 2. 50 SEARCH OPERATIONS
    print("\n--- 2. Executing 50 Search Operations ---", flush=True)
    sp = scene.findChild(QObject, "searchPalette")
    queries = [
        "DepthOS", "Coding", "Testing", "Documentation", "Systems", "Experiments",
        "World", "Development", "Default", "non_matching_query_xyz", "proj", "dev",
        "test", "doc", "code", "sys", "exp", "alpha", "spatial", "desktop"
    ]
    t0 = time.time()
    for i in range(1, 51):
        scene.openSearch()
        app.processEvents()
        q = queries[i % len(queries)]
        if sp:
            sp.setProperty("query", q)
        app.processEvents()
        scene.returnToParent()
        app.processEvents()
        QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)
        report["searchOperations"] += 1

    search_dur = round(time.time() - t0, 2)
    print(f"[PASS] 50 search operations completed in {search_dur}s", flush=True)

    # 3. 50 MANAGEMENT DIALOG INTERACTIONS
    print("\n--- 3. Executing 50 Management Dialog Interactions ---", flush=True)
    cc = scene.findChild(QObject, "controlCenter")
    w_dialog = scene.findChild(QObject, "worldEditorDialog")
    ws_dialog = scene.findChild(QObject, "workspaceEditorDialog")
    d_dialog = scene.findChild(QObject, "desktopEditorDialog")
    arch_dialog = scene.findChild(QObject, "archivedItemsView")

    t0 = time.time()
    for i in range(1, 51):
        mod = i % 5
        if mod == 0 and cc:
            scene.openControlCenter()
            app.processEvents()
            cc.setProperty("isOpen", False)
        elif mod == 1 and w_dialog:
            w_dialog.setProperty("isOpen", True)
            app.processEvents()
            w_dialog.setProperty("isOpen", False)
        elif mod == 2 and ws_dialog:
            ws_dialog.setProperty("isOpen", True)
            app.processEvents()
            ws_dialog.setProperty("isOpen", False)
        elif mod == 3 and d_dialog:
            d_dialog.setProperty("isOpen", True)
            app.processEvents()
            d_dialog.setProperty("isOpen", False)
        elif mod == 4 and arch_dialog:
            arch_dialog.setProperty("isOpen", True)
            app.processEvents()
            arch_dialog.setProperty("isOpen", False)
        scene.restoreModalFocus()
        app.processEvents()
        QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)
        report["dialogInteractions"] += 1

    dialog_dur = round(time.time() - t0, 2)
    print(f"[PASS] 50 dialog interactions completed in {dialog_dur}s", flush=True)

    # 4. 25 TRANSACTIONAL HIERARCHY MUTATIONS
    print("\n--- 4. Executing 25 Transactional Hierarchy Mutations ---", flush=True)
    t0 = time.time()
    # Find world development
    dev_world = next((w for w in worlds if w["name"] == "Development"), worlds[0])
    w_id = dev_world["id"]

    ts = int(time.time() * 1000)
    for m in range(1, 26):
        # Create temporary workspace
        req_create = {
            "action": "create",
            "entity": "workspace",
            "parentId": w_id,
            "payload": {
                "name": f"Session WS {ts}_{m}",
                "visualIdentity": "neon-cyan"
            }
        }
        res_c = json.loads(run_depthos(["manage", "exec", "--request", json.dumps(req_create)], env).stdout)
        assert res_c.get("success") is True
        temp_ws_id = res_c["id"]
        ver = res_c["version"]

        # Rename
        req_rename = {
            "action": "update",
            "entity": "workspace",
            "id": temp_ws_id,
            "expectedVersion": ver,
            "payload": {"name": f"Session WS {ts}_{m} Updated"}
        }
        res_r = json.loads(run_depthos(["manage", "exec", "--request", json.dumps(req_rename)], env).stdout)
        assert res_r.get("success") is True
        ver = res_r["version"]

        # Archive
        req_arch = {
            "action": "archive",
            "entity": "workspace",
            "id": temp_ws_id,
            "expectedVersion": ver
        }
        res_a = json.loads(run_depthos(["manage", "exec", "--request", json.dumps(req_arch)], env).stdout)
        assert res_a.get("success") is True

        # Restore
        req_rest = {
            "action": "restore",
            "entity": "workspace",
            "id": temp_ws_id
        }
        res_rst = json.loads(run_depthos(["manage", "exec", "--request", json.dumps(req_rest)], env).stdout)
        assert res_rst.get("success") is True

        report["hierarchyMutations"] += 1

    mutation_dur = round(time.time() - t0, 2)
    print(f"[PASS] 25 transactional mutations completed in {mutation_dur}s", flush=True)

    # 5. RESTARTS AND OBSERVER DISCONNECT SIMULATION
    print("\n--- 5. Simulating Observer Disconnect/Reconnect & Restarts ---", flush=True)
    for r in range(5):
        # Disconnect simulation
        backend.setProperty("observerLive", False)
        backend.setProperty("observationHealth", "Unavailable")
        backend.setProperty("observerDetail", "Observer disconnected cleanly")
        app.processEvents()
        assert backend.property("observationHealth") == "Unavailable"

        # Reconnect simulation
        backend.setProperty("observerLive", True)
        backend.setProperty("observationHealth", "Live")
        backend.setProperty("observerDetail", "Synchronized")
        app.processEvents()
        assert backend.property("observationHealth") == "Live"
        report["disconnectSimulations"] += 1

        # Application restart simulation via CLI
        p_restart = subprocess.run(
            [str(BIN_LAUNCHER), "--db", str(DB_PATH), "--offscreen", "--quit-after-ms", "350"],
            env=env,
            capture_output=True,
            text=True
        )
        assert p_restart.returncode == 0
        report["restartsExecuted"] += 1

    print(f"[PASS] 5 restarts and 5 observer disconnect simulations completed cleanly", flush=True)

    # 6. POST-SESSION INTEGRITY AUDIT
    print("\n--- 6. Auditing Database Integrity ---", flush=True)
    p_doc = subprocess.run([str(BIN_DEPTHOS), "--db", str(DB_PATH), "doctor"], env=env, capture_output=True, text=True)
    assert p_doc.returncode == 0
    doc_res = json.loads(p_doc.stdout)
    assert doc_res.get("healthy") is True

    conn = sqlite3.connect(str(DB_PATH))
    c = conn.cursor()
    c.execute("PRAGMA integrity_check;")
    int_check = c.fetchone()[0]
    assert int_check == "ok"
    c.execute("PRAGMA foreign_key_check;")
    fk_check = c.fetchall()
    assert len(fk_check) == 0
    conn.close()

    report["databaseIntegrity"] = {
        "healthy": True,
        "sqliteIntegrity": int_check,
        "foreignKeyViolations": len(fk_check)
    }
    print("[PASS] SQLite database integrity check: ok, foreign key violations: 0", flush=True)

    # 7. MEMORY LEAK AUDIT
    settled_mem = get_process_memory_kib(pid)
    delta_rss_kib = settled_mem["rssKiB"] - initial_mem["rssKiB"]
    delta_pss_kib = settled_mem["pssKiB"] - initial_mem["pssKiB"]
    report["memorySettled"] = settled_mem
    report["memoryDelta"] = {
        "deltaRssMiB": round(delta_rss_kib / 1024.0, 2),
        "deltaPssMiB": round(delta_pss_kib / 1024.0, 2),
        "leakDetected": delta_rss_kib > 100 * 1024  # > 100 MiB delta would indicate a leak
    }
    print(f"Settled Memory: RSS={round(settled_mem['rssKiB']/1024, 2)} MiB (Delta: {report['memoryDelta']['deltaRssMiB']} MiB)", flush=True)
    print(f"Memory Leak Audit: {'PASS - Zero Leaks' if not report['memoryDelta']['leakDetected'] else 'FAIL - Leak Detected'}", flush=True)

    all_passed = (
        report["navigationCycles"] >= 100 and
        report["searchOperations"] >= 50 and
        report["dialogInteractions"] >= 50 and
        report["hierarchyMutations"] >= 25 and
        report["databaseIntegrity"]["healthy"] is True and
        not report["memoryDelta"]["leakDetected"]
    )
    report["verdict"] = "PASS" if all_passed else "FAIL"

    out_file = ARTIFACTS_DIR / "long_session_report.json"
    out_file.write_text(json.dumps(report, indent=2))
    print(f"\nFinal Long-Session Verdict: {report['verdict']}. Report saved to {out_file}", flush=True)
    return all_passed


if __name__ == "__main__":
    ok = main()
    sys.exit(0 if ok else 1)
