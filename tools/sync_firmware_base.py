#!/usr/bin/env python3
# =============================================================================
# EH3D-COMMON - tools/sync_firmware_base.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Keeps the part of the node firmware that the EH3D node projects share (the settings, the network, the panel's server, the log, the
certificate, the JSON reader...) identical in every one of them.

The node projects (EH3D-NODE-BUNGARD today, the next ones tomorrow) grow from one base, and a fix made in one used to have to be made in each. The files
they share are kept here, once, in `firmware_base/`, with the name of the project written as placeholders; this tool writes them into every project that uses
them, and `check` says when a copy has drifted (a fix made in one project only). A project keeps its own copy in its own tree, so each builds on its own:
nothing is shared at build time.

    python tools/sync_firmware_base.py check     # exit 1 when a copy differs from the base
    python tools/sync_firmware_base.py sync      # write the base into every project that uses each file
    python tools/sync_firmware_base.py import    # (re)make the base from the files of a project: python ... import EH3D-NODE-BUNGARD
    python tools/sync_firmware_base.py status    # which file goes where

The project's name appears in a shared file in four forms, each with a placeholder: EH3D-NODE-BUNGARD (@PROJECT@), eh3d-node-bungard (@project-@),
eh3d_node_bungard (@project_@) and bungard (@kind@, only where a file says it by hand). A file that says something else in one project than in the others
(a route of the panel, a setting of the machine) is not shared: it stays the project's own.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
BASE_DIR = HERE.parent / "firmware_base"
MANIFEST = BASE_DIR / "manifest.json"
ROOT = HERE.parent.parent                 # the folder that holds the EH3D repositories

# project -> (EH3D-X, eh3d-x, eh3d_x, x); the forms in this order become the placeholders below
PROJECTS = {
    "EH3D-NODE-BUNGARD": ("EH3D-NODE-BUNGARD", "eh3d-node-bungard", "eh3d_node_bungard", "bungard"),
}
PLACEHOLDERS = ("@PROJECT@", "@project-@", "@project_@", "@kind@")

# The files worth sharing, by their place in a project: the generic part of a node (nothing here says what the node controls).
CANDIDATES = [
    "main/api_shared.cpp", "main/api_shared.hpp", "main/board_ethernet.cpp", "main/board_ethernet.hpp", "main/clock_sync.cpp", "main/clock_sync.hpp",
    "main/entropy.cpp", "main/entropy.hpp", "main/improv_serial.cpp", "main/improv_serial.hpp", "main/log_buffer.cpp", "main/log_buffer.hpp",
    "main/network.cpp", "main/network.hpp", "main/node_guard.cpp", "main/node_guard.hpp", "main/node_store.cpp", "main/node_store.hpp",
    "main/psram_mem.cpp", "main/psram_mem.hpp", "main/task_health.cpp", "main/task_health.hpp", "main/tls_cert.cpp", "main/tls_cert.hpp",
    "main/web_server.hpp",
    "core/admin_sources.hpp", "core/auth.hpp", "core/backoff.hpp", "core/board_s3.hpp", "core/civil_time.hpp", "core/config_common.hpp",
    "core/improv.hpp", "core/json.hpp", "core/memory_budget.hpp", "core/net_text.hpp", "core/netplan.hpp", "core/node_guard.hpp", "core/node_id.hpp",
    "core/node_settings.hpp", "core/semver.hpp", "core/stored_load.hpp", "core/task_health.hpp", "core/web_policy.hpp",
    "tools/pack_panel.py", "tools/check_sdkconfig.py",
    "tests/test_base.cpp", "tests/test_shared.cpp", "tests/test_clock.cpp", "tests/check_task_safety.py",
]


def to_base(text: str, names: tuple[str, ...]) -> str:
    # the longest forms first, so that a short one never eats part of a long one
    for name, placeholder in sorted(zip(names, PLACEHOLDERS), key=lambda p: -len(p[0])):
        if placeholder != "@kind@":
            text = text.replace(name, placeholder)
    return text


def from_base(text: str, names: tuple[str, ...]) -> str:
    for name, placeholder in zip(names, PLACEHOLDERS):
        text = text.replace(placeholder, name)
    return text


def read(path: Path) -> str | None:
    try:
        return path.read_bytes().decode("utf-8")
    except FileNotFoundError:
        return None


def load_manifest() -> dict:
    return json.loads(MANIFEST.read_text(encoding="utf-8")) if MANIFEST.exists() else {"files": {}}


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


def command_import(project: str) -> int:
    if project not in PROJECTS:
        print(f"unknown project {project}; known: {', '.join(PROJECTS)}")
        return 2
    names = PROJECTS[project]
    manifest = load_manifest()
    taken = 0
    for rel in CANDIDATES:
        source = read(ROOT / project / rel)
        if source is None:
            continue
        write_text(BASE_DIR / rel, to_base(source, names))
        entry = manifest["files"].setdefault(rel, {"targets": {}})
        entry["targets"][project] = rel
        taken += 1
    BASE_DIR.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"imported {taken} files from {project} into firmware_base/")
    return 0


def command_sync(check_only: bool) -> int:
    manifest = load_manifest()
    drift = []
    skipped: set[str] = set()
    written = 0
    for rel, entry in sorted(manifest["files"].items()):
        base = read(BASE_DIR / rel)
        if base is None:
            drift.append(f"firmware_base/{rel} is missing")
            continue
        for project, target in entry["targets"].items():
            if not (ROOT / project).is_dir():
                skipped.add(project)   # CI has this repository alone: there is nothing to compare with
                continue
            names = PROJECTS[project]
            want = from_base(base, names)
            path = ROOT / project / target
            have = read(path)
            if have == want:
                continue
            if check_only:
                drift.append(f"{project}/{target} differs from firmware_base/{rel}")
            else:
                write_text(path, want)
                written += 1
    if check_only:
        for line in drift:
            print("  DRIFT", line)
        for project in sorted(skipped):
            print(f"  (skipped {project}: its folder is not next to this repository)")
        print("FIRMWARE_BASE=" + ("PASS" if not drift else "FAIL"))
        return 0 if not drift else 1
    for line in drift:
        print("  PROBLEM", line)
    print(f"wrote {written} files")
    return 0 if not drift else 1


def command_status() -> int:
    manifest = load_manifest()
    for rel, entry in sorted(manifest["files"].items()):
        print(f"{rel:36s} -> {', '.join(sorted(entry['targets']))}")
    print(f"{len(manifest['files'])} shared files")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Keeps the shared part of the node firmware identical in every project")
    parser.add_argument("command", choices=["check", "sync", "import", "status"])
    parser.add_argument("project", nargs="?", default="EH3D-NODE-BUNGARD", help="for import: the project whose files become the base")
    args = parser.parse_args()
    if args.command == "check":
        return command_sync(True)
    if args.command == "sync":
        return command_sync(False)
    if args.command == "import":
        return command_import(args.project)
    return command_status()


if __name__ == "__main__":
    sys.exit(main())
