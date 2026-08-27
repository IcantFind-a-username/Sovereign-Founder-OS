#!/usr/bin/env python3
"""Verify an unsigned provenance dry-run against its repository ref."""

from __future__ import annotations

import argparse
import stat
import sys
import tempfile
from pathlib import Path

from provenance_evidence import (
    cargo_package_metadata,
    EvidenceError,
    SAFE_ID,
    normalized_source_archive,
    parse_cargo_lock,
    read_git_blob,
    read_json,
    require_regular_file,
    run_git,
    sha256_bytes,
    sha256_file,
    validate_cargo_lock_sbom,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path, required=True)
    return parser.parse_args()


def _verify_checksums(evidence: Path) -> set[str]:
    checksum_path = evidence / "SHA256SUMS"
    require_regular_file(checksum_path)
    lines = checksum_path.read_text(encoding="utf-8").splitlines()
    seen: set[str] = set()
    for line in lines:
        if "  " not in line:
            raise EvidenceError("malformed SHA256SUMS line")
        expected, name = line.split("  ", 1)
        if Path(name).name != name or name == "SHA256SUMS" or name in seen:
            raise EvidenceError(f"unsafe or duplicate checksum entry: {name}")
        path = evidence / name
        require_regular_file(path)
        if sha256_file(path) != expected:
            raise EvidenceError(f"checksum mismatch: {name}")
        seen.add(name)
    return seen


def verify(args: argparse.Namespace) -> None:
    evidence = args.evidence_dir.expanduser().absolute()
    try:
        evidence_mode = evidence.lstat().st_mode
    except OSError as error:
        raise EvidenceError("evidence directory is unavailable") from error
    if not stat.S_ISDIR(evidence_mode) or evidence.is_symlink():
        raise EvidenceError("evidence directory must be a real directory")
    checksum_names = _verify_checksums(evidence)
    manifest = read_json(evidence / "release-manifest.json")
    metadata = read_json(evidence / "build-metadata.json")
    for label, document in (("manifest", manifest), ("build metadata", metadata)):
        schema_version = document.get("schema_version")
        if type(schema_version) is not int or schema_version != 1:
            raise EvidenceError(f"{label} must use evidence schema version 1")
        if document.get("mode") != "dry-run" or document.get("publishable") is not False:
            raise EvidenceError(f"{label} must describe a non-publishable dry-run")

    assets = manifest.get("assets")
    if not isinstance(assets, list):
        raise EvidenceError("manifest assets must be a list")
    expected_names: set[str] = set()
    for asset in assets:
        if (
            not isinstance(asset, dict)
            or not isinstance(asset.get("name"), str)
            or not isinstance(asset.get("role"), str)
        ):
            raise EvidenceError("malformed manifest asset")
        name = asset["name"]
        if Path(name).name != name or name in expected_names:
            raise EvidenceError(f"unsafe or duplicate manifest asset: {name}")
        path = evidence / name
        require_regular_file(path)
        if sha256_file(path) != asset.get("sha256") or path.stat().st_size != asset.get("size"):
            raise EvidenceError(f"manifest payload mismatch: {name}")
        expected_names.add(name)

    required_checksums = expected_names | {"release-manifest.json"}
    if checksum_names != required_checksums:
        raise EvidenceError("SHA256SUMS entries do not exactly match the manifest")
    actual_names = {path.name for path in evidence.iterdir()}
    permitted_names = required_checksums | {"SHA256SUMS"}
    if actual_names != permitted_names:
        raise EvidenceError("evidence directory contains an unmanifested or missing file")

    ref = manifest.get("ref")
    ref_object = manifest.get("ref_object")
    commit = manifest.get("commit")
    tree = manifest.get("tree")
    if not all(isinstance(item, str) for item in (ref, ref_object, commit, tree)):
        raise EvidenceError("manifest ref identity is incomplete")
    for key in (
        "snapshot_id",
        "repository_url",
        "ref",
        "ref_object",
        "commit",
        "tree",
        "materials",
    ):
        if metadata.get(key) != manifest.get(key):
            raise EvidenceError(f"build metadata disagrees on {key}")
    expected_build_context = {
        "tools": metadata.get("tools"),
        "compression": metadata.get("compression"),
        "workflow": metadata.get("workflow"),
    }
    if manifest.get("build_context") != expected_build_context:
        raise EvidenceError("manifest build context disagrees with build metadata")
    sbom_context = manifest.get("sbom")
    if not isinstance(sbom_context, dict) or sbom_context.get("scope") != (
        "Cargo.lock dependency graph"
    ):
        raise EvidenceError("manifest SBOM context is incomplete")

    bundle_name = metadata.get("git_bundle")
    source_name = metadata.get("source_archive")
    if not isinstance(bundle_name, str) or not isinstance(source_name, str):
        raise EvidenceError("build metadata payload names are invalid")
    snapshot_id = manifest.get("snapshot_id")
    if not isinstance(snapshot_id, str) or not SAFE_ID.fullmatch(snapshot_id):
        raise EvidenceError("manifest snapshot ID is unsafe")
    expected_source_name = f"{snapshot_id}-source.tar.gz"
    expected_bundle_name = f"{snapshot_id}.bundle"
    sbom_format = sbom_context.get("format")
    if sbom_format != "spdx.json":
        raise EvidenceError("manifest SBOM format is unsupported")
    expected_sbom_name = f"{snapshot_id}.sbom.{sbom_format}"
    if source_name != expected_source_name:
        raise EvidenceError("build metadata source archive name is unsafe")
    if bundle_name != expected_bundle_name:
        raise EvidenceError("build metadata Git bundle name is unsafe")
    expected_roles = {
        expected_source_name: "normalized-source",
        expected_bundle_name: "exact-ref-git-bundle",
        "build-metadata.json": "build-metadata",
        expected_sbom_name: "cargo-lock-sbom",
        "VERIFY.md": "verification-guide",
    }
    actual_roles = {asset["name"]: asset["role"] for asset in assets}
    if actual_roles != expected_roles:
        raise EvidenceError("manifest asset names or roles do not match the evidence schema")
    with tempfile.TemporaryDirectory() as temporary:
        imported = Path(temporary) / "imported.git"
        imported.mkdir()
        run_git(imported, "init", "--bare", ".")
        if not ref.startswith("refs/") or any(char.isspace() for char in ref):
            raise EvidenceError("manifest ref must be an explicit full ref under refs/")
        run_git(imported, "check-ref-format", ref)
        bundle = evidence / bundle_name
        run_git(imported, "bundle", "verify", str(bundle))
        heads = run_git(imported, "bundle", "list-heads", str(bundle)).splitlines()
        if heads != [f"{ref_object} {ref}"]:
            raise EvidenceError("Git bundle advertised ref does not match the manifest")
        imported_ref = "refs/evidence/imported"
        run_git(imported, "fetch", "--no-tags", str(bundle), f"{ref}:{imported_ref}")
        run_git(imported, "fsck", "--full", "--strict")
        actual_ref_object = run_git(imported, "rev-parse", "--verify", imported_ref)
        actual_commit = run_git(
            imported, "rev-parse", "--verify", f"{imported_ref}^{{commit}}"
        )
        actual_tree = run_git(imported, "rev-parse", "--verify", f"{actual_commit}^{{tree}}")
        if (actual_ref_object, actual_commit, actual_tree) != (ref_object, commit, tree):
            raise EvidenceError("imported bundle identity does not match the manifest")

        materials = manifest.get("materials")
        if not isinstance(materials, dict):
            raise EvidenceError("manifest materials are missing")
        material_contents: dict[str, bytes] = {}
        for name in ("Cargo.lock", "CITATION.cff"):
            material = materials.get(name)
            if not isinstance(material, dict):
                raise EvidenceError(f"manifest material is missing: {name}")
            blob_id, content = read_git_blob(imported, f"{commit}:{name}")
            material_contents[name] = content
            if (
                material.get("git_blob") != blob_id
                or material.get("sha256") != sha256_bytes(content)
                or material.get("size") != len(content)
            ):
                raise EvidenceError(f"manifest material mismatch: {name}")

        cargo_packages = parse_cargo_lock(material_contents["Cargo.lock"])
        if cargo_package_metadata(metadata.get("cargo_lock_packages")) != cargo_packages:
            raise EvidenceError(
                "build metadata Cargo.lock package graph disagrees with the bundled blob"
            )
        generator = sbom_context.get("generator")
        generator_version = sbom_context.get("generator_version")
        generator_source = sbom_context.get("generator_source")
        if not all(
            isinstance(item, str) and item
            for item in (generator, generator_version, generator_source)
        ):
            raise EvidenceError("manifest SBOM generator context is incomplete")
        sbom_value = read_json(evidence / expected_sbom_name)
        actual_format, covered_package_count = validate_cargo_lock_sbom(
            sbom_value,
            cargo_packages,
            sha256_bytes(material_contents["Cargo.lock"]),
            generator,
            generator_version,
        )
        expected_package_count = sum(cargo_packages.values())
        if actual_format != sbom_format:
            raise EvidenceError("manifest SBOM format disagrees with the SBOM payload")
        if (
            sbom_context.get("covered_package_count") != covered_package_count
            or sbom_context.get("cargo_lock_package_count") != expected_package_count
        ):
            raise EvidenceError("manifest SBOM package counts are inconsistent")

        rebuilt = Path(temporary) / "rebuilt-source.tar.gz"
        normalized_source_archive(imported, commit, rebuilt, str(manifest["snapshot_id"]))
        if sha256_file(rebuilt) != sha256_file(evidence / source_name):
            raise EvidenceError("source archive is not the normalized archive for the ref")


def main() -> int:
    args = parse_args()
    try:
        verify(args)
    except (EvidenceError, OSError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("Unsigned dry-run evidence verification passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
