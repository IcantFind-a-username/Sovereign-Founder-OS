#!/usr/bin/env python3
"""Finalize an unsigned dry-run evidence directory with acyclic hashes."""

from __future__ import annotations

import argparse
import os
import stat
import sys
from pathlib import Path

from provenance_evidence import (
    cargo_package_metadata,
    EvidenceError,
    SAFE_ID,
    read_json,
    require_regular_file,
    sha256_file,
    validate_cargo_lock_sbom,
    write_json,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--sbom", type=Path, required=True)
    parser.add_argument("--verify-guide", type=Path)
    parser.add_argument("--sbom-generator", required=True)
    parser.add_argument("--sbom-generator-version", required=True)
    parser.add_argument("--sbom-generator-source", required=True)
    return parser.parse_args()


def _copy_new(source: Path, destination: Path) -> None:
    require_regular_file(source)
    if os.path.lexists(destination):
        raise EvidenceError(f"refusing to overwrite existing {destination.name}")
    with source.open("rb") as incoming, destination.open("xb") as outgoing:
        while chunk := incoming.read(1024 * 1024):
            outgoing.write(chunk)


def finalize(args: argparse.Namespace) -> None:
    evidence = args.evidence_dir.expanduser().absolute()
    try:
        evidence_mode = evidence.lstat().st_mode
    except OSError as error:
        raise EvidenceError("evidence directory is unavailable") from error
    if not stat.S_ISDIR(evidence_mode) or evidence.is_symlink():
        raise EvidenceError("evidence directory must be a real directory")
    metadata = read_json(evidence / "build-metadata.json")
    if metadata.get("mode") != "dry-run" or metadata.get("publishable") is not False:
        raise EvidenceError("only explicitly non-publishable dry runs can be finalized")
    snapshot_id = metadata.get("snapshot_id")
    source_name = metadata.get("source_archive")
    bundle_name = metadata.get("git_bundle")
    if not all(isinstance(item, str) for item in (snapshot_id, source_name, bundle_name)):
        raise EvidenceError("build metadata is missing required names")
    if not SAFE_ID.fullmatch(snapshot_id):
        raise EvidenceError("build metadata contains an unsafe snapshot ID")
    builder_names = {source_name, bundle_name, "build-metadata.json"}
    if {path.name for path in evidence.iterdir()} != builder_names:
        raise EvidenceError("evidence directory contains unexpected pre-finalization entries")
    for name in builder_names:
        if Path(name).name != name:
            raise EvidenceError(f"required payload is missing or unsafe: {name}")
        require_regular_file(evidence / name)

    expected_packages = cargo_package_metadata(metadata.get("cargo_lock_packages"))
    materials = metadata.get("materials")
    cargo_material = materials.get("Cargo.lock") if isinstance(materials, dict) else None
    cargo_lock_sha256 = (
        cargo_material.get("sha256") if isinstance(cargo_material, dict) else None
    )
    if not isinstance(cargo_lock_sha256, str):
        raise EvidenceError("build metadata has no Cargo.lock material hash")

    sbom_source = args.sbom.expanduser().absolute()
    sbom_value = read_json(sbom_source)
    sbom_suffix, covered_package_count = validate_cargo_lock_sbom(
        sbom_value,
        expected_packages,
        cargo_lock_sha256,
        args.sbom_generator,
        args.sbom_generator_version,
    )
    sbom_name = f"{snapshot_id}.sbom.{sbom_suffix}"
    _copy_new(sbom_source, evidence / sbom_name)

    default_guide = Path(__file__).resolve().parent.parent / "docs/release/VERIFY.md"
    guide = (args.verify_guide or default_guide).expanduser().absolute()
    _copy_new(guide, evidence / "VERIFY.md")

    roles = {
        source_name: "normalized-source",
        bundle_name: "exact-ref-git-bundle",
        "build-metadata.json": "build-metadata",
        sbom_name: "cargo-lock-sbom",
        "VERIFY.md": "verification-guide",
    }
    assets = []
    for name in sorted(roles):
        path = evidence / name
        assets.append(
            {
                "name": name,
                "role": roles[name],
                "sha256": sha256_file(path),
                "size": path.stat().st_size,
            }
        )

    manifest = {
        "schema_version": 1,
        "snapshot_id": snapshot_id,
        "mode": "dry-run",
        "publishable": False,
        "repository_url": metadata.get("repository_url"),
        "ref": metadata.get("ref"),
        "ref_object": metadata.get("ref_object"),
        "commit": metadata.get("commit"),
        "tree": metadata.get("tree"),
        "materials": metadata.get("materials"),
        "build_context": {
            "tools": metadata.get("tools"),
            "compression": metadata.get("compression"),
            "workflow": metadata.get("workflow"),
        },
        "sbom": {
            "scope": "Cargo.lock dependency graph",
            "format": sbom_suffix,
            "generator": args.sbom_generator,
            "generator_version": args.sbom_generator_version,
            "generator_source": args.sbom_generator_source,
            "covered_package_count": covered_package_count,
            "cargo_lock_package_count": sum(expected_packages.values()),
        },
        "assets": assets,
    }
    manifest_path = evidence / "release-manifest.json"
    write_json(manifest_path, manifest, exclusive=True)

    checksummed = [asset["name"] for asset in assets] + [manifest_path.name]
    lines = [f"{sha256_file(evidence / name)}  {name}" for name in sorted(checksummed)]
    with (evidence / "SHA256SUMS").open("x", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


def main() -> int:
    args = parse_args()
    try:
        finalize(args)
    except (EvidenceError, OSError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"Finalized unsigned dry-run evidence in {args.evidence_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
