# Vision

## What We Are Building

**Sovereign Founder OS** is a local-first, model-neutral, jurisdiction-aware, cryptographically protected, and distributively recoverable AI operating system for one-person companies, independent developers, small consultancies, and Micro-SaaS founders.

It is not a chatbot with business features bolted on. It is an operating system that helps ordinary people run a real company — with AI agents that operate inside strict permission boundaries, under user-controlled data and keys.

## Brand Promise

> **Your company. Your data. Your keys.**
> **Many models. No master. No single point of failure.**

中文：

> **你的公司、你的数据、你的密钥。**
> **依赖多种智能，但不受制于任何单一节点。**

## The Problem

High-privilege AI agents are increasingly touching email, code, contracts, financial data, and customer records. When a single model provider, cloud vendor, or platform fails — or revokes access — a business built on that dependency can stop overnight.

Most AI tools optimize for capability first. Sovereign Founder OS optimizes for **continuity, ownership, and verifiable control** first.

## Two-Layer Product Structure

### Layer 1: Sovereign Agent Runtime

A reusable secure runtime that other AI projects can adopt:

- Model routing and failover
- Agent permission control
- Tool sandboxing
- Local encrypted storage
- Distributed replication
- Workflow recovery
- Cryptographic audit
- Plugin supply chain verification
- Data privacy boundaries
- Automated security response

### Layer 2: Founder OS

A reference application built on the Runtime, proving the system works for real business operations:

- Enterprise digital twin
- Venture planning and task orchestration
- Dynamic AI crew composition
- Legal and tax assistance
- Customer, product, and marketing management
- Security operations center
- Asset management

**Development order: Runtime Alpha first, full Founder OS second.**

## Core Design Principles

### 1. State Over Conversation

The system maintains a **Sovereign Enterprise Graph** — a structured digital twin of the company — not a pile of chat logs. Every AI action must change enterprise state, produce verifiable artifacts, and leave an auditable record.

### 2. Mutually Constrained Autonomy

No single agent or component may simultaneously: decide goals, grant permissions, access full data, execute actions, delete records, and approve results.

Roles are separated: Planner, Policy Guard, Executor, Auditor, Recovery Controller, Human Owner.

> No node can initiate a sensitive action, approve it, and destroy the evidence.

### 3. Sovereignty

Users own their data, keys, model choices, asset controls, and business continuity. Failure of any single AI company, cloud vendor, server, or platform must not directly halt the company.

### 4. Progressive Disclosure

The system is internally complex. The user interface hides that complexity. Users see what their company needs next — not agent parameters, token counts, or tool schemas.

### 5. Verifiable Security

Security claims are proven through public threat models, adversarial tests, and reproducible chaos experiments — not marketing adjectives.

## What Success Looks Like

A stranger can:

1. Create a one-person consulting company in the system
2. Run a real customer onboarding workflow
3. Survive model provider failure without data loss
4. Block a malicious plugin and prompt injection
5. Approve high-risk actions with clear natural-language explanations
6. Export all company data and recovery keys without official servers

The defining demo:

> **We killed the model, the server and the plugin. The company kept running.**

## What We Are Not

- Not a global AI lawyer or automated tax filing system (in early versions)
- Not a blockchain database for personal data
- Not a multi-agent chat room
- Not dependent on a single model, cloud, or official server
- Not "absolutely secure" — we pursue verifiable, local-first, server-blind security

## Target Users (v1)

- Digital service providers
- Independent consultants
- Micro-SaaS founders

Initial jurisdiction focus: **Singapore** one-person digital service companies. Global expansion via versioned Jurisdiction Packs.

## Open Source Philosophy

Core local runtime capabilities are fully usable without payment. Commercial services (encrypted sync, verified jurisdiction packs, professional networks, managed recovery nodes) must not compromise the security or sovereignty of the free core.

License: **Apache 2.0**. See [LICENSE](LICENSE) and [TRADEMARK.md](TRADEMARK.md).

## Further Reading

| Document | Description |
| --- | --- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System architecture and trust boundaries |
| [THREAT_MODEL.md](THREAT_MODEL.md) | Threat model v0.1 |
| [PRIVACY_MODEL.md](PRIVACY_MODEL.md) | Data classification and privacy design |
| [ROADMAP.md](ROADMAP.md) | Development stages and milestones |
| [docs/INDEX.md](docs/INDEX.md) | Full documentation map |
| [docs/zh/](docs/zh/) | Complete Chinese design specifications |
