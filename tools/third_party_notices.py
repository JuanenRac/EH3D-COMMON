#!/usr/bin/env python3
# =============================================================================
# EH3D-COMMON - tools/third_party_notices.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""The third-party notices of a Rust program of the suite, made from its real dependency tree and checked against it.

    python third_party_notices.py                   write THIRD_PARTY_NOTICES of the repository in the current folder
    python third_party_notices.py --check           fail when a dependency, its licence or a notice is not in the file
    python third_party_notices.py --package NAME --output crates/NAME/THIRD_PARTY_NOTICES

The tree is what `cargo metadata` resolves for the platforms the suite releases for: normal dependencies only (nothing of the
tests or of the build scripts), with every feature of the members. The copyright lines and licence texts are copied from the
source of each crate. A path dependency (a sibling repository) is our own code, but the THIRD_PARTY_NOTICES file written by
hand beside it (data or a library taken from others) is carried along. `--check` compares names and licences, not versions.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

NAME = "THIRD_PARTY_NOTICES"
MARKER = "Made by third_party_notices.py from the dependency tree of Cargo; do not edit by hand."
TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "i686-pc-windows-msvc",
           "aarch64-apple-darwin", "x86_64-apple-darwin")
NOTICE_FILE = re.compile(r"^((un)?licen[cs]e|copying|copyright|notice)([-_.].*)?$|^.*[-_]licen[cs]e(\.(txt|md))?$|^(ofl|ufl)(\.txt)?$", re.IGNORECASE)
CODE_SUFFIXES = {".rs", ".toml", ".json", ".py", ".sh", ".c", ".h", ".cpp", ".png", ".ttf", ".otf", ".yml", ".yaml", ".html"}
# when a crate offers a choice and ships no text of its own, the first of these it offers is used
PREFERENCE = ("MIT", "Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Zlib")

PERMISSION = {
    "MIT": """Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.""",
    "ISC": """Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby
granted, provided that the above copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.""",
    "Zlib": """This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held
liable for any damages arising from the use of this software.

Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter
it and redistribute it freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If
you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not
required.
2. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original
software.
3. This notice may not be removed or altered from any source distribution.""",
}


def flat(text: str) -> str:
    return " ".join(text.split())


def clean(text: str) -> str:
    return "\n".join(line.rstrip() for line in text.replace("﻿", "").replace("\r\n", "\n").replace("\r", "\n").replace("\f", "").split("\n")).strip("\n")


def licence_of(package: dict) -> str:
    return flat((package.get("license") or "see the licence file of the crate").replace("/", " OR "))


def metadata(root: Path, target: str) -> dict:
    command = ["cargo", "metadata", "--format-version", "1", "--all-features", "--filter-platform", target, "--manifest-path", str(root / "Cargo.toml")]
    result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", timeout=600)
    if result.returncode != 0:
        raise SystemExit("cargo metadata failed:\n" + result.stderr[-2000:])
    return json.loads(result.stdout)


def tree(root: Path, wanted: list[str]) -> tuple[list[dict], list[dict]]:
    """(third-party packages, our own packages) reached by normal dependencies from the members (or only those named)."""
    third: dict[str, dict] = {}
    own: dict[str, dict] = {}
    for target in TARGETS:
        data = metadata(root, target)
        packages = {p["id"]: p for p in data["packages"]}
        nodes = {n["id"]: n for n in data["resolve"]["nodes"]}
        pending = [m for m in data["workspace_members"] if not wanted or packages[m]["name"] in wanted]
        if not pending:
            raise SystemExit(f"no member of the workspace is named {wanted}")
        seen: set[str] = set()
        while pending:
            current = pending.pop()
            if current in seen:
                continue
            seen.add(current)
            package = packages[current]
            (third if package.get("source") else own)[current] = package
            pending += [d["pkg"] for d in nodes[current]["deps"] if any(k.get("kind") is None for k in d["dep_kinds"])]

    def order(package: dict) -> tuple[str, str]:
        return package["name"], package["version"]

    return sorted(third.values(), key=order), sorted(own.values(), key=order)


def crate_texts(package: dict) -> list[str]:
    folder = Path(package["manifest_path"]).parent
    files = [p for p in folder.rglob("*") if p.is_file() and NOTICE_FILE.match(p.name) and p.suffix.lower() not in CODE_SUFFIXES]
    # the licences of fonts that are embedded are named after the font
    files += [p for p in folder.glob("fonts/*.txt")]
    if package.get("license_file"):
        files.append(folder / package["license_file"])
    texts = []
    for path in sorted(set(files), key=lambda p: p.as_posix()):
        relative = path.relative_to(folder).parts if path.is_relative_to(folder) else ()
        if any(part in ("target", "tests", "benches", "examples") for part in relative[:-1]) or not path.is_file() or path.stat().st_size > 200_000:
            continue
        text = clean(path.read_text(encoding="utf-8", errors="replace"))
        if text:
            texts.append(text)
    return texts


def missing_texts(package: dict, apache: str | None) -> list[str]:
    """The texts of a crate that ships none, from the licence it states."""
    holders = ", ".join(re.sub(r"\s*<[^>]*>", "", a) for a in package.get("authors") or []) or f"the {package['name']} authors"
    chosen = []
    for part in re.split(r"\s+AND\s+", licence_of(package)):
        offered = [o.strip(" ()") for o in re.split(r"\s+OR\s+", part)]
        name = next((p for p in PREFERENCE if p in offered), None)
        if name in PERMISSION:
            chosen.append(f"{name} License\n\nCopyright (c) {holders}\n\n{PERMISSION[name]}")
        elif name == "Apache-2.0" and apache:
            chosen.append(f"Copyright {holders}\n\n{apache}")
        else:
            raise SystemExit(f"{package['name']} {package['version']} ships no licence text and there is none for '{licence_of(package)}'")
    return chosen


def own_notices(own: list[dict], output: Path) -> list[tuple[str, str]]:
    """The notices written by hand beside our own packages: (package, text). The ones this tool makes are left out."""
    found = []
    for package in own:
        path = Path(package["manifest_path"]).parent / NAME
        if path.is_file() and path.resolve() != output.resolve():
            text = clean(path.read_text(encoding="utf-8"))
            if MARKER not in text:
                found.append((package["name"], text))
    return found


def render(third: list[dict], extra: list[tuple[str, str]]) -> str:
    shipped = {p["id"]: crate_texts(p) for p in third}
    apache = next((t for texts in shipped.values() for t in texts if "TERMS AND CONDITIONS FOR USE, REPRODUCTION" in t and "Apache License" in t), None)
    used: dict[str, tuple[str, list[str]]] = {}
    for package in third:
        for text in shipped[package["id"]] or missing_texts(package, apache):
            used.setdefault(flat(text), (text, []))[1].append(f"{package['name']} {package['version']}")
    lines = ["Third-party notices", "===================", "", MARKER, "",
             "The program is built with the Rust crates below, fetched from crates.io. Each is used under the licence named beside it",
             "(where there is a choice, under any one of those offered); their copyright notices and licence texts follow. The",
             "program also holds the Rust standard library (MIT or Apache-2.0, copyright of The Rust Project contributors).", "",
             "Components", "----------", ""]
    lines += [f"{p['name']} | {p['version']} | {licence_of(p)}" for p in third]
    lines += ["", "Copyright notices and licence texts", "-----------------------------------"]
    for text, users in used.values():
        lines += ["", "=" * 78, "Used by: " + ", ".join(dict.fromkeys(users)), "=" * 78, "", text]
    if extra:
        lines += ["", "=" * 78, "Other notices", "=" * 78]
        for name, text in extra:
            lines += ["", f"--- {name} ---", "", text]
    return "\n".join(lines) + "\n"


def problems(written: str, third: list[dict], extra: list[tuple[str, str]]) -> list[str]:
    if MARKER not in written:
        return ["the file was not made by this tool"]
    listed: set[tuple[str, str]] = set()
    section = written.split("\nComponents\n", 1)[-1].split("\nCopyright notices and licence texts\n", 1)[0]
    for line in section.splitlines():
        parts = [p.strip() for p in line.split("|")]
        if len(parts) == 3:
            listed.add((parts[0], parts[2]))
    found = [f"not in the notices: {p['name']} {p['version']} ({licence_of(p)})" for p in third if (p["name"], licence_of(p)) not in listed]
    whole = flat(written)
    found += [f"the notices of {name} are not carried" for name, text in extra if flat(text) not in whole]
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path("."), help="the folder of the Cargo.toml (default: here)")
    parser.add_argument("--package", action="append", default=[], help="only what this member of the workspace needs (may be repeated)")
    parser.add_argument("--output", type=Path, help=f"the notices file (default: {NAME} in the root)")
    parser.add_argument("--check", action="store_true", help="write nothing; fail when the file does not cover the tree")
    args = parser.parse_args()
    root = args.root.resolve()
    output = (root / (args.output or NAME)).resolve()
    third, own = tree(root, args.package)
    extra = own_notices(own, output)
    if args.check:
        found = problems(output.read_text(encoding="utf-8"), third, extra) if output.is_file() else [f"{output.name} is missing"]
        for problem in found:
            print(f"- {problem}")
        print(f"THIRD_PARTY_NOTICES={'FAIL' if found else 'PASS'} ({len(third)} crates)")
        if found:
            print("Make the file again with tools/third_party_notices.py of EH3D-COMMON (same arguments without --check) and commit it.")
        return 1 if found else 0
    output.write_bytes(render(third, extra).encode("utf-8"))
    print(f"wrote {output} ({len(third)} crates, {output.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
