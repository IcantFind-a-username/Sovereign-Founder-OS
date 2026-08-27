#!/usr/bin/env python3
"""Build deterministic source and history evidence for one explicit ref."""

from __future__ import annotations

import argparse
import platform
import shutil
import sys
import zlib
from pathlib import Path

from provenance_evidence import (
    EvidenceError,
    SAFE_ID,
    exact_ref_bundle,
    normalized_source_archive,
    parse_cargo_lock,
    read_git_blob,
    resolve_ref,
    run_git,
    sha256_bytes,
    write_json,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--ref", required=True)
    parser.add_argument("--snapshot-id", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--mode", choices=("dry-run", "publishable"), required=True)
    parser.add_argument("--repository-url")
    parser.add_argument("--workflow-ref", default="local")
    parser.add_argument("--workflow-run-url", default="local")
    return parser.parse_args()


def build(args: argparse.Namespace) -> None:
    root = args.repository.resolve()
    output = args.output_dir.resolve()
    if args.mode != "dry-run":
        raise EvidenceError(
            "publishable mode is locked until a maintainer trust anchor is configured"
        )
    if not SAFE_ID.fullmatch(args.snapshot_id):
        raise EvidenceError("invalid snapshot ID")
    if output.is_symlink():
        raise EvidenceError("output directory must not be a symlink")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise EvidenceError("output directory must be absent or empty")
    output.mkdir(parents=True, exist_ok=True)

    ref_object, commit, tree = resolve_ref(root, args.ref)
    repository_url = args.repository_url
    if not repository_url:
        repository_url = run_git(root, "remote", "get-url", "origin")
    materials: dict[str, dict[str, object]] = {}
    material_bytes: dict[str, bytes] = {}
    for name in ("Cargo.lock", "CITATION.cff"):
        blob_id, content = read_git_blob(root, f"{commit}:{name}")
        material_bytes[name] = content
        materials[name] = {
            "git_blob": blob_id,
            "sha256": sha256_bytes(content),
            "size": len(content),
        }
    package_counts = parse_cargo_lock(material_bytes["Cargo.lock"])
    cargo_packages = [
        {"name": name, "version": version}
        for (name, version), count in sorted(package_counts.items())
        for _ in range(count)
    ]
    source_name = f"{args.snapshot_id}-source.tar.gz"
    bundle_name = f"{args.snapshot_id}.bundle"
    normalized_source_archive(root, args.ref, output / source_name, args.snapshot_id)
    exact_ref_bundle(root, args.ref, output / bundle_name)

    write_json(
        output / "build-metadata.json",
        {
            "schema_version": 1,
            "snapshot_id": args.snapshot_id,
            "mode": "dry-run",
            "publishable": False,
            "ref": args.ref,
            "ref_object": ref_object,
            "commit": commit,
            "tree": tree,
            "repository_url": repository_url,
            "source_archive": source_name,
            "git_bundle": bundle_name,
            "materials": materials,
            "cargo_lock_packages": cargo_packages,
            "workflow": {
                "ref": args.workflow_ref,
                "run_url": args.workflow_run_url,
            },
            "tools": {
                "git": run_git(root, "--version"),
                "python": platform.python_version(),
                "zlib_compile": zlib.ZLIB_VERSION,
                "zlib_runtime": zlib.ZLIB_RUNTIME_VERSION,
            },
            "compression": {
                "algorithm": "gzip",
                "level": 9,
                "filename": "",
                "mtime": 0,
            },
        },
        exclusive=True,
    )


def main() -> int:
    args = parse_args()
    try:
        build(args)
    except (EvidenceError, OSError, shutil.Error) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"Built unsigned dry-run evidence in {args.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
