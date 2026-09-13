from __future__ import annotations

import importlib.util
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "developer_preview_release.py"
SPEC = importlib.util.spec_from_file_location("developer_preview_release", SCRIPT)
assert SPEC and SPEC.loader
preview = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = preview
SPEC.loader.exec_module(preview)


class DeveloperPreviewTagTests(unittest.TestCase):
    def test_accepts_valid_tag(self) -> None:
        parsed = preview.parse_developer_preview_tag("developer-preview-2026.09.20.1")
        self.assertEqual(parsed.publication_date, date(2026, 9, 20))

    def test_rejects_semver_tags(self) -> None:
        for tag in ("v0.1.0", "v0.1", "v0.2.0", "v1.0.0"):
            self.assertIsNotNone(preview.tag_rejection_reason(tag))

    def test_rejects_provenance_family(self) -> None:
        self.assertIn(
            "provenance",
            preview.tag_rejection_reason("provenance-2026.08.27.1") or "",
        )

    def test_rejects_invalid_preview_shapes(self) -> None:
        for tag in (
            "developer-preview-2026.9.20.1",
            "developer-preview-2026.09.20.01",
            "developer-preview-2026.09.20.0",
        ):
            self.assertIsNotNone(preview.tag_rejection_reason(tag))
        with self.assertRaises(preview.PreviewReleaseError):
            preview.parse_developer_preview_tag("developer-preview-2026.13.40.1")

    def test_utc_same_day_rule(self) -> None:
        parsed = preview.parse_developer_preview_tag("developer-preview-2026.09.20.1")
        self.assertTrue(
            preview.publication_date_matches_utc_day(parsed, as_of=date(2026, 9, 20))
        )
        self.assertFalse(
            preview.publication_date_matches_utc_day(parsed, as_of=date(2026, 9, 21))
        )


class DeveloperPreviewStagingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        self.git("init")
        self.git("config", "user.email", "preview@example.com")
        self.git("config", "user.name", "Preview Tester")
        (self.repo / "Cargo.lock").write_text('[metadata]\n', encoding="utf-8")
        (self.repo / "docs" / "release").mkdir(parents=True)
        shutil.copy(
            Path(__file__).parents[2] / "docs/release/VERIFY-PREVIEW.md",
            self.repo / "docs/release/VERIFY-PREVIEW.md",
        )
        shutil.copy(
            Path(__file__).parents[2] / "docs/release/developer-preview-non-claims.txt",
            self.repo / "docs/release/developer-preview-non-claims.txt",
        )
        self.git("add", ".")
        self.git("commit", "-m", "fixture")
        self.git("tag", "-a", "developer-preview-2026.09.20.1", "-m", "preview")
        self.binary = self.base / "sovereign"
        self.binary.write_bytes(b"\x7fELF-preview-binary\n")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=self.repo,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        return result.stdout.strip()

    def assemble(self) -> Path:
        staging = self.base / "staging"
        preview.assemble_staging(
            self.repo,
            staging,
            tag_name="developer-preview-2026.09.20.1",
            binary=self.binary,
            repository="example/repo",
            rustc_version="1.97.0",
            host_triple="x86_64-unknown-linux-gnu",
            workflow_run_url="https://example.com/run/1",
        )
        return staging

    def test_staging_verifies(self) -> None:
        staging = self.assemble()
        preview.verify_staging_directory(staging)
        manifest = json.loads((staging / "release-manifest.json").read_text())
        self.assertEqual(manifest["publication_class"], "developer-preview")
        self.assertNotIn("SHA256SUMS", {a["name"] for a in manifest["assets"]})

    def test_missing_asset_fails(self) -> None:
        staging = self.assemble()
        (staging / preview.CLI_ARCHIVE).unlink()
        with self.assertRaises(preview.PreviewReleaseError):
            preview.verify_staging_directory(staging)

    def test_hash_mismatch_fails(self) -> None:
        staging = self.assemble()
        path = staging / preview.CLI_ARCHIVE
        with path.open("ab") as handle:
            handle.write(b"tamper")
        with self.assertRaises(preview.PreviewReleaseError):
            preview.verify_staging_directory(staging)

    def test_prerelease_flags_required(self) -> None:
        with self.assertRaises(preview.PreviewReleaseError):
            preview.assert_prerelease_flags({"prerelease": False, "draft": False})
        with self.assertRaises(preview.PreviewReleaseError):
            preview.assert_prerelease_flags({"prerelease": True, "draft": True})
        with self.assertRaises(preview.PreviewReleaseError):
            preview.assert_prerelease_flags(
                {"prerelease": True, "draft": False, "make_latest": True}
            )
        preview.assert_prerelease_flags({"prerelease": True, "draft": False})

    def test_double_publish_guard(self) -> None:
        with self.assertRaises(preview.PreviewReleaseError):
            preview.assert_no_existing_release({"id": 1})
