#!/usr/bin/env python3
"""Materialize the exact Cargo.lock blob recorded by evidence metadata."""

from __future__ import annotations

import argparse
import os
import re
import stat
import sys
from pathlib import Path

from provenance_evidence import (
    EvidenceError,
    read_git_blob,
    read_json,
    sha256_bytes,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def materialize(args: argparse.Namespace) -> None:
    repository = args.repository.resolve()
    metadata = read_json(args.metadata.expanduser().absolute())
    commit = metadata.get("commit")
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", commit):
        raise EvidenceError("build metadata commit is not a full Git object ID")
    materials = metadata.get("materials")
    cargo_material = materials.get("Cargo.lock") if isinstance(materials, dict) else None
    if not isinstance(cargo_material, dict):
        raise EvidenceError("build metadata has no Cargo.lock material identity")

    blob_id, content = read_git_blob(repository, f"{commit}:Cargo.lock")
    if (
        cargo_material.get("git_blob") != blob_id
        or cargo_material.get("sha256") != sha256_bytes(content)
        or cargo_material.get("size") != len(content)
    ):
        raise EvidenceError("recorded Cargo.lock material does not match the Git blob")

    output = args.output.expanduser().absolute()
    if output.name != "Cargo.lock":
        raise EvidenceError("materialized output must be named Cargo.lock")
    output.parent.mkdir(parents=True, exist_ok=True)
    parent_mode = output.parent.lstat().st_mode
    if not stat.S_ISDIR(parent_mode) or output.parent.is_symlink():
        raise EvidenceError("materialized output parent must be a real directory")
    if os.path.lexists(output):
        raise EvidenceError("refusing to overwrite materialized Cargo.lock")
    with output.open("xb") as handle:
        handle.write(content)


def main() -> int:
    args = parse_args()
    try:
        materialize(args)
    except (EvidenceError, OSError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"Materialized recorded Cargo.lock at {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
