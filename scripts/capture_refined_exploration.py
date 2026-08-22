#!/usr/bin/env python3
"""Comprehensive visual exploration screenshot generator for Lattice Studio."""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCREENSHOTS_DIR = ROOT / "docs" / "screenshots"
SCREENSHOTS_DIR.mkdir(parents=True, exist_ok=True)
EXE = ROOT / "target" / "debug" / "lattice-studio"
WINDOW_PY = ROOT / "scripts" / "studio-linux-smoke-window.py"


def run_cmd(cmd, check=True):
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if check and res.returncode != 0:
        print(f"Error running {cmd}:\nSTDOUT: {res.stdout}\nSTDERR: {res.stderr}", file=sys.stderr)
        raise RuntimeError(f"Command failed: {cmd}")
    return res


def identify_window(pid, display=":1"):
    out_json = SCREENSHOTS_DIR / "temp_win.json"
    cmd = [sys.executable, str(WINDOW_PY), "identify", "--pid", str(pid), "--out", str(out_json), "--display", display]
    res = run_cmd(cmd, check=False)
    if res.returncode == 0 and out_json.exists():
        with open(out_json, "r") as f:
            data = json.load(f)
        out_json.unlink(missing_ok=True)
        return data
    return None


def capture_window_png(win_id, dest_path, width, height, display=":1"):
    cmd = [
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
        "-f", "x11grab", "-window_id", str(win_id),
        "-video_size", f"{width}x{height}",
        "-i", display,
        "-frames:v", "1", str(dest_path)
    ]
    run_cmd(cmd)
    print(f"Saved: {dest_path.name} ({width}x{height})")


class StudioSessionRunner:
    def __init__(self, fixture="timeline-basic", preview=True, display=":1"):
        self.fixture = fixture
        self.preview = preview
        self.display = display
        self.proc = None
        self.log_file = SCREENSHOTS_DIR / f"temp_{fixture}.log"
        self.geom_file = SCREENSHOTS_DIR / f"temp_{fixture}.geom.json"
        self.state_file = SCREENSHOTS_DIR / f"temp_{fixture}.state.json"
        self.win_info = None

    def __enter__(self):
        env = os.environ.copy()
        env["DISPLAY"] = self.display
        env["LATTICE_STUDIO_LOG"] = str(self.log_file)
        env["LATTICE_STUDIO_GEOM"] = str(self.geom_file)
        env["LATTICE_STUDIO_STATE"] = str(self.state_file)
        env["LATTICE_STUDIO_PREVIEW"] = "1" if self.preview else "0"
        env["LATTICE_STUDIO_AUDIO_MONITOR"] = "0"
        env["LATTICE_STUDIO_AUTOPLAY"] = "0"
        env["LATTICE_STUDIO_RENDERER"] = "cpu"
        env["LATTICE_STUDIO_SMOKE_MS"] = "60000"
        env["RUST_BACKTRACE"] = "1"

        self.geom_file.unlink(missing_ok=True)
        self.log_file.unlink(missing_ok=True)

        self.proc = subprocess.Popen(
            [str(EXE), "--ui-fixture", self.fixture],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env
        )

        deadline = time.time() + 15
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError("Studio exited early")
            self.win_info = identify_window(self.proc.pid, self.display)
            if self.win_info and self.geom_file.exists() and self.geom_file.stat().st_size > 0:
                break
            time.sleep(0.3)

        if not self.win_info:
            self.stop()
            raise RuntimeError("Failed to find window")

        win_id = self.win_info["id"]
        run_cmd(["xdotool", "windowactivate", "--sync", str(win_id)], check=False)
        run_cmd(["xdotool", "windowfocus", "--sync", str(win_id)], check=False)
        time.sleep(0.5)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.stop()

    def geom(self):
        if self.geom_file.exists():
            try:
                with open(self.geom_file) as f:
                    return json.load(f)
            except Exception:
                pass
        return {}

    def client_pos(self, lx, ly):
        return int(self.win_info["client_x"] + lx), int(self.win_info["client_y"] + ly)

    def mouse_move(self, lx, ly):
        sx, sy = self.client_pos(lx, ly)
        run_cmd(["xdotool", "mousemove", "--sync", str(sx), str(sy)])
        time.sleep(0.1)

    def mouse_click(self, lx, ly):
        self.mouse_move(lx, ly)
        run_cmd(["xdotool", "click", "1"])
        time.sleep(0.3)

    def mouse_down(self, lx, ly):
        self.mouse_move(lx, ly)
        run_cmd(["xdotool", "mousedown", "1"])
        time.sleep(0.1)

    def mouse_up(self):
        run_cmd(["xdotool", "mouseup", "1"])
        time.sleep(0.2)

    def key_type(self, text):
        run_cmd(["xdotool", "type", "--delay", "50", text])
        time.sleep(0.3)

    def key_press(self, key):
        run_cmd(["xdotool", "key", key])
        time.sleep(0.2)

    def resize(self, w, h):
        win_id = self.win_info["id"]
        run_cmd(["xdotool", "windowsize", "--sync", str(win_id), str(w), str(h)])
        time.sleep(0.6)
        info = identify_window(self.proc.pid, self.display)
        if info:
            self.win_info = info

    def capture(self, filename):
        dest = SCREENSHOTS_DIR / filename
        win_id = self.win_info["id"]
        info = identify_window(self.proc.pid, self.display)
        if info:
            self.win_info = info
        capture_window_png(win_id, dest, self.win_info["w"], self.win_info["h"], self.display)
        return dest

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()
        self.log_file.unlink(missing_ok=True)
        self.geom_file.unlink(missing_ok=True)
        self.state_file.unlink(missing_ok=True)


def main():
    print("Capturing refined visual exploration artifact suite...")

    # Set 1: Standard Window (1400x840) - Timeline Basic (with Live Video Canvas Preview)
    with StudioSessionRunner(fixture="timeline-basic", preview=True) as s:
        g = s.geom()
        # 01. Initial overview with shared locus on Title "Hello"
        s.capture("01_overview_title_locus_selected.png")

        # 02. Toolbar hover
        play_btn = g.get("play", {"x": 1115, "y": 67, "w": 51, "h": 30})
        s.mouse_move(play_btn["x"] + play_btn["w"]/2, play_btn["y"] + play_btn["h"]/2)
        s.capture("02_toolbar_hover_play.png")

        # 03. Select Scene locus in Sequence tree
        tree = g.get("tree", [])
        scene_node = next((n for n in tree if n.get("kind") == "scene"), None)
        if scene_node:
            s.mouse_click(scene_node["x"] + scene_node["w"]/2, scene_node["y"] + scene_node["h"]/2)
            time.sleep(0.4)
            s.capture("03_scene_locus_selected.png")

        # 04. Focus and edit text in Inspector
        # Inspector title input box
        s.mouse_click(1250, 260)
        s.key_press("BackSpace")
        s.key_press("BackSpace")
        s.key_type(" World")
        s.capture("04_inspector_title_editing.png")

        # Apply edit
        s.mouse_click(1220, 295)
        time.sleep(0.4)
        s.capture("05_inspector_applied_edit.png")

        # Propose review diff
        s.key_type("!")
        s.mouse_click(1300, 295)
        time.sleep(0.4)
        s.capture("06_inspector_proposed_diff_review.png")

        # 07. Timeline Scrub in-flight & committed
        ruler = g.get("ruler", {"x": 72, "y": 685, "w": 640, "h": 23})
        s.mouse_down(ruler["x"] + ruler["w"] * 0.25, ruler["y"] + ruler["h"]/2)
        s.mouse_move(ruler["x"] + ruler["w"] * 0.65, ruler["y"] + ruler["h"]/2)
        time.sleep(0.2)
        s.capture("07_timeline_scrub_in_flight.png")
        s.mouse_up()
        time.sleep(0.3)
        s.capture("08_timeline_scrub_committed.png")

        # 08. Select Video clip in timeline
        tracks = g.get("tracks", [])
        vtrack = next((t for t in tracks if t.get("name") == "Video"), None)
        if vtrack:
            s.mouse_click(vtrack["x"] + vtrack["w"] * 0.4, vtrack["y"] + vtrack["h"]/2)
            time.sleep(0.4)
            s.capture("09_timeline_video_clip_selected.png")

        # 09. Canvas overlay in-flight drag
        # Re-select title
        title_node = next((n for n in tree if n.get("kind") == "title"), None)
        if title_node:
            s.mouse_click(title_node["x"] + title_node["w"]/2, title_node["y"] + title_node["h"]/2)
            time.sleep(0.4)
            # Drag on canvas overlay
            s.mouse_down(450, 350)
            s.mouse_move(520, 390)
            time.sleep(0.2)
            s.capture("10_canvas_overlay_drag_in_flight.png")
            s.mouse_up()

    # Set 2: Multi-scene & High Density: Dense Project (4 scenes)
    with StudioSessionRunner(fixture="dense-project", preview=True) as s:
        time.sleep(0.5)
        s.capture("11_dense_project_multi_scene.png")
        g = s.geom()
        tree = g.get("tree", [])
        scene3 = next((n for n in tree if "scene three" in n.get("label", "").lower() or "three" in n.get("id", "").lower()), None)
        if scene3:
            s.mouse_click(scene3["x"] + scene3["w"]/2, scene3["y"] + scene3["h"]/2)
            time.sleep(0.4)
            s.capture("12_dense_project_scene3_selected.png")

    # Set 3: Valid Reorder Drag: Drag Valid fixture
    with StudioSessionRunner(fixture="drag-valid", preview=True) as s:
        time.sleep(0.5)
        s.capture("13_drag_valid_overview.png")
        g = s.geom()
        tracks = g.get("tracks", [])
        vtrack = next((t for t in tracks if t.get("name") == "Video"), None)
        if vtrack:
            s.mouse_down(vtrack["x"] + vtrack["w"] * 0.25, vtrack["y"] + vtrack["h"]/2)
            s.mouse_move(vtrack["x"] + vtrack["w"] * 0.75, vtrack["y"] + vtrack["h"]/2)
            time.sleep(0.2)
            s.capture("14_drag_valid_reorder_in_flight.png")
            s.mouse_up()

    # Set 4: Window Scale & Layout Adaptability (Wide, Compact, Ultra-compact)
    with StudioSessionRunner(fixture="timeline-basic", preview=True) as s:
        # Wide / 1080p style (1800x1000)
        s.resize(1800, 1000)
        time.sleep(0.5)
        s.capture("15_window_wide_1800x1000.png")

        # Compact (1024x720)
        s.resize(1024, 720)
        time.sleep(0.5)
        s.capture("16_window_compact_1024x720.png")

        # Ultra-compact (800x600)
        s.resize(800, 600)
        time.sleep(0.5)
        s.capture("17_window_ultracompact_800x600.png")

    print("\nAll refined screenshots captured successfully!")


if __name__ == "__main__":
    main()
