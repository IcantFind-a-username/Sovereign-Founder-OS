#!/usr/bin/env python3
"""Developer Preview release helpers (tag policy, staging, manifest)."""

from __future__ import annotations

import argparse
import json
import tarfile
import os
import re
import stat
import subprocess
import sys
from dataclasses import dataclass
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

from provenance_evidence import EvidenceError, run_git, sha256_file

DEVELOPER_PREVIEW_TAG = re.compile(
    r"^developer-preview-(?P<year>\d{4})\.(?P<month>\d{2})\.(?P<day>\d{2})\.(?P<n>[1-9]\d*)$"
)

FORBIDDEN_TAG_RULES: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"^v0\.1"), "product v0.1 family is forbidden"),
    (re.compile(r"^v"), "SemVer product tags (v*) are forbidden"),
    (re.compile(r"^provenance-"), "provenance snapshot family is forbidden"),
)

CLI_ARCHIVE = "sovereign-cli-x86_64-unknown-linux-gnu.tar.gz"
MANIFEST_NAME = "release-manifest.json"
CHECKSUMS_NAME = "SHA256SUMS"
VERIFY_NAME = "VERIFY.md"
NON_CLAIMS_RELATIVE = "docs/release/developer-preview-non-claims.txt"
VERIFY_SOURCE_RELATIVE = "docs/release/VERIFY-PREVIEW.md"
TRUST_ANCHOR_MARKER = "maintainer signing-key fingerprint"


@dataclass(frozen=True)
class ParsedDeveloperPreviewTag:
    name: str
    publication_date: date


class PreviewReleaseError(RuntimeError):
    """Raised when a Developer Preview release cannot be built or verified."""


def tag_rejection_reason(tag: str) -> str | None:
    for pattern, reason in FORBIDDEN_TAG_RULES:
        if pattern.search(tag):
            return reason
    if not DEVELOPER_PREVIEW_TAG.fullmatch(tag):
        return "tag must match developer-preview-YYYY.MM.DD.N"
    return None


def parse_developer_preview_tag(tag: str) -> ParsedDeveloperPreviewTag:
    rejection = tag_rejection_reason(tag)
    if rejection:
        raise PreviewReleaseError(rejection)
    match = DEVELOPER_PREVIEW_TAG.fullmatch(tag)
    assert match is not None
    year = int(match.group("year"))
    month = int(match.group("month"))
    day = int(match.group("day"))
    try:
        publication_date = date(year, month, day)
    except ValueError as error:
        raise PreviewReleaseError("tag date is not a valid UTC calendar day") from error
    return ParsedDeveloperPreviewTag(name=tag, publication_date=publication_date)


def publication_date_matches_utc_day(
    tag: ParsedDeveloperPreviewTag, as_of: date | None = None
) -> bool:
    today = as_of or datetime.now(timezone.utc).date()
    return tag.publication_date == today


def trust_anchor_published(repo_root: Path) -> bool:
    """True only when PROVENANCE.md documents a real, verifiable trust anchor."""
    path = repo_root / "PROVENANCE.md"
    if not path.is_file():
        return False
    text = path.read_text(encoding="utf-8")
    if "No immutable provenance snapshot has been published" in text:
        return False
    if "No placeholder fingerprint" in text:
        return False
    return TRUST_ANCHOR_MARKER in text and "BEGIN PGP PUBLIC KEY BLOCK" in text


def read_non_claims_block(repo_root: Path) -> str:
    path = repo_root / NON_CLAIMS_RELATIVE
    if not path.is_file():
        raise PreviewReleaseError(f"missing required non-claims block at {NON_CLAIMS_RELATIVE}")
    return path.read_text(encoding="utf-8").rstrip() + "\n"


def release_title(tag: ParsedDeveloperPreviewTag) -> str:
    return (
        f"Sovereign Founder OS — Developer Preview "
        f"{tag.publication_date.isoformat()} ({tag.name.split('.')[-1]})"
    )


def build_release_notes(
    repo_root: Path,
    *,
    tag: ParsedDeveloperPreviewTag,
    commit: str,
    tree: str,
    tag_object: str,
    workflow_run_url: str,
    maintainer_signature_attached: bool,
) -> str:
    non_claims = read_non_claims_block(repo_root)
    signature_line = (
        "Detached maintainer signature over release-manifest.json: attached."
        if maintainer_signature_attached
        else "Detached maintainer signature: omitted (no pinned trust anchor in PROVENANCE.md)."
    )
    provenance_notes = (
        f"## Provenance notes (Developer Preview)\n\n"
        f"- Tag: `{tag.name}` (object `{tag_object}`)\n"
        f"- Peeled commit: `{commit}`\n"
        f"- Tree: `{tree}`\n"
        f"- Workflow run: {workflow_run_url}\n"
        f"- Signed vs hashed: GitHub artifact attestations cover listed payload files, "
        f"`{MANIFEST_NAME}`, and `{CHECKSUMS_NAME}` when built on GitHub-hosted runners. "
        f"{signature_line}\n"
        f"- This is **not** a `provenance-*` snapshot and does not include a published "
        f"provenance evidence bundle.\n"
    )
    what_this_is = (
        "## What this build is\n\n"
        "Linux x86_64 `sovereign-cli` built from the annotated tag above after the same "
        "locked gates CI runs (fmt, clippy, tests, file-size, frontend type-check, "
        "dependency audit, secret scan). Developer Preview only — not a SemVer product "
        "release.\n"
    )
    return f"{non_claims}\n{provenance_notes}\n{what_this_is}"


def _require_regular_file(path: Path) -> None:
    try:
        mode = path.lstat().st_mode
    except OSError as error:
        raise PreviewReleaseError(f"{path.name} is unavailable") from error
    if path.is_symlink() or not stat.S_ISREG(mode):
        raise PreviewReleaseError(f"{path.name} must be a regular file")


def _asset_record(path: Path) -> dict[str, Any]:
    _require_regular_file(path)
    digest = sha256_file(path)
    return {"name": path.name, "sha256": digest, "size_bytes": path.stat().st_size}


def write_sha256sums(staging: Path, manifest_path: Path, payload_paths: list[Path]) -> Path:
    checksums_path = staging / CHECKSUMS_NAME
    if checksums_path.exists():
        raise PreviewReleaseError(f"refusing to overwrite existing {CHECKSUMS_NAME}")
    lines: list[str] = []
    for path in sorted(payload_paths, key=lambda item: item.name):
        lines.append(f"{sha256_file(path)}  {path.name}")
    lines.append(f"{sha256_file(manifest_path)}  {manifest_path.name}")
    checksums_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return checksums_path


def build_manifest(
    *,
    repository: str,
    tag_name: str,
    tag_object: str,
    commit: str,
    tree: str,
    rustc_version: str,
    host_triple: str,
    cargo_lock_sha256: str,
    workflow_run_url: str,
    payload_assets: list[dict[str, Any]],
) -> dict[str, Any]:
    sorted_assets = sorted(payload_assets, key=lambda item: item["name"])
    return {
        "publication_class": "developer-preview",
        "repository": repository,
        "tag": {"name": tag_name, "object": tag_object, "peeled_commit": commit},
        "commit": commit,
        "tree": tree,
        "toolchain": {"rustc": rustc_version, "host": host_triple},
        "cargo_lock_sha256": cargo_lock_sha256,
        "workflow_run_url": workflow_run_url,
        "assets": sorted_assets,
    }


def write_manifest(path: Path, manifest: dict[str, Any]) -> None:
    if path.exists():
        raise PreviewReleaseError(f"refusing to overwrite existing {path.name}")
    write_json(path, manifest)


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def verify_staging_directory(staging: Path) -> None:
    staging = staging.expanduser().absolute()
    if not staging.is_dir() or staging.is_symlink():
        raise PreviewReleaseError("staging directory must be a real directory")

    manifest_path = staging / MANIFEST_NAME
    checksums_path = staging / CHECKSUMS_NAME
    verify_path = staging / VERIFY_NAME
    archive_path = staging / CLI_ARCHIVE

    for required in (manifest_path, checksums_path, verify_path, archive_path):
        _require_regular_file(required)

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("publication_class") != "developer-preview":
        raise PreviewReleaseError("manifest publication_class must be developer-preview")

    expected_names = {asset["name"] for asset in manifest.get("assets", [])}
    if CLI_ARCHIVE not in expected_names:
        raise PreviewReleaseError("manifest must list the Linux CLI archive")

    on_disk = {
        path.name
        for path in staging.iterdir()
        if path.is_file() and not path.is_symlink()
    }
    allowed = expected_names | {MANIFEST_NAME, CHECKSUMS_NAME, VERIFY_NAME}
    extra = on_disk - allowed
    if extra:
        raise PreviewReleaseError(f"unmanifested staging files: {sorted(extra)}")
    missing = expected_names - on_disk
    if missing:
        raise PreviewReleaseError(f"missing manifest assets: {sorted(missing)}")

    for asset in manifest["assets"]:
        name = asset["name"]
        path = staging / name
        _require_regular_file(path)
        if sha256_file(path) != asset["sha256"]:
            raise PreviewReleaseError(f"checksum mismatch for {name}")
        if path.stat().st_size != asset["size_bytes"]:
            raise PreviewReleaseError(f"size mismatch for {name}")

    checksum_lines = checksums_path.read_text(encoding="utf-8").splitlines()
    checksum_names = [line.split("  ", 1)[1] for line in checksum_lines if line.strip()]
    if CHECKSUMS_NAME in checksum_names:
        raise PreviewReleaseError("SHA256SUMS must not list itself")
    if set(checksum_names) != expected_names | {MANIFEST_NAME}:
        raise PreviewReleaseError("SHA256SUMS entries do not match manifest payloads")

    for line in checksum_lines:
        if not line.strip():
            continue
        digest, name = line.split("  ", 1)
        target = staging / name
        _require_regular_file(target)
        if sha256_file(target) != digest:
            raise PreviewReleaseError(f"SHA256SUMS mismatch for {name}")


def assert_prerelease_flags(release_json: dict[str, Any]) -> None:
    if not release_json.get("prerelease"):
        raise PreviewReleaseError("GitHub Release must be marked prerelease")
    if release_json.get("draft"):
        raise PreviewReleaseError("publish step requires a non-draft release")
    if release_json.get("make_latest") or release_json.get("makeLatest"):
        raise PreviewReleaseError("GitHub Release must not be marked latest")


def assert_no_existing_release(release_json: dict[str, Any] | None) -> None:
    if release_json is not None:
        raise PreviewReleaseError("refusing double-publish: release already exists for tag")


def resolve_tag_metadata(repo: Path, tag_name: str) -> tuple[str, str, str]:
    ref = f"refs/tags/{tag_name}"
    object_type = run_git(repo, "cat-file", "-t", ref)
    if object_type != "tag":
        raise PreviewReleaseError("tag must be annotated (git tag -a)")
    tag_object = run_git(repo, "rev-parse", ref)
    commit = run_git(repo, "rev-parse", f"{ref}^{{commit}}")
    tree = run_git(repo, "rev-parse", f"{commit}^{{tree}}")
    return tag_object, commit, tree


def commit_reachable_from_main(repo: Path, commit: str, main_ref: str = "refs/remotes/origin/main") -> None:
    try:
        run_git(repo, "merge-base", "--is-ancestor", commit, main_ref)
    except EvidenceError as error:
        raise PreviewReleaseError(f"commit is not reachable from protected main: {error}") from error


def cargo_lock_sha256(repo: Path) -> str:
    path = repo / "Cargo.lock"
    _require_regular_file(path)
    return sha256_file(path)


def copy_verify_guide(repo: Path, staging: Path) -> None:
    source = repo / VERIFY_SOURCE_RELATIVE
    destination = staging / VERIFY_NAME
    if destination.exists():
        raise PreviewReleaseError(f"refusing to overwrite existing {VERIFY_NAME}")
    _require_regular_file(source)
    destination.write_bytes(source.read_bytes())


def stage_cli_archive(binary: Path, staging: Path) -> Path:
    archive_path = staging / CLI_ARCHIVE
    if archive_path.exists():
        raise PreviewReleaseError(f"refusing to overwrite existing {CLI_ARCHIVE}")
    _require_regular_file(binary)
    with tarfile.open(archive_path, "w:gz") as archive:
        archive.add(binary, arcname="sovereign")
    return archive_path


def cmd_print_title(args: argparse.Namespace) -> int:
    try:
        parsed = parse_developer_preview_tag(args.tag)
        print(release_title(parsed))
    except PreviewReleaseError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


def cmd_validate_tag(args: argparse.Namespace) -> int:
    try:
        parsed = parse_developer_preview_tag(args.tag)
        if args.require_utc_today and not publication_date_matches_utc_day(parsed):
            raise PreviewReleaseError("tag UTC publication date must match workflow UTC day")
    except PreviewReleaseError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"OK: {parsed.name}")
    return 0


def cmd_write_notes(args: argparse.Namespace) -> int:
    repo = Path(args.repository).resolve()
    parsed = parse_developer_preview_tag(args.tag)
    notes = build_release_notes(
        repo,
        tag=parsed,
        commit=args.commit,
        tree=args.tree,
        tag_object=args.tag_object,
        workflow_run_url=args.workflow_run_url,
        maintainer_signature_attached=False,
    )
    output = Path(args.output)
    if output.exists() and not args.allow_overwrite:
        print(f"ERROR: refusing to overwrite {output}", file=sys.stderr)
        return 1
    output.write_text(notes, encoding="utf-8")
    return 0


def cmd_verify_staging(args: argparse.Namespace) -> int:
    try:
        verify_staging_directory(Path(args.staging))
    except PreviewReleaseError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("Developer Preview staging verification passed.")
    return 0


def assemble_staging(
    repo: Path,
    staging: Path,
    *,
    tag_name: str,
    binary: Path,
    repository: str,
    rustc_version: str,
    host_triple: str,
    workflow_run_url: str,
) -> dict[str, Any]:
    staging.mkdir(parents=True, exist_ok=True)
    if any(staging.iterdir()):
        raise PreviewReleaseError("staging directory must be empty before assembly")
    parse_developer_preview_tag(tag_name)
    tag_object, commit, tree = resolve_tag_metadata(repo, tag_name)
    copy_verify_guide(repo, staging)
    archive_path = stage_cli_archive(binary, staging)
    payload_assets = [_asset_record(archive_path)]
    manifest = build_manifest(
        repository=repository,
        tag_name=tag_name,
        tag_object=tag_object,
        commit=commit,
        tree=tree,
        rustc_version=rustc_version,
        host_triple=host_triple,
        cargo_lock_sha256=cargo_lock_sha256(repo),
        workflow_run_url=workflow_run_url,
        payload_assets=payload_assets,
    )
    manifest_path = staging / MANIFEST_NAME
    write_manifest(manifest_path, manifest)
    write_sha256sums(staging, manifest_path, [archive_path])
    verify_staging_directory(staging)
    return manifest


def cmd_assemble(args: argparse.Namespace) -> int:
    try:
        assemble_staging(
            Path(args.repository).resolve(),
            Path(args.staging).resolve(),
            tag_name=args.tag,
            binary=Path(args.binary).resolve(),
            repository=args.github_repository,
            rustc_version=args.rustc_version,
            host_triple=args.host_triple,
            workflow_run_url=args.workflow_run_url,
        )
    except PreviewReleaseError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("Developer Preview staging assembly passed.")
    return 0


def cmd_assert_prerelease(args: argparse.Namespace) -> int:
    try:
        release = json.loads(Path(args.release_json).read_text(encoding="utf-8"))
        assert_prerelease_flags(release)
    except (PreviewReleaseError, json.JSONDecodeError, OSError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("GitHub Release prerelease flags OK.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser("validate-tag")
    validate.add_argument("--tag", required=True)
    validate.add_argument("--require-utc-today", action="store_true")
    validate.set_defaults(func=cmd_validate_tag)

    title = subparsers.add_parser("print-title")
    title.add_argument("--tag", required=True)
    title.set_defaults(func=cmd_print_title)

    notes = subparsers.add_parser("write-notes")
    notes.add_argument("--repository", default=".")
    notes.add_argument("--tag", required=True)
    notes.add_argument("--commit", required=True)
    notes.add_argument("--tree", required=True)
    notes.add_argument("--tag-object", required=True)
    notes.add_argument("--workflow-run-url", required=True)
    notes.add_argument("--output", required=True)
    notes.add_argument("--allow-overwrite", action="store_true")
    notes.set_defaults(func=cmd_write_notes)

    verify = subparsers.add_parser("verify-staging")
    verify.add_argument("--staging", required=True)
    verify.set_defaults(func=cmd_verify_staging)

    prerelease = subparsers.add_parser("assert-prerelease")
    prerelease.add_argument("--release-json", required=True)
    prerelease.set_defaults(func=cmd_assert_prerelease)

    assemble = subparsers.add_parser("assemble-staging")
    assemble.add_argument("--repository", default=".")
    assemble.add_argument("--staging", required=True)
    assemble.add_argument("--tag", required=True)
    assemble.add_argument("--binary", required=True)
    assemble.add_argument("--github-repository", required=True)
    assemble.add_argument("--rustc-version", required=True)
    assemble.add_argument("--host-triple", required=True)
    assemble.add_argument("--workflow-run-url", required=True)
    assemble.set_defaults(func=cmd_assemble)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
