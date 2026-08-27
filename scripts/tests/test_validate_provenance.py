from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "validate-provenance.py"
SPEC = importlib.util.spec_from_file_location("validate_provenance", SCRIPT)
assert SPEC and SPEC.loader
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class ProvenanceValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.write_valid_fixture()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def write_valid_fixture(self) -> None:
        self.write(
            "CITATION.cff",
            """cff-version: 1.2.0
title: Sovereign Founder OS
authors:
  - family-names: Xu
    given-names: Franz
repository-code: https://github.com/IcantFind-a-username/Sovereign-Founder-OS
license: Apache-2.0
""",
        )
        self.write(
            "PROVENANCE.md",
            "Franz Xu maintainer's representation; abstract ideas. "
            "No immutable provenance snapshot has been published",
        )
        self.write("NOTICE", "Franz Xu PROVENANCE.md Apache License, Version 2.0")
        self.write("README.md", "PROVENANCE.md provenance record")
        self.write("CONTRIBUTING.md", "Developer Certificate of Origin Signed-off-by")
        self.write("TRADEMARK.md", "No registration is claimed")
        self.write("GOVERNANCE.md", "SemVer product releases Provenance snapshots")
        self.write("SECURITY.md", "Provenance snapshots")
        self.write(".github/workflows/release.yml", "on: workflow_dispatch\npermissions: {}\n")
        self.write(
            ".github/workflows/provenance-dry-run.yml",
            """on: workflow_dispatch
permissions:
  contents: read
jobs:
  evidence:
    steps:
      - uses: actions/example@0123456789abcdef0123456789abcdef01234567
      - run: echo UNSIGNED-DRY-RUN
      - run: echo github.event.repository.default_branch
      - run: python3 scripts/materialize-provenance-cargo-lock.py
      - run: echo sha256:2a2e837a2c8d59ec9af5472ee22d3b04ee463c4e44476ecf993fd1e5ab6ebc7f
""",
        )

    def test_undated_metadata_is_valid_before_candidate_freeze(self) -> None:
        self.assertEqual([], validator.validate_repository(self.root))

    def test_candidate_fields_must_appear_together(self) -> None:
        with (self.root / "CITATION.cff").open("a", encoding="utf-8") as handle:
            handle.write("version: 2026.08.27-provenance.1\n")
        self.assertIn(
            "CITATION.cff: version and date-released must appear together",
            validator.validate_cff(self.root),
        )

    def test_candidate_version_and_date_must_agree(self) -> None:
        with (self.root / "CITATION.cff").open("a", encoding="utf-8") as handle:
            handle.write(
                "version: 2026.08.27-provenance.1\n"
                "date-released: 2026-08-28\n"
            )
        self.assertIn(
            "CITATION.cff: snapshot version and release date disagree",
            validator.validate_cff(self.root),
        )

    def test_well_formed_candidate_is_valid(self) -> None:
        with (self.root / "CITATION.cff").open("a", encoding="utf-8") as handle:
            handle.write(
                "version: 2026.08.27-provenance.2\n"
                "date-released: 2026-08-27\n"
            )
        self.assertEqual([], validator.validate_cff(self.root))

    def test_missing_policy_marker_is_rejected(self) -> None:
        self.write("TRADEMARK.md", "Trademark policy")
        self.assertTrue(
            any("TRADEMARK.md: missing required marker" in error for error in validator.validate_repository(self.root))
        )

    def test_absolute_idea_claim_is_rejected(self) -> None:
        with (self.root / "README.md").open("a", encoding="utf-8") as handle:
            handle.write("\nCopyright protects the ideas.\n")
        self.assertTrue(
            any("forbidden overclaim" in error for error in validator.validate_repository(self.root))
        )

    def test_unsafe_release_publisher_is_rejected(self) -> None:
        self.write(".github/workflows/release.yml", "permissions:\n  contents: write\n")
        self.assertTrue(
            any("unsafe publisher" in error for error in validator.validate_repository(self.root))
        )

    def test_movable_action_tag_is_rejected(self) -> None:
        workflow = self.root / ".github/workflows/provenance-dry-run.yml"
        workflow.write_text(
            workflow.read_text(encoding="utf-8").replace(
                "actions/example@0123456789abcdef0123456789abcdef01234567",
                "actions/example@v7",
            ),
            encoding="utf-8",
        )
        self.assertTrue(
            any("not pinned" in error for error in validator.validate_repository(self.root))
        )

    def test_workflow_must_materialize_the_recorded_cargo_lock(self) -> None:
        workflow = self.root / ".github/workflows/provenance-dry-run.yml"
        workflow.write_text(
            workflow.read_text(encoding="utf-8").replace(
                "python3 scripts/materialize-provenance-cargo-lock.py",
                "echo scan mutable Cargo.lock",
            ),
            encoding="utf-8",
        )

        self.assertTrue(
            any(
                "materialize-provenance-cargo-lock.py" in error
                for error in validator.validate_repository(self.root)
            )
        )


if __name__ == "__main__":
    unittest.main()
