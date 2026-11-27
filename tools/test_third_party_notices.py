"""Tests of tools/third_party_notices.py with made-up crates (no cargo, no network)."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import third_party_notices as notices  # noqa: E402

APACHE = "Apache License\n\nTERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION\nsample text"


class NoticesTest(unittest.TestCase):
    def setUp(self) -> None:
        self.folder = tempfile.TemporaryDirectory()
        self.addCleanup(self.folder.cleanup)

    def crate(self, name: str, licence: str, files: dict[str, str]) -> dict:
        root = Path(self.folder.name) / name
        root.mkdir()
        (root / "Cargo.toml").write_text("", encoding="utf-8")
        for file, text in files.items():
            (root / file).parent.mkdir(parents=True, exist_ok=True)
            (root / file).write_text(text, encoding="utf-8")
        return {"id": name, "name": name, "version": "1.0.0", "license": licence, "authors": ["Ann <ann@example.org>"],
                "manifest_path": str(root / "Cargo.toml")}

    def test_copyright_lines_and_notices_are_copied(self) -> None:
        crate = self.crate("alpha", "MIT OR Apache-2.0", {"LICENSE-MIT": "Copyright (c) 2021 Alpha\nterms", "NOTICE": "Alpha notice",
                                                          "LICENSE.rs": "code", "tests/LICENSE": "not shipped"})
        written = notices.render([crate], [])
        self.assertIn("Copyright (c) 2021 Alpha", written)
        self.assertIn("Alpha notice", written)
        self.assertNotIn("not shipped", written)
        self.assertEqual(notices.problems(written, [crate], []), [])

    def test_a_crate_with_no_text_gets_the_text_of_its_licence(self) -> None:
        giver = self.crate("giver", "Apache-2.0", {"LICENSE-APACHE": APACHE})
        bare = self.crate("bare", "MIT", {})
        only_apache = self.crate("other", "Apache-2.0", {})
        written = notices.render([bare, giver, only_apache], [])
        self.assertIn("Copyright (c) Ann\n", written)
        self.assertIn("Copyright Ann\n\nApache License", written)

    def test_a_public_domain_crate_with_no_text_is_named(self) -> None:
        written = notices.render([self.crate("free", "CC0-1.0", {})], [])
        self.assertIn("free: dedicated to the public domain", written)

    def test_a_licence_with_no_known_text_stops_the_tool(self) -> None:
        with self.assertRaises(SystemExit):
            notices.render([self.crate("odd", "MPL-2.0", {})], [])

    def test_a_new_dependency_or_a_changed_licence_fails_and_a_new_version_does_not(self) -> None:
        old = self.crate("alpha", "MIT", {"LICENSE": "Copyright (c) Alpha"})
        written = notices.render([old], [("data", "Data notice\nof others")])
        self.assertEqual(notices.problems(written, [dict(old, version="9.9.9")], [("data", "Data notice of others")]), [])
        self.assertEqual(len(notices.problems(written, [old, self.crate("beta", "MIT", {})], [])), 1)
        self.assertEqual(len(notices.problems(written, [dict(old, license="BSD-3-Clause")], [])), 1)
        self.assertEqual(len(notices.problems(written, [old], [("viewer", "PDF library")])), 1)
        self.assertEqual(len(notices.problems("written by hand", [old], [])), 1)


if __name__ == "__main__":
    unittest.main()
