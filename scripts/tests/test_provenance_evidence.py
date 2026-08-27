from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]
BUILD = ROOT / "scripts/build-provenance-evidence.py"
FINALIZE = ROOT / "scripts/finalize-provenance-evidence.py"
MATERIALIZE = ROOT / "scripts/materialize-provenance-cargo-lock.py"
VERIFY = ROOT / "scripts/verify-provenance-evidence.py"


class ProvenanceEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        self.repository = self.base / "repository"
        self.repository.mkdir()
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Fixture Author")
        self.git("config", "user.email", "fixture@example.invalid")
        (self.repository / "README.md").write_text("fixture\n", encoding="utf-8")
        (self.repository / "CITATION.cff").write_text(
            "cff-version: 1.2.0\ntitle: Fixture\n", encoding="utf-8"
        )
        cargo_lock = (
            'version = 3\n\n[[package]]\nname = "fixture-dependency"\nversion = "1.2.3"\n'
        )
        (self.repository / "Cargo.lock").write_text(cargo_lock, encoding="utf-8")
        self.git("add", "README.md", "CITATION.cff", "Cargo.lock")
        environment = os.environ.copy()
        environment.update(
            {
                "GIT_AUTHOR_DATE": "2026-01-02T03:04:05Z",
                "GIT_COMMITTER_DATE": "2026-01-02T03:04:05Z",
            }
        )
        self.git("commit", "-m", "initial", environment=environment)
        self.git("remote", "add", "origin", "https://example.invalid/fixture.git")
        self.sbom = self.base / "sbom.json"
        self.sbom.write_text(
            json.dumps(
                {
                    "spdxVersion": "SPDX-2.3",
                    "creationInfo": {"creators": ["Tool: syft-1.0.0"]},
                    "packages": [
                        {"name": "fixture-dependency", "versionInfo": "1.2.3"},
                        {
                            "name": "Cargo.lock",
                            "SPDXID": "SPDXRef-DocumentRoot-File-Cargo.lock",
                            "versionInfo": (
                                "sha256:"
                                + hashlib.sha256(cargo_lock.encode("utf-8")).hexdigest()
                            ),
                            "checksums": [
                                {
                                    "algorithm": "SHA256",
                                    "checksumValue": hashlib.sha256(
                                        cargo_lock.encode("utf-8")
                                    ).hexdigest(),
                                }
                            ],
                            "primaryPackagePurpose": "FILE",
                        },
                    ],
                }
            )
            + "\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def git(self, *args: str, environment: dict[str, str] | None = None) -> str:
        return subprocess.run(
            ["git", *args],
            cwd=self.repository,
            env=environment,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        ).stdout.strip()

    def command(self, script: Path, *args: str, succeeds: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            ["python3", str(script), *args],
            cwd=ROOT,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if succeeds and result.returncode:
            self.fail(f"command failed: {result.stderr}")
        if not succeeds and result.returncode == 0:
            self.fail("command unexpectedly succeeded")
        return result

    def build(
        self,
        destination: Path,
        mode: str = "dry-run",
        ref: str = "refs/heads/main",
    ) -> subprocess.CompletedProcess[str]:
        return self.command(
            BUILD,
            "--repository",
            str(self.repository),
            "--ref",
            ref,
            "--snapshot-id",
            "fixture-dry-run",
            "--output-dir",
            str(destination),
            "--mode",
            mode,
            succeeds=mode == "dry-run",
        )

    def finalize(
        self, destination: Path, succeeds: bool = True
    ) -> subprocess.CompletedProcess[str]:
        return self.command(
            FINALIZE,
            "--evidence-dir",
            str(destination),
            "--sbom",
            str(self.sbom),
            "--sbom-generator",
            "syft",
            "--sbom-generator-version",
            "1.0.0",
            "--sbom-generator-source",
            "fixture-generator",
            succeeds=succeeds,
        )

    def verify(self, destination: Path, succeeds: bool = True) -> subprocess.CompletedProcess[str]:
        return self.command(
            VERIFY,
            "--evidence-dir",
            str(destination),
            succeeds=succeeds,
        )

    def rehash_evidence(self, destination: Path) -> None:
        manifest_path = destination / "release-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        for asset in manifest["assets"]:
            payload = destination / asset["name"]
            asset["sha256"] = hashlib.sha256(payload.read_bytes()).hexdigest()
            asset["size"] = payload.stat().st_size
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        names = [asset["name"] for asset in manifest["assets"]] + [manifest_path.name]
        checksums = [
            f"{hashlib.sha256((destination / name).read_bytes()).hexdigest()}  {name}"
            for name in sorted(names)
        ]
        (destination / "SHA256SUMS").write_text(
            "\n".join(checksums) + "\n", encoding="utf-8"
        )

    def test_source_archive_is_reproducible(self) -> None:
        first = self.base / "first"
        second = self.base / "second"
        self.build(first)
        self.build(second)
        self.assertEqual(
            (first / "fixture-dry-run-source.tar.gz").read_bytes(),
            (second / "fixture-dry-run-source.tar.gz").read_bytes(),
        )

    def test_materialized_cargo_lock_comes_from_the_recorded_commit(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        expected = (self.repository / "Cargo.lock").read_bytes()
        (self.repository / "Cargo.lock").write_text(
            'version = 3\n\n[[package]]\nname = "dirty-worktree"\nversion = "9.9.9"\n',
            encoding="utf-8",
        )
        materialized = self.base / "material" / "Cargo.lock"

        self.command(
            MATERIALIZE,
            "--repository",
            str(self.repository),
            "--metadata",
            str(destination / "build-metadata.json"),
            "--output",
            str(materialized),
        )

        self.assertEqual(expected, materialized.read_bytes())

    def test_bundle_contains_exact_requested_ref(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        heads = self.git("bundle", "list-heads", str(destination / "fixture-dry-run.bundle"))
        self.assertEqual(f"{self.git('rev-parse', 'HEAD')} refs/heads/main", heads)

    def test_annotated_tag_preserves_tag_object_and_peels_to_commit(self) -> None:
        self.git("tag", "--annotate", "fixture-tag", "--message", "fixture tag")
        destination = self.base / "tag-evidence"
        self.build(destination, ref="refs/tags/fixture-tag")
        self.finalize(destination)
        manifest = json.loads((destination / "release-manifest.json").read_text())
        self.assertNotEqual(manifest["ref_object"], manifest["commit"])
        self.verify(destination)

    def test_finalize_writes_acyclic_sorted_hashes(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        manifest = json.loads((destination / "release-manifest.json").read_text())
        self.assertFalse(manifest["publishable"])
        asset_names = [asset["name"] for asset in manifest["assets"]]
        self.assertEqual(sorted(asset_names), asset_names)
        checksum_names = [
            line.split("  ", 1)[1]
            for line in (destination / "SHA256SUMS").read_text().splitlines()
        ]
        self.assertNotIn("SHA256SUMS", checksum_names)
        self.assertIn("release-manifest.json", checksum_names)
        self.verify(destination)

    def test_tampered_payload_is_rejected(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        with (destination / "fixture-dry-run-source.tar.gz").open("ab") as handle:
            handle.write(b"tampered")
        result = self.verify(destination, succeeds=False)
        self.assertIn("checksum mismatch", result.stderr)

    def test_missing_payload_is_rejected(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        (destination / "fixture-dry-run.bundle").unlink()
        result = self.verify(destination, succeeds=False)
        self.assertIn("unavailable", result.stderr)

    def test_unmanifested_payload_is_rejected(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        (destination / "unmanifested.txt").write_text("not evidence\n", encoding="utf-8")
        result = self.verify(destination, succeeds=False)
        self.assertIn("unmanifested", result.stderr)

    def test_verification_is_standalone_after_source_repository_is_removed(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        shutil.rmtree(self.repository)
        self.verify(destination)

    def test_verifier_rejects_unsafe_source_path_without_touching_it(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        outside = self.base / "outside-source.tar.gz"
        sentinel = b"must not be overwritten\n"
        outside.write_bytes(sentinel)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["source_archive"] = str(outside)
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = subprocess.run(
            ["python3", str(VERIFY), "--evidence-dir", str(destination)],
            cwd=ROOT,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertEqual(sentinel, outside.read_bytes())
        self.assertNotEqual(0, result.returncode)

    def test_verifier_rejects_bundle_path_outside_evidence_directory(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        outside = self.base / "outside.bundle"
        shutil.copyfile(destination / "fixture-dry-run.bundle", outside)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["git_bundle"] = str(outside)
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("bundle", result.stderr)

    def test_verifier_rejects_manifest_role_substitution(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        manifest_path = destination / "release-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        roles = {asset["name"]: asset["role"] for asset in manifest["assets"]}
        source_name = "fixture-dry-run-source.tar.gz"
        bundle_name = "fixture-dry-run.bundle"
        for asset in manifest["assets"]:
            if asset["name"] == source_name:
                asset["role"] = roles[bundle_name]
            elif asset["name"] == bundle_name:
                asset["role"] = roles[source_name]
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("role", result.stderr)

    def test_verifier_rejects_symlinked_evidence_asset(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        source = destination / "fixture-dry-run-source.tar.gz"
        outside = self.base / "outside-source.tar.gz"
        shutil.copyfile(source, outside)
        source.unlink()
        source.symlink_to(outside)

        result = self.verify(destination, succeeds=False)

        self.assertIn("regular non-symlink", result.stderr)

    def test_verifier_validates_manifest_ref_before_using_the_bundle(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        unsafe_ref = "refs/heads/main^{commit}"
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["ref"] = unsafe_ref
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        manifest_path = destination / "release-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["ref"] = unsafe_ref
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("check-ref-format", result.stderr)

    def test_unsafe_snapshot_id_in_mutated_metadata_is_rejected(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text())
        metadata["snapshot_id"] = "../escape"
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        result = self.command(
            FINALIZE,
            "--evidence-dir",
            str(destination),
            "--sbom",
            str(self.sbom),
            "--sbom-generator",
            "syft",
            "--sbom-generator-version",
            "1.0.0",
            "--sbom-generator-source",
            "fixture-generator",
            succeeds=False,
        )
        self.assertIn("unsafe snapshot ID", result.stderr)

    def test_symlinked_builder_payload_is_rejected_before_copy(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        source = destination / "fixture-dry-run-source.tar.gz"
        source.unlink()
        source.symlink_to(self.sbom)
        result = self.finalize(destination, succeeds=False)
        self.assertIn("regular non-symlink", result.stderr)

    def test_revision_expression_is_not_accepted_as_a_ref(self) -> None:
        result = self.command(
            BUILD,
            "--repository",
            str(self.repository),
            "--ref",
            "refs/heads/main^{commit}",
            "--snapshot-id",
            "fixture-dry-run",
            "--output-dir",
            str(self.base / "invalid-ref"),
            "--mode",
            "dry-run",
            succeeds=False,
        )
        self.assertIn("check-ref-format", result.stderr)

    def test_incomplete_sbom_is_rejected(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.sbom.write_text(
            json.dumps(
                {
                    "spdxVersion": "SPDX-2.3",
                    "creationInfo": {"creators": ["Tool: syft-1.0.0"]},
                    "packages": [],
                }
            ),
            encoding="utf-8",
        )
        result = self.command(
            FINALIZE,
            "--evidence-dir",
            str(destination),
            "--sbom",
            str(self.sbom),
            "--sbom-generator",
            "syft",
            "--sbom-generator-version",
            "1.0.0",
            "--sbom-generator-source",
            "fixture-generator",
            succeeds=False,
        )
        self.assertIn("Cargo.lock", result.stderr)

    def test_sbom_with_unrelated_package_is_rejected(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        sbom = json.loads(self.sbom.read_text(encoding="utf-8"))
        sbom["packages"].append({"name": "unrelated", "versionInfo": "9.9.9"})
        self.sbom.write_text(json.dumps(sbom) + "\n", encoding="utf-8")

        result = self.finalize(destination, succeeds=False)

        self.assertIn("unrelated", result.stderr)

    def test_sbom_root_must_bind_the_recorded_cargo_lock_hash(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        sbom = json.loads(self.sbom.read_text(encoding="utf-8"))
        root = next(package for package in sbom["packages"] if package["name"] == "Cargo.lock")
        root["versionInfo"] = "sha256:" + "0" * 64
        root["checksums"][0]["checksumValue"] = "0" * 64
        self.sbom.write_text(json.dumps(sbom) + "\n", encoding="utf-8")

        result = self.finalize(destination, succeeds=False)

        self.assertIn("Cargo.lock", result.stderr)

    def test_sbom_root_rejects_conflicting_cargo_lock_hashes(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        sbom = json.loads(self.sbom.read_text(encoding="utf-8"))
        root = next(package for package in sbom["packages"] if package["name"] == "Cargo.lock")
        root["checksums"].append(
            {"algorithm": "SHA256", "checksumValue": "0" * 64}
        )
        self.sbom.write_text(json.dumps(sbom) + "\n", encoding="utf-8")

        result = self.finalize(destination, succeeds=False)

        self.assertIn("Cargo.lock", result.stderr)

    def test_verifier_revalidates_sbom_against_bundled_cargo_lock(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        sbom_path = destination / "fixture-dry-run.sbom.spdx.json"
        sbom = json.loads(sbom_path.read_text(encoding="utf-8"))
        sbom["packages"] = [
            package for package in sbom["packages"] if package["name"] != "fixture-dependency"
        ]
        sbom_path.write_text(json.dumps(sbom) + "\n", encoding="utf-8")
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("Cargo.lock", result.stderr)

    def test_verifier_rejects_sbom_generator_version_substitution(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        manifest_path = destination / "release-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["sbom"]["generator_version"] = "9.9.9"
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("generator", result.stderr)

    def test_verifier_rejects_metadata_package_graph_substitution(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["cargo_lock_packages"] = []
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("package graph", result.stderr)

    def test_verifier_rejects_publishable_build_metadata(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["mode"] = "publishable"
        metadata["publishable"] = True
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("dry-run", result.stderr)

    def test_verifier_rejects_unknown_evidence_schema_versions(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["schema_version"] = 999
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        manifest_path = destination / "release-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["schema_version"] = 999
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("schema", result.stderr)

    def test_verifier_rejects_boolean_evidence_schema_versions(self) -> None:
        destination = self.base / "evidence"
        self.build(destination)
        self.finalize(destination)
        metadata_path = destination / "build-metadata.json"
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        metadata["schema_version"] = True
        metadata_path.write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        manifest_path = destination / "release-manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["schema_version"] = True
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        self.rehash_evidence(destination)

        result = self.verify(destination, succeeds=False)

        self.assertIn("schema", result.stderr)

    def test_publishable_mode_is_locked_without_trust_anchor(self) -> None:
        result = self.build(self.base / "evidence", mode="publishable")
        self.assertIn("trust anchor", result.stderr)


if __name__ == "__main__":
    unittest.main()
