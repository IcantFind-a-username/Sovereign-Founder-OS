# Documentation Language Policy

## Strategy

| Layer | Language | Purpose |
| --- | --- | --- |
| Root docs (`README`, `VISION`, `ARCHITECTURE`, etc.) | **English** | International discovery, citations, contributors |
| Complete blueprint (`docs/zh/`) | **中文** | Full product and engineering specifications |
| Code, APIs, commits, issues | **English** | Professional, neutral, tooling-friendly |
| User-facing UI (future) | **Bilingual** | EN + 中文 |

## Rules

1. English docs are the **entry point** for the global open source community.
2. Chinese docs in `docs/zh/` are the **authoritative complete blueprint** — nothing is hidden.
3. Do not mix languages in the same file.
4. When a Chinese spec gains an English counterpart, link both ways in `docs/INDEX.md`.

## For Contributors

- Propose architectural changes in English (RFCs).
- Jurisdiction Packs may be written in the local language with English summaries.
