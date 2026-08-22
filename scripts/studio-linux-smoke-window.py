#!/usr/bin/env python3
"""CHI-82: identify the Studio X11 client by PID / _NET_WM_PID only.

Title, WM_CLASS, and largest-area are not identities. The unique viewable
top-level client owned by the process PID is the window, or this fails.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from typing import Callable, Iterable

MIN_CLIENT = 200
HEX_OR_DEC = re.compile(r"0x[0-9a-fA-F]+|\b\d+\b")


class IdentifyError(Exception):
    pass


def parse_ids(text: str) -> list[int]:
    ids: list[int] = []
    for tok in HEX_OR_DEC.findall(text):
        try:
            ids.append(int(tok, 0))
        except ValueError:
            continue
    return ids


def parse_client_list(text: str) -> list[int]:
    """IDs from _NET_CLIENT_LIST only. Ignore property-name punctuation."""
    if "#" in text:
        text = text.split("#", 1)[1]
    elif "=" in text:
        text = text.split("=", 1)[1]
    else:
        return []
    return parse_ids(text)


def parse_net_pid(text: str) -> int | None:
    match = re.search(r"=\s*(\d+)", text)
    return int(match.group(1)) if match else None


def parse_frame_extents(text: str) -> tuple[int, int, int, int]:
    if "=" not in text:
        return (0, 0, 0, 0)
    nums = [int(n) for n in re.findall(r"-?\d+", text.split("=", 1)[1])]
    nums.extend([0] * (4 - len(nums)))
    left, right, top, bottom = nums[:4]
    return left, right, top, bottom


def parse_xwininfo(text: str) -> dict:
    def num(label: str) -> int | None:
        match = re.search(rf"{re.escape(label)}:\s*(-?\d+)", text)
        return int(match.group(1)) if match else None

    return {
        "x": num("Absolute upper-left X"),
        "y": num("Absolute upper-left Y"),
        "w": num("Width"),
        "h": num("Height"),
        "mapped": "IsViewable" in text,
    }


def plausible_client(info: dict) -> bool:
    width, height = info.get("w"), info.get("h")
    return (
        info.get("mapped") is True
        and isinstance(width, int)
        and isinstance(height, int)
        and width >= MIN_CLIENT
        and height >= MIN_CLIENT
    )


def select_unique_viewable(
    pid: int,
    client_ids: Iterable[int],
    windows: dict[int, tuple[int | None, dict]],
) -> tuple[int, dict]:
    """Return the unique viewable plausible client owned by pid.

    `windows` maps wid -> (net_pid or None, xwininfo dict).
    """
    owned = []
    unmapped = []
    too_small = []
    other_pid = []
    for wid in client_ids:
        if wid not in windows:
            continue
        net_pid, info = windows[wid]
        if net_pid != pid:
            if net_pid is not None:
                other_pid.append(wid)
            continue
        if info.get("mapped") is not True:
            unmapped.append(wid)
            continue
        if not plausible_client(info):
            too_small.append(f"{wid}:{info.get('w')}x{info.get('h')}")
            continue
        owned.append((wid, info))

    if len(owned) == 1:
        return owned[0]
    if not owned:
        raise IdentifyError(
            f"no unique viewable Studio client for _NET_WM_PID={pid} "
            f"(unmapped={unmapped} too_small={too_small})"
        )
    listed = ", ".join(f"{wid} {info['w']}x{info['h']}" for wid, info in owned)
    raise IdentifyError(
        f"ambiguous Studio clients for _NET_WM_PID={pid}: {listed} "
        "(largest-area is not an identity; fail-closed)"
    )


Runner = Callable[[list[str]], tuple[int, str, str]]


def default_runner(cmd: list[str]) -> tuple[int, str, str]:
    try:
        proc = subprocess.run(cmd, text=True, capture_output=True, check=False)
    except OSError as exc:
        raise IdentifyError(f"{cmd[0]} failed to start: {exc}") from exc
    return proc.returncode, proc.stdout, proc.stderr


def identify(
    pid: int,
    display: str,
    expect_id: int | None = None,
    runner: Runner = default_runner,
) -> dict:
    code, out, err = runner(["xprop", "-root", "_NET_CLIENT_LIST"])
    if code != 0:
        detail = (err or out).strip() or f"exit {code}"
        raise IdentifyError(f"xprop -root _NET_CLIENT_LIST failed: {detail}")
    client_ids = parse_client_list(out)
    if not client_ids:
        raise IdentifyError("root _NET_CLIENT_LIST is empty")

    windows: dict[int, tuple[int | None, dict]] = {}
    for wid in client_ids:
        p_code, p_out, p_err = runner(["xprop", "-id", str(wid), "_NET_WM_PID"])
        i_code, i_out, i_err = runner(["xwininfo", "-id", str(wid)])
        if p_code != 0 or i_code != 0:
            raise IdentifyError(
                f"cannot prove uniqueness: probe failed for client {wid} "
                f"(xprop={p_err.strip() or p_code} xwininfo={i_err.strip() or i_code})"
            )
        windows[wid] = (parse_net_pid(p_out), parse_xwininfo(i_out))

    wid, info = select_unique_viewable(pid, client_ids, windows)
    if expect_id is not None and wid != expect_id:
        raise IdentifyError(
            f"Studio XID changed: expected {expect_id} got {wid} (pid={pid})"
        )

    e_code, e_out, e_err = runner(["xprop", "-id", str(wid), "_NET_FRAME_EXTENTS"])
    if e_code != 0:
        print(
            f"extents probe failed for {wid}: {e_err.strip() or e_code}",
            file=sys.stderr,
        )
    left, right, top, bottom = parse_frame_extents(e_out if e_code == 0 else "")

    c_code, c_out, _c_err = runner(["xprop", "-id", str(wid), "WM_CLASS"])
    wm_class = ""
    if c_code == 0:
        match = re.search(r"=\s*(.*)", c_out)
        wm_class = match.group(1).strip() if match else ""

    payload = {
        "id": str(wid),
        "pid": pid,
        "identity": "net_wm_pid",
        "wm_class": wm_class,
        "client_x": info["x"],
        "client_y": info["y"],
        "w": info["w"],
        "h": info["h"],
        "frame_left": left,
        "frame_right": right,
        "frame_top": top,
        "frame_bottom": bottom,
        "frame_x": info["x"] - left,
        "frame_y": info["y"] - top,
        "display": display,
        "mapped": True,
    }
    return payload


def write_payload(path: str, payload: dict) -> None:
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, indent=2)
        fh.write("\n")


def parse_seconds(value: str) -> float | None:
    if not value:
        return None
    text = str(value).strip()
    if text.endswith("s"):
        text = text[:-1]
    try:
        return float(text)
    except ValueError:
        return None


def ruler_commit_ok(playhead: str, duration: str, min_frac: float = 0.5) -> bool:
    commit = parse_seconds(playhead)
    total = parse_seconds(duration)
    return commit is not None and total is not None and total > 0 and commit >= total * min_frac


def playhead_from_suffix(text: str, reason: str) -> str | None:
    for line in text.splitlines():
        match = re.search(r"semantic_state\s+(\{.*\})", line)
        if not match:
            continue
        try:
            obj = json.loads(match.group(1))
        except json.JSONDecodeError:
            continue
        if obj.get("reason") == reason:
            playhead = obj.get("playhead")
            if playhead:
                return str(playhead)
    return None


def self_test() -> None:
    client_list = (
        "_NET_CLIENT_LIST(WINDOW): window id # 0x1a00007, 0x1a0000c, 0x1a00011"
    )
    ids = parse_client_list(client_list)
    assert ids == [0x1A00007, 0x1A0000C, 0x1A00011], ids
    assert parse_client_list("no delimiter") == []
    assert parse_ids("Lattice Studio 1400") == [1400]

    studio = {
        "x": 1,
        "y": 57,
        "w": 1400,
        "h": 840,
        "mapped": True,
    }
    dialog = {
        "x": 10,
        "y": 10,
        "w": 1600,
        "h": 900,
        "mapped": True,
    }
    hidden = {
        "x": 0,
        "y": 0,
        "w": 1920,
        "h": 1080,
        "mapped": False,
    }
    tiny = {"x": 0, "y": 0, "w": 80, "h": 24, "mapped": True}

    wid, info = select_unique_viewable(
        23500,
        [0x1A00007, 0x1A0000C, 0x1A00011],
        {
            0x1A00007: (23500, studio),
            0x1A0000C: (99, dialog),
            0x1A00011: (23500, hidden),
        },
    )
    assert wid == 0x1A00007 and info is studio

    try:
        select_unique_viewable(
            23500,
            [0x1A00007, 0x1A0000C],
            {
                0x1A00007: (23500, studio),
                0x1A0000C: (23500, dialog),
            },
        )
    except IdentifyError as exc:
        assert "ambiguous" in str(exc)
        assert "largest-area" in str(exc)
    else:
        raise AssertionError("ambiguous clients must fail-closed")

    try:
        select_unique_viewable(
            23500,
            [0x1A00011],
            {0x1A00011: (23500, hidden)},
        )
    except IdentifyError as exc:
        assert "unmapped" in str(exc)
    else:
        raise AssertionError("unmapped-only must fail")

    try:
        select_unique_viewable(
            23500,
            [0x1],
            {0x1: (23500, tiny)},
        )
    except IdentifyError as exc:
        assert "too_small" in str(exc)
    else:
        raise AssertionError("tiny windows must not be Studio")

    replies = {
        ("xprop", "-root", "_NET_CLIENT_LIST"): (
            0,
            "_NET_CLIENT_LIST(WINDOW): window id # 0x1a00007\n",
            "",
        ),
        ("xprop", "-id", "27262983", "_NET_WM_PID"): (
            0,
            "_NET_WM_PID(CARDINAL) = 23500\n",
            "",
        ),
        ("xwininfo", "-id", "27262983"): (
            0,
            "Absolute upper-left X:  1\nAbsolute upper-left Y:  57\n"
            "Width: 1400\nHeight: 840\nMap State: IsViewable\n",
            "",
        ),
        ("xprop", "-id", "27262983", "_NET_FRAME_EXTENTS"): (
            0,
            "_NET_FRAME_EXTENTS(CARDINAL) = 1, 1, 28, 1\n",
            "",
        ),
        ("xprop", "-id", "27262983", "WM_CLASS"): (0, 'WM_CLASS(STRING) = \n', ""),
    }

    def runner(cmd: list[str]) -> tuple[int, str, str]:
        return replies[tuple(cmd)]

    payload = identify(23500, ":1", runner=runner)
    assert payload["id"] == "27262983"
    assert payload["identity"] == "net_wm_pid"
    assert payload["w"] == 1400 and payload["h"] == 840
    assert payload["frame_top"] == 28
    assert payload["client_x"] == 1 and payload["client_y"] == 57

    try:
        identify(23500, ":1", expect_id=1, runner=runner)
    except IdentifyError as exc:
        assert "changed" in str(exc)
    else:
        raise AssertionError("XID change must fail")

    probe_fail = dict(replies)
    probe_fail[("xprop", "-root", "_NET_CLIENT_LIST")] = (
        0,
        "_NET_CLIENT_LIST(WINDOW): window id # 0x1a00007, 0x1a0000c\n",
        "",
    )
    probe_fail[("xprop", "-id", "27262988", "_NET_WM_PID")] = (
        1,
        "",
        "xprop: error: No such window",
    )
    probe_fail[("xwininfo", "-id", "27262988")] = (1, "", "xwininfo: error")

    def failing_runner(cmd: list[str]) -> tuple[int, str, str]:
        return probe_fail[tuple(cmd)]

    try:
        identify(23500, ":1", runner=failing_runner)
    except IdentifyError as exc:
        assert "cannot prove uniqueness" in str(exc)
        assert "27262988" in str(exc)
    else:
        raise AssertionError("unprobed sibling client must fail-closed")

    suffix = (
        'semantic_state {"reason":"timeline-pointer-begin","playhead":"0.80s"}\n'
        'semantic_state {"reason":"timeline-pointer-commit","playhead":"3.200056s"}\n'
    )
    assert playhead_from_suffix(suffix, "timeline-pointer-commit") == "3.200056s"
    assert parse_seconds("3.200056s") == 3.200056
    assert parse_seconds("4s") == 4.0
    assert ruler_commit_ok("3.200056s", "4s")
    assert not ruler_commit_ok("0.90s", "4s")
    print("studio-linux-smoke-window self-test ok")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    sub = parser.add_subparsers(dest="cmd")
    identify_p = sub.add_parser("identify")
    identify_p.add_argument("--pid", type=int, required=True)
    identify_p.add_argument("--out", required=True)
    identify_p.add_argument("--display", required=True)
    identify_p.add_argument("--expect-id", type=int, default=None)
    playhead_p = sub.add_parser("playhead")
    playhead_p.add_argument("--log", required=True)
    playhead_p.add_argument("--offset", type=int, default=0)
    playhead_p.add_argument("--reason", required=True)
    ruler_p = sub.add_parser("ruler-commit")
    ruler_p.add_argument("--playhead", required=True)
    ruler_p.add_argument("--duration", required=True)
    args = parser.parse_args(argv)
    if args.self_test:
        self_test()
        return 0
    if args.cmd == "playhead":
        text = open(args.log, encoding="utf-8", errors="replace").read()[args.offset :]
        value = playhead_from_suffix(text, args.reason)
        if not value:
            print(f"no playhead for reason={args.reason} after offset {args.offset}", file=sys.stderr)
            return 1
        print(value)
        return 0
    if args.cmd == "ruler-commit":
        if not ruler_commit_ok(args.playhead, args.duration):
            print(
                f"playhead {args.playhead} is below half of duration {args.duration}",
                file=sys.stderr,
            )
            return 1
        return 0
    if args.cmd != "identify":
        parser.print_help()
        return 2
    try:
        payload = identify(args.pid, args.display, expect_id=args.expect_id)
    except IdentifyError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    write_payload(args.out, payload)
    print(payload["id"])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
