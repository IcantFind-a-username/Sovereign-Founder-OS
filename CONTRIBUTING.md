# Contributing

Thank you for your interest in Sovereign Founder OS. This project welcomes contributions from developers, security researchers, privacy engineers, and domain experts.

## Before You Start

1. Read [VISION.md](VISION.md) and [ARCHITECTURE.md](ARCHITECTURE.md)
2. Review [THREAT_MODEL.md](THREAT_MODEL.md) for security constraints
3. Check [ROADMAP.md](ROADMAP.md) for current stage and priorities
4. Look at open issues before starting new work

## How to Contribute

### Code

1. Fork the repository
2. Create a feature branch from `main`
3. Make focused changes with tests
4. Ensure CI passes (when configured)
5. Open a pull request with a clear description

### Documentation

Documentation improvements are valuable, especially:
- Threat model refinements
- Architecture RFCs
- Jurisdiction pack specifications
- Security test cases

### Security Research

We actively welcome:
- Adversarial test cases
- Chaos engineering scenarios
- Agent Security Gauntlet benchmark contributions
- Threat model reviews

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## Development Setup

Setup instructions will be added when Stage 1 (Secure Kernel) code lands. Planned stack:

- Rust (secure core)
- TypeScript + React + Tauri (desktop UI)
- Python (isolated agent workers)

## Pull Request Guidelines

- One logical change per PR
- Include tests for security-relevant behavior
- No secrets, API keys, or credentials in commits
- Follow existing code style and naming conventions
- Reference related issues when applicable

## Security-Critical Changes

Changes to vault, policy, capability, sandbox, identity, or audit-ledger require:

- Threat model impact assessment in PR description
- Adversarial test coverage
- Dual review (once team is established)

## RFC Process

Significant architectural changes go through the `rfcs/` directory:

1. Open a draft RFC as a PR
2. Community discussion (minimum 7 days for substantial changes)
3. Maintainer acceptance or rejection with rationale
4. Implementation PR references accepted RFC

## Code of Conduct

All contributors must follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE).

## Questions

Open a GitHub Discussion or Issue for questions not covered here.
