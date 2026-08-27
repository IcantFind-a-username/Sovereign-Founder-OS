#!/usr/bin/env python3
"""Shared primitives for deterministic provenance evidence."""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import re
import stat
import subprocess
import tomllib
from collections import Counter
from pathlib import Path


SAFE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")


class EvidenceError(RuntimeError):
    """Raised when evidence cannot be built or verified safely."""


def _git_environment() -> dict[str, str]:
    environment = os.environ.copy()
    environment.update({"LC_ALL": "C", "LANG": "C", "TZ": "UTC"})
    return environment


def run_git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=root,
        env=_git_environment(),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise EvidenceError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout.strip()


def resolve_ref(root: Path, ref: str) -> tuple[str, str, str]:
    if not ref.startswith("refs/") or any(char.isspace() for char in ref):
        raise EvidenceError("ref must be an explicit full ref under refs/")
    run_git(root, "check-ref-format", ref)
    run_git(root, "show-ref", "--verify", ref)
    ref_object = run_git(root, "rev-parse", "--verify", ref)
    commit = run_git(root, "rev-parse", "--verify", f"{ref}^{{commit}}")
    tree = run_git(root, "rev-parse", "--verify", f"{commit}^{{tree}}")
    return ref_object, commit, tree


def normalized_source_archive(
    root: Path, ref: str, destination: Path, prefix: str
) -> None:
    if not SAFE_ID.fullmatch(prefix):
        raise EvidenceError("snapshot ID is not safe for an archive prefix")
    archive = subprocess.Popen(
        ["git", "archive", "--format=tar", f"--prefix={prefix}/", ref],
        cwd=root,
        env=_git_environment(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert archive.stdout is not None
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        with destination.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as zipped:
                while chunk := archive.stdout.read(1024 * 1024):
                    zipped.write(chunk)
        stderr = archive.communicate()[1].decode("utf-8", errors="replace").strip()
        if archive.returncode:
            destination.unlink(missing_ok=True)
            raise EvidenceError(f"git archive failed: {stderr}")
    finally:
        if archive.poll() is None:
            archive.kill()
            archive.wait()


def exact_ref_bundle(root: Path, ref: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    run_git(root, "bundle", "create", str(destination), ref)
    heads = run_git(root, "bundle", "list-heads", str(destination)).splitlines()
    expected_suffix = f" {ref}"
    if len(heads) != 1 or not heads[0].endswith(expected_suffix):
        destination.unlink(missing_ok=True)
        raise EvidenceError("bundle did not contain exactly the requested ref")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_git_blob(root: Path, object_spec: str) -> tuple[str, bytes]:
    blob_id = run_git(root, "rev-parse", "--verify", object_spec)
    if run_git(root, "cat-file", "-t", blob_id) != "blob":
        raise EvidenceError(f"Git object is not a blob: {object_spec}")
    result = subprocess.run(
        ["git", "cat-file", "blob", blob_id],
        cwd=root,
        env=_git_environment(),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        raise EvidenceError(
            f"cannot read Git blob {object_spec}: "
            f"{result.stderr.decode('utf-8', errors='replace').strip()}"
        )
    return blob_id, result.stdout


def parse_cargo_lock(content: bytes) -> Counter[tuple[str, str]]:
    try:
        value = tomllib.loads(content.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise EvidenceError(f"cannot parse Cargo.lock package graph: {error}") from error
    packages = value.get("package")
    if not isinstance(packages, list) or not packages:
        raise EvidenceError("Cargo.lock package graph is empty or malformed")
    identities: Counter[tuple[str, str]] = Counter()
    for package in packages:
        if not isinstance(package, dict):
            raise EvidenceError("Cargo.lock contains malformed package metadata")
        name, version = package.get("name"), package.get("version")
        if not isinstance(name, str) or not name or not isinstance(version, str) or not version:
            raise EvidenceError("Cargo.lock contains a malformed package identity")
        identities[(name, version)] += 1
    return identities


def cargo_package_metadata(value: object) -> Counter[tuple[str, str]]:
    if not isinstance(value, list) or not value:
        raise EvidenceError("build metadata has no Cargo.lock package graph")
    identities: Counter[tuple[str, str]] = Counter()
    for package in value:
        if not isinstance(package, dict):
            raise EvidenceError("malformed Cargo.lock package metadata")
        name, version = package.get("name"), package.get("version")
        if not isinstance(name, str) or not name or not isinstance(version, str) or not version:
            raise EvidenceError("malformed Cargo.lock package identity")
        identities[(name, version)] += 1
    return identities


def _require_exact_packages(
    discovered: Counter[tuple[str, str]], expected: Counter[tuple[str, str]]
) -> None:
    missing = expected - discovered
    unexpected = discovered - expected
    if not missing and not unexpected:
        return
    details: list[str] = []
    if missing:
        examples = ", ".join(
            f"{name}@{version}"
            for (name, version), _count in sorted(missing.items())[:5]
        )
        details.append(f"missing {examples}")
    if unexpected:
        examples = ", ".join(
            f"{name}@{version}"
            for (name, version), _count in sorted(unexpected.items())[:5]
        )
        details.append(f"unexpected {examples}")
    raise EvidenceError(
        "SBOM does not cover Cargo.lock package graph exactly: " + "; ".join(details)
    )


def _validate_spdx_root(package: dict[str, object], cargo_lock_sha256: str) -> None:
    expected_version = f"sha256:{cargo_lock_sha256}"
    checksums = package.get("checksums")
    sha256_checksums = (
        [
            checksum.get("checksumValue")
            for checksum in checksums
            if isinstance(checksum, dict) and checksum.get("algorithm") == "SHA256"
        ]
        if isinstance(checksums, list)
        else []
    )
    if (
        package.get("name") != "Cargo.lock"
        or package.get("versionInfo") != expected_version
        or sha256_checksums != [cargo_lock_sha256]
    ):
        raise EvidenceError("SPDX SBOM Cargo.lock document root has the wrong hash")


def validate_cargo_lock_sbom(
    value: dict[str, object],
    expected_packages: Counter[tuple[str, str]],
    cargo_lock_sha256: str,
    generator: str,
    generator_version: str,
) -> tuple[str, int]:
    if generator != "syft" or not generator_version:
        raise EvidenceError("SBOM generator must identify the pinned Syft version")
    expected_tool = f"Tool: syft-{generator_version}"
    discovered: Counter[tuple[str, str]] = Counter()

    if value.get("spdxVersion") != "SPDX-2.3":
        raise EvidenceError("SBOM must be Syft SPDX 2.3 JSON")
    packages = value.get("packages")
    if not isinstance(packages, list):
        raise EvidenceError("SPDX SBOM packages must be a list")
    roots = 0
    for package in packages:
        if not isinstance(package, dict):
            raise EvidenceError("SPDX SBOM contains malformed package metadata")
        if package.get("primaryPackagePurpose") == "FILE":
            roots += 1
            _validate_spdx_root(package, cargo_lock_sha256)
            continue
        name, version = package.get("name"), package.get("versionInfo")
        if not isinstance(name, str) or not name or not isinstance(version, str) or not version:
            raise EvidenceError("SPDX SBOM contains a malformed package identity")
        discovered[(name, version)] += 1
    if roots != 1:
        raise EvidenceError("SPDX SBOM must contain exactly one Cargo.lock document root")
    creation_info = value.get("creationInfo")
    creators = creation_info.get("creators") if isinstance(creation_info, dict) else None
    tools = (
        [
            creator
            for creator in creators
            if isinstance(creator, str) and creator.startswith("Tool: ")
        ]
        if isinstance(creators, list)
        else []
    )
    if tools != [expected_tool]:
        raise EvidenceError("SPDX SBOM generator identity or version is inconsistent")

    _require_exact_packages(discovered, expected_packages)
    return "spdx.json", sum(discovered.values())


def require_regular_file(path: Path) -> None:
    try:
        mode = path.lstat().st_mode
    except OSError as error:
        raise EvidenceError(f"required file is unavailable: {path.name}") from error
    if not stat.S_ISREG(mode) or path.is_symlink():
        raise EvidenceError(f"required file is not a regular non-symlink: {path.name}")


def write_json(path: Path, value: object, *, exclusive: bool = False) -> None:
    mode = "x" if exclusive else "w"
    with path.open(mode, encoding="utf-8") as handle:
        handle.write(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n")


def read_json(path: Path) -> dict[str, object]:
    require_regular_file(path)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"cannot read JSON {path.name}: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError(f"{path.name} must contain a JSON object")
    return value
