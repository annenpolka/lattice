#!/usr/bin/env python3
"""Capture visual exploration screenshots of Lattice Studio across various states and sizes."""

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET_BIN = ROOT / "target" / "debug" / "lattice-studio"
SCREENSHOT_DIR = ROOT / "docs" / "screenshots"
SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)

WINDOW_PY = ROOT / "scripts" / "studio-linux-smoke-window.py"


def run_cmd(cmd, check=True):
    return subprocess.run(cmd, shell=True, check=check, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


def identify_window(pid):
    out_json = SCREENSHOT_DIR / "temp_window.json"
    cmd = f'python3 "{WINDOW_PY}" identify --pid {pid} --out "{out_json}" --display :1'
    res = subprocess.run(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if res.returncode != 0:
        return None
    with open(out_json) as f:
        return json.load(f)


def capture_window(wid, width, height, dest_path):
    cmd = (
        f'ffmpeg -y -hide_banner -loglevel error '
        f'-f x11grab -window_id "{wid}" -video_size "{width}x{height}" '
        f'-i :1 -frames:v 1 "{dest_path}"'
    )
    subprocess.run(cmd, shell=True, check=True)


class StudioRunner:
    def __init__(self, fixture="timeline-basic", smoke_ms=60000):
        self.fixture = fixture
        self.smoke_ms = smoke_ms
        self.proc = None
        self.log_path = SCREENSHOT_DIR / "temp_studio.log"
        self.state_path = SCREENSHOT_DIR / "temp_studio_state.json"
        self.geom_path = SCREENSHOT_DIR / "temp_studio_geom.json"
        self.wid = None
        self.win_info = None

    def start(self):
        if self.log_path.exists():
            self.log_path.unlink()
        if self.state_path.exists():
            self.state_path.unlink()
        if self.geom_path.exists():
            self.geom_path.unlink()

        env = os.environ.copy()
        env["DISPLAY"] = ":1"
        env["LATTICE_STUDIO_LOG"] = str(self.log_path)
        env["LATTICE_STUDIO_STATE"] = str(self.state_path)
        env["LATTICE_STUDIO_GEOM"] = str(self.geom_path)
        env["LATTICE_STUDIO_PREVIEW"] = "0"
        env["LATTICE_STUDIO_AUDIO_MONITOR"] = "0"
        env["LATTICE_STUDIO_AUTOPLAY"] = "0"
        env["LATTICE_STUDIO_SMOKE_MS"] = str(self.smoke_ms)
        env["LATTICE_STUDIO_RENDERER"] = "cpu"
        env["RUST_BACKTRACE"] = "1"

        cmd = [str(TARGET_BIN), "--ui-fixture", self.fixture]
        self.proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        # wait for first paint & window
        deadline = time.time() + 15
        while time.time() < deadline:
            if self.log_path.exists():
                text = self.log_path.read_text()
                if "open_window ok" in text and "first paint" in text:
                    break
            time.sleep(0.2)

        # identify window
        deadline = time.time() + 10
        while time.time() < deadline:
            self.win_info = identify_window(self.proc.pid)
            if self.win_info:
                self.wid = self.win_info["id"]
                break
            time.sleep(0.2)

        if not self.wid:
            self.stop()
            raise RuntimeError(f"Could not identify window for PID {self.proc.pid}")

        run_cmd(f"xdotool windowactivate --sync {self.wid}")
        run_cmd(f"xdotool windowfocus --sync {self.wid}")
        time.sleep(0.5)

    def refresh_geom(self):
        deadline = time.time() + 5
        while time.time() < deadline and not self.geom_path.exists():
            time.sleep(0.1)
        if self.geom_path.exists():
            with open(self.geom_path) as f:
                return json.load(f)
        return None

    def refresh_win_info(self):
        self.win_info = identify_window(self.proc.pid)
        return self.win_info

    def resize_window(self, w, h):
        run_cmd(f"xdotool windowsize --sync {self.wid} {w} {h}")
        time.sleep(0.5)
        self.refresh_win_info()

    def mouse_move(self, lx, ly):
        info = self.refresh_win_info()
        cx = info["client_x"] + lx
        cy = info["client_y"] + ly
        run_cmd(f"xdotool mousemove --sync {cx} {cy}")
        time.sleep(0.1)

    def click(self, lx, ly):
        self.mouse_move(lx, ly)
        run_cmd("xdotool click 1")
        time.sleep(0.3)

    def mouse_down(self, lx, ly):
        self.mouse_move(lx, ly)
        run_cmd("xdotool mousedown 1")
        time.sleep(0.1)

    def mouse_up(self, lx, ly):
        self.mouse_move(lx, ly)
        run_cmd("xdotool mouseup 1")
        time.sleep(0.1)

    def capture(self, filename):
        dest = SCREENSHOT_DIR / filename
        info = self.refresh_win_info()
        capture_window(self.wid, info["w"], info["h"], dest)
        print(f"Captured: {dest.name} ({info['w']}x{info['h']})")
        return dest

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None


def main():
    print("Building lattice-studio...")
    run_cmd(f'RUSTFLAGS="-C linker=gcc" cargo build -p lattice-studio --features window', check=True)

    # 1. Timeline Basic - Default Layout
    print("\n--- 1. Timeline Basic (Default Layout) ---")
    runner = StudioRunner(fixture="timeline-basic", smoke_ms=60000)
    runner.start()
    try:
        runner.resize_window(1400, 840)
        time.sleep(0.5)
        runner.capture("01_default_layout_1400x840.png")

        geom = runner.refresh_geom()

        # 2. Hover over Action Button ("Split at Playhead" or "Set In")
        print("\n--- 2. Hover over Actions Bar ---")
        runner.mouse_move(200, 75)
        time.sleep(0.3)
        runner.capture("02_hover_action_button.png")

        # 3. Hover over Timeline Track / Clip
        print("\n--- 3. Hover over Timeline Track ---")
        if geom and "tracks" in geom and len(geom["tracks"]) > 0:
            track = geom["tracks"][0]
            tx = int(track["x"] + track["w"] * 0.4)
            ty = int(track["y"] + track["h"] / 2)
            runner.mouse_move(tx, ty)
            time.sleep(0.3)
            runner.capture("03_hover_timeline_clip.png")

        # 4. Click Timeline Clip (Selected state)
        print("\n--- 4. Click Timeline Clip (Selected) ---")
        if geom and "tracks" in geom and len(geom["tracks"]) > 0:
            track = geom["tracks"][0]
            tx = int(track["x"] + track["w"] * 0.4)
            ty = int(track["y"] + track["h"] / 2)
            runner.click(tx, ty)
            time.sleep(0.5)
            runner.capture("04_selected_timeline_clip.png")

        # 5. Click Sequence Tree Scene Item
        print("\n--- 5. Click Sequence Tree Item ---")
        geom = runner.refresh_geom()
        if geom and "tree" in geom and len(geom["tree"]) > 1:
            tree_item = geom["tree"][1]  # scene:demo
            sx = int(tree_item["x"] + tree_item["w"] / 2)
            sy = int(tree_item["y"] + tree_item["h"] / 2)
            runner.click(sx, sy)
            time.sleep(0.5)
            runner.capture("05_selected_tree_scene.png")

        # 6. Click Sequence Tree Title Item
        if geom and "tree" in geom:
            title_item = next((item for item in geom["tree"] if item.get("kind") == "title"), None)
            if title_item:
                print("\n--- 6. Click Sequence Tree Title Item ---")
                tx = int(title_item["x"] + title_item["w"] / 2)
                ty = int(title_item["y"] + title_item["h"] / 2)
                runner.click(tx, ty)
                time.sleep(0.5)
                runner.capture("06_selected_tree_title.png")

        # 7. Drag Ruler / Scrub in Progress
        print("\n--- 7. Drag Ruler (Scrubbing) ---")
        geom = runner.refresh_geom()
        if geom and "ruler" in geom:
            ruler = geom["ruler"]
            rx_start = int(ruler["x"] + ruler["w"] * 0.2)
            rx_mid = int(ruler["x"] + ruler["w"] * 0.6)
            ry = int(ruler["y"] + ruler["h"] / 2)
            runner.mouse_down(rx_start, ry)
            time.sleep(0.1)
            runner.mouse_move(rx_mid, ry)
            time.sleep(0.2)
            runner.capture("07_drag_scrub_ruler.png")
            runner.mouse_up(rx_mid, ry)
            time.sleep(0.2)

        # 8. Window size: Compact (1024x768)
        print("\n--- 8. Window Size Compact (1024x768) ---")
        runner.resize_window(1024, 768)
        time.sleep(0.5)
        runner.capture("08_window_compact_1024x768.png")

        # 9. Window size: Wide / High-Res (1800x1000)
        print("\n--- 9. Window Size Wide (1800x1000) ---")
        runner.resize_window(1800, 1000)
        time.sleep(0.5)
        runner.capture("09_window_wide_1800x1000.png")

    finally:
        runner.stop()

    # 10. Dense Project Fixture
    print("\n--- 10. Dense Project Fixture ---")
    runner_dense = StudioRunner(fixture="dense-project", smoke_ms=60000)
    runner_dense.start()
    try:
        runner_dense.resize_window(1400, 840)
        time.sleep(0.5)
        runner_dense.capture("10_dense_project_layout.png")
    finally:
        runner_dense.stop()

    # 11. Drag-Valid Fixture
    print("\n--- 11. Drag-Valid Fixture ---")
    runner_drag = StudioRunner(fixture="drag-valid", smoke_ms=60000)
    runner_drag.start()
    try:
        runner_drag.resize_window(1400, 840)
        time.sleep(0.5)
        runner_drag.capture("11_drag_valid_layout.png")
    finally:
        runner_drag.stop()

    print("\nAll visual exploration screenshots captured successfully!")


if __name__ == "__main__":
    main()
