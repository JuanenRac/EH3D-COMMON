#!/usr/bin/env python3
# =============================================================================
# EH3D-COMMON - tools/fetch_pdfium.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Puts the pinned PDFium library of one platform, with its licences, in a folder.

    python tools/fetch_pdfium.py --out target/debug/deps
    python tools/fetch_pdfium.py --platform windows-x86 --out dist/program

The release, the file names and the SHA-256 of the archive and of the library are read from
crates/viewer/pdfium.json. Nothing is written unless both sums are right. Only the library and the
licence texts are taken out of the archive, each under a name chosen here. It is for the builds of the
programs and the tests: a program never downloads anything when it runs.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import platform
import sys
import tarfile
import urllib.request
from pathlib import Path, PurePosixPath

PINS = Path(__file__).resolve().parents[1] / "crates" / "viewer" / "pdfium.json"
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024


def this_platform() -> str:
    system = {"Windows": "windows", "Linux": "linux", "Darwin": "macos"}.get(platform.system(), platform.system())
    machine = platform.machine().lower()
    arch = {"amd64": "x86_64", "x86_64": "x86_64", "x86": "x86", "i686": "x86", "arm64": "aarch64", "aarch64": "aarch64"}.get(machine, machine)
    return f"{system}-{arch}"


def check(data: bytes, expected: str, what: str) -> None:
    found = hashlib.sha256(data).hexdigest()
    if found != expected:
        raise SystemExit(f"PDFIUM=FAIL the SHA-256 of {what} is {found}, the pinned one is {expected}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--platform", default=this_platform(), help="a key of pdfium.json (default: this computer)")
    parser.add_argument("--out", required=True, type=Path, help="the folder of the program")
    parser.add_argument("--archive", type=Path, help="an archive already downloaded, instead of fetching it")
    args = parser.parse_args()

    pins = json.loads(PINS.read_text(encoding="utf-8"))
    pin = pins["platforms"].get(args.platform)
    if pin is None:
        raise SystemExit(f"PDFIUM=FAIL no pinned build for {args.platform}; known: {', '.join(sorted(pins['platforms']))}")
    if args.archive:
        data = args.archive.read_bytes()
    else:
        url = f"{pins['project']}/releases/download/{pins['release']}/{pin['asset']}"
        with urllib.request.urlopen(url, timeout=120) as response:  # noqa: S310 - a fixed https address
            data = response.read(MAX_ARCHIVE_BYTES + 1)
    if len(data) > MAX_ARCHIVE_BYTES:
        raise SystemExit("PDFIUM=FAIL the archive is larger than expected")
    check(data, pin["archive_sha256"], pin["asset"])

    wanted: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive.getmembers():
            path = PurePosixPath(member.name.removeprefix("./"))
            if not member.isfile():
                continue
            if str(path) == pin["member"]:
                target = pin["library"]
            elif str(path) == "LICENSE":
                target = "pdfium-licenses/LICENSE"
            elif len(path.parts) == 2 and path.parts[0] == "licenses" and path.name.endswith((".txt", ".md", ".ijg")):
                target = f"pdfium-licenses/{path.name}"
            else:
                continue
            handle = archive.extractfile(member)
            if handle is not None:
                wanted[target] = handle.read()
    if pin["library"] not in wanted:
        raise SystemExit(f"PDFIUM=FAIL {pin['member']} is not in the archive")
    check(wanted[pin["library"]], pin["library_sha256"], pin["library"])

    for name, content in wanted.items():
        target = args.out / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
    print(f"PDFIUM=PASS {pins['version']} {args.platform}: {pin['library']} and {len(wanted) - 1} licence files in {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
