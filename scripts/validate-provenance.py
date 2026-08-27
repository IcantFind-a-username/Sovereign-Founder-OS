#!/usr/bin/env python3
"""Validate repository-local provenance invariants.

This intentionally checks only project policy invariants. Full CITATION.cff
schema validation is performed separately in CI.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


EXPECTED = {
    "title": "Sovereign Founder OS",
    "repository-code": "https://github.com/IcantFind-a-username/Sovereign-Founder-OS",
    "license": "Apache-2.0",
}
SNAPSHOT_VERSION = re.compile(r"^(\d{4})\.(\d{2})\.(\d{2})-provenance\.([1-9]\d*)$")
RELEASE_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def parse_top_level_cff(text: str) -> dict[str, str]:
    """Return unindented scalar CFF fields needed by this validator."""
    fields: dict[str, str] = {}
    for line in text.splitlines():
        if not line or line[0].isspace() or line.startswith("#"):
            continue
        match = re.match(r"^([a-z][a-z0-9-]*):\s*(.*?)\s*$", line)
        if match and match.group(2) not in {"", ">", ">-", "|", "|-"}:
            fields[match.group(1)] = match.group(2).strip("'\"")
    return fields


def validate_cff(root: Path) -> list[str]:
    path = root / "CITATION.cff"
    if not path.is_file():
        return ["CITATION.cff: missing"]
    text = path.read_text(encoding="utf-8")
    fields = parse_top_level_cff(text)
    errors = [
        f"CITATION.cff: {key} must be {expected!r}"
        for key, expected in EXPECTED.items()
        if fields.get(key) != expected
    ]
    if "family-names: Xu" not in text or "given-names: Franz" not in text:
        errors.append("CITATION.cff: author must identify Franz Xu")

    version = fields.get("version")
    released = fields.get("date-released")
    if bool(version) != bool(released):
        errors.append("CITATION.cff: version and date-released must appear together")
    elif version and released:
        match = SNAPSHOT_VERSION.fullmatch(version)
        if not match:
            errors.append("CITATION.cff: invalid provenance snapshot version")
        elif not RELEASE_DATE.fullmatch(released):
            errors.append("CITATION.cff: invalid date-released")
        elif released != f"{match.group(1)}-{match.group(2)}-{match.group(3)}":
            errors.append("CITATION.cff: snapshot version and release date disagree")
    return errors


def _read(root: Path, relative: str, errors: list[str]) -> str:
    path = root / relative
    if not path.is_file():
        errors.append(f"{relative}: missing")
        return ""
    return path.read_text(encoding="utf-8")


def validate_document_markers(root: Path) -> list[str]:
    errors: list[str] = []
    required = {
        "PROVENANCE.md": [
            "Franz Xu",
            "maintainer's representation",
            "abstract ideas",
            "No immutable provenance snapshot has been published",
        ],
        "NOTICE": ["Franz Xu", "PROVENANCE.md", "Apache License"],
        "README.md": ["PROVENANCE.md", "provenance record"],
        "CONTRIBUTING.md": ["Developer Certificate of Origin", "Signed-off-by"],
        "TRADEMARK.md": ["No registration is claimed"],
        "GOVERNANCE.md": ["SemVer product releases", "Provenance snapshots"],
        "SECURITY.md": ["Provenance snapshots"],
    }
    documents: dict[str, str] = {}
    for relative, markers in required.items():
        text = _read(root, relative, errors)
        documents[relative] = text
        for marker in markers:
            if marker not in text:
                errors.append(f"{relative}: missing required marker {marker!r}")

    forbidden = {
        "independently verified authorship",
        "copyright protects the ideas",
        "registered trademark",
        "original and proprietary",
    }
    for relative, text in documents.items():
        lowered = text.lower()
        for phrase in forbidden:
            if phrase in lowered:
                errors.append(f"{relative}: forbidden overclaim {phrase!r}")

    workflow = _read(root, ".github/workflows/release.yml", errors)
    for forbidden_workflow in ("contents: write", "gh release", "--clobber", 'tags: ["v*"]'):
        if forbidden_workflow in workflow:
            errors.append(f".github/workflows/release.yml: unsafe publisher contains {forbidden_workflow!r}")
    provenance_workflow = _read(root, ".github/workflows/provenance-dry-run.yml", errors)
    for marker in (
        "permissions:\n  contents: read",
        "github.event.repository.default_branch",
        "materialize-provenance-cargo-lock.py",
        "UNSIGNED-DRY-RUN",
        "sha256:2a2e837a2c8d59ec9af5472ee22d3b04ee463c4e44476ecf993fd1e5ab6ebc7f",
    ):
        if marker not in provenance_workflow:
            errors.append(
                ".github/workflows/provenance-dry-run.yml: "
                f"missing trust-boundary marker {marker!r}"
            )
    for forbidden_workflow in ("contents: write", "gh release", "git push", "--clobber"):
        if forbidden_workflow in provenance_workflow:
            errors.append(
                ".github/workflows/provenance-dry-run.yml: "
                f"unsafe operation contains {forbidden_workflow!r}"
            )

    workflow_directory = root / ".github/workflows"
    if workflow_directory.is_dir():
        for path in sorted(workflow_directory.glob("*.yml")):
            for line_number, line in enumerate(
                path.read_text(encoding="utf-8").splitlines(), start=1
            ):
                match = re.search(r"\buses:\s*[^\s@]+@([^\s#]+)", line)
                if match and not re.fullmatch(r"[0-9a-f]{40}", match.group(1)):
                    errors.append(
                        f"{path.relative_to(root)}:{line_number}: action is not pinned "
                        "to a full commit SHA"
                    )
    return errors


def validate_repository(root: Path) -> list[str]:
    return validate_cff(root) + validate_document_markers(root)


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    errors = validate_repository(root)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("Provenance metadata validation passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
