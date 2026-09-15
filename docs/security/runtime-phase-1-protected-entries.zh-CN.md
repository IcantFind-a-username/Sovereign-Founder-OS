# Runtime Phase 1 — 受保护入口清单（产品 MVP 快照）

日期：2026-09-16。事实基线：`b6dc508e`。状态：**当前行为记录**；用于 Phase 1 指南 §4 第 0 步与 RP1-01，不构成资格声明。

本表列出**今天**可通过 CLI、本地 UI HTTP、或 `Store` 直接触达、并能创建/消费身份类密钥、签署批准、写入 outbox、或改写 authority 状态的路径。Fixture（RFC 0006）与 `legacy-experimental` 配置文件另表；此处仅覆盖默认 `sovereign ui` 产品 MVP 路径。

## 汇总

| 区域 | 可信主体（声称） | 实际认证 | 可触达的密钥/状态 | 失败语义 | 已知旁路/缺口 |
| --- | --- | --- | --- | --- | --- |
| 本地 UI HTTP | 浏览器中的“创始人” | **无** session/WebAuthn；仅 loopback + Host 检查 | 经 `Store` 间接读写 `vault/`、`authority/`、`outbox/`、`ledger/` | 4xx/JSON `ok:false`；workspace 错误字符串 | **未认证 POST 即可批准发送** → app-local owner signer |
| `Store::decide` | 同上 | 进程锁（单写者）；无 owner 证明 | 完整 RFC 0003 链 + outbox | 失败保持 `Pending`；锁 冲突报错 | 任何能 POST 的本地进程等价于 owner |
| `Store::request_send` | 同上 | 无 | 创建 pending approval | 校验文档状态 | 与 decide 相同信任模型 |
| `Store::revoke_delivery` | 同上 | 无 | 删除 outbox `.eml`；不撤销 authority bundle | 非待交付状态拒绝 | 不写入 `revoked-*`；与 RFC 0003 持久撤销不同 |
| Vault `owner_*` 密钥 | “设备上的创始人” | 首次发送时由 `owner_secret` 自动创建 | `vault/vault.key` + 命名条目 | 存储错误 fail-closed | 原型：vault 密钥本地文件，非 RFC 0005 ActiveV2 |
| Admission 信任 | 审计读者 | `admission_trust` 只读；GET 可能加载 owner admission 公钥 | 只读验证 | 缺 key 则无法验证 owner 签名链 | 仍含 demo 固定 admission 锚 |

## HTTP（`apps/cli/src/ui.rs` + `ui_mvp.rs`）

监听：loopback（`sovereign ui` 指定端口）。DNS rebinding：`Host` 必须匹配绑定端口。

| 方法 | 路径 | 调用的 workspace / 其他 | 安全相关效果 |
| --- | --- | --- | --- |
| POST | `/api/workspace/request-send` | `Store::request_send` | 创建待批准发送 |
| POST | `/api/workspace/decide` | `Store::decide` | **批准** → 签署、发 capability、沙箱执行、**写 outbox**、审计 |
| POST | `/api/workspace/revoke` | `Store::revoke_delivery` | 删除 outbox 文件（非 authority revoke） |
| POST | `/api/workspace/confirm-delivery` | 状态提交 | 业务状态，不新发效果 |
| POST | `/api/workspace/venture` / `customer` / `offer` / `invoice` | ERP 图 | 业务数据 + 审计 |
| POST | `/api/workspace/assist` | 草稿辅助 | 模型网关（自报 data class） |
| POST | `/api/gauntlet` | 演示 | 非产品发送链 |
| GET | `/api/workspace` | `workspace_get` | 读 workspace（含审批状态） |
| GET | `/api/outbox/*` | 读 outbox 文件 | 读 `.eml` |
| GET | `/api/export` | 导出 | 读业务快照 |
| POST | `/api/privacy/*` | `ui_mvp` 隐私编译 | RFC 0004 相关；非 outbox |

**诚实旁路：** 上述 POST **不需要** cookie、CSRF 或 WebAuthn。在默认部署下，同 UID 的任意本地进程或能访问 loopback 的页面均可调用 `decide(approve=true)`，效果等同于创始人点击批准。

## Workspace 发送链（`kernel_exec.rs` / `send_workflow.rs`）

批准路径（`decide` → `run_durable_send`）在持有进程锁时顺序执行：

1. **策略** — `evaluate_delivery_policy`（确定性）。
2. **Owner 批准证据** — `sign_owner_approval`：从 `vault` 读取/创建 `owner_approval_key`（`TypedSigner<ApprovalRole>`）。
3. **Admission** — `owner_admission_key` 签署 manifest admission。
4. **Capability** — `runtime_authority_key` 签发 `CapabilityTokenV2`。
5. **执行** — `AuthorityStore::consume_bundle`（经 validator）+ Wasm 沙箱 + `ExecutionJournal`。
6. **Outbox** — `OutboxBroker::write_message`（Amber 类 `.eml`）。
7. **状态** — 审计优先 `commit`。

密钥文件权力：运行 `sovereign` 的 OS 用户对 `--root` 下全部目录有读写权；无 UID/容器隔离。

## Authority / 撤销

| 入口 | 主体 | 效果 |
| --- | --- | --- |
| `consume_bundle`（仅经 capability 验证器，产品发送链内） | 已签发 token + approval | 消费 token/approval/idempotency；写 `bundles/*.committed` |
| `AuthorityStore::revoke_*`（Rust API） | 调用方 | 写 `revoked-tokens/` / `revoked-approvals/` |
| `Store::revoke_delivery` | 未认证的 HTTP/调用方 | **仅**删 outbox；**不**调用 `revoke_*` |

产品 MVP **没有** HTTP 暴露 RFC 0003 持久撤销；与发送链上的 bundle 撤销竞争由 authority store 与测试覆盖，见 F04 屏障测试。

## CLI / 其他

| 入口 | 说明 |
| --- | --- |
| `sovereign ui` | 启动上述 HTTP 服务 |
| `sovereign demo` | 独立 demo 密钥与 admission 锚（见 `admission_trust`） |
| `sovereign verify-export` | 只读验证导出文件 |
| Desktop（`apps/desktop`） | 当前为 Tauri 壳；核心仍指向 CLI/UI 能力时需按同一清单审查 |

## 与 Phase 1 目标的关系

- **RP1-01** 要求消除“未认证 loopback → app-local owner signer”类旁路，并换成可证明的 owner admission（1C0）与密钥隔离（1C1/1D）。
- 本清单 **不** 声称已满足 RP1-01；仅标记当前必须冻结或关闭的入口。
- RFC 0006 fixture 与 `owner-effect-fixture` 特性下的 broker 路径不在此表；不得用 fixture 证据替代本表产品行。

## 相关测试

| 测试 | 证明什么 |
| --- | --- |
| `tests/adversarial/.../workspace_authority_invariants.rs` | workspace 路径 bundle 中断与撤销竞争 soak |
| `crates/authority/tests/subprocess_claims.rs`：`revoke_during_bundle_commit_barrier_fails_closed` | F04：commit 前撤销的确定性屏障 |
| `crates/authority/src/tests.rs`：`a_revoke_vs_consume_race_ends_in_one_durable_outcome` | authority 层撤销 vs consume 结果枚举 |
