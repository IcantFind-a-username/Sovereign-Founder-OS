# Runtime Phase 1 — 产品 1C0 Owner Admission 设计冻结卡

日期：2026-09-16。事实基线：`d8ffec98`（Phase 1 指南、受保护入口清单、F04 屏障）。状态：**Target — 设计冻结与写集边界**；不构成产品 1C0 已实施、RP1-01 已通过或 owner admission 已合格的声明。

## 用途与权威链

本卡完成 [Runtime Phase 1 开发与验收指南](runtime-phase-1-development-guide.zh-CN.md) §4 **第 0 步**中「产品 owner admission」冻结交付物：在 RFC 0006 fixture 机制证据与产品 MVP 现状之间，列出缺口、准入条件、允许写集与显式非目标，供后续实现切片领取前评审。

| 文档 | 角色 |
| --- | --- |
| [Phase 1 指南](runtime-phase-1-development-guide.zh-CN.md) | 范围、合同、实施顺序、RP1-01–08 |
| [受保护入口清单](runtime-phase-1-protected-entries.zh-CN.md) | 产品 MVP 当前可触达路径（含诚实旁路） |
| [RFC 0006](../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md) | Fixture 机制合同（**无**产品准入声明） |
| [Security Architecture Program — 1C0](../superpowers/plans/2026-08-13-security-architecture-program.md#program-1c-owner-authenticatorsession) | 产品 1C0 退出门（normative target） |
| [Owner-session fixture 实施计划](../superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md) | RFC 0006 任务 1–16；**不**替代产品 1C0 设计 |
| [Owner 机制矩阵](owner-auth-mechanism-matrix.md) | 浏览器/ cookie / 端口观测；真实矩阵当前为空 |

**三种状态（不可混用）：**

| 状态 | 本卡如何使用 |
| --- | --- |
| RFC 0006 synthetic fixture | 证明「给定已建立 fixture credential」后的 session / 批准 / 预留 / 效果 / 证据；**不能**作为产品 owner 或 RP1-01 证据 |
| 当前产品 MVP（Experimental） | [受保护入口清单](runtime-phase-1-protected-entries.zh-CN.md) 描述的行为；含未认证 loopback → app-local owner signer |
| 目标产品 Phase 1 / 1C0 | 本卡冻结的准入与写集；满足前 **不得** 宣称 RP1-01 或「产品 1C0 已交付」 |

## 核心缺口（产品 vs RFC 0006 fixture）

RFC 0006 与产品路径 **故意分离**（G4–G5）。下表对比的是 **机制能力** 与 **产品今天是否具备可证明的 owner admission**，而非要求产品复制 fixture 常量。

| RFC 0006 / fixture 主题 | Fixture 已冻结或已证明的方向 | 产品 MVP 现状 | 产品 1C0 必须补齐 |
| --- | --- | --- | --- |
| **G6 — 空注册表不是 admission** | 明确披露同账户原生进程可赢空注册；不声称独立准入 | **无** WebAuthn/会话/CSRF；任意 loopback POST → `Store::decide` → `owner_secret` 自动建钥并签署 | 经准入的 owner authenticator + 不可被同账户本地进程合成的会话/批准发行者 |
| **G2 — 合取产品门** | 1B1 + 1C1 + 1D `ActiveV2` + protected-payload review | 未满足任一合取门；仍用 `vault/vault.key` 旁路文件与 app-local 角色密钥 | 1C0 **单独**不取消合取门；但 RP1-01 要求产品路径在 **指定部署** 上先满足 1C0 退出门 |
| **G3 — 无持久长寿命 owner 批准钥（fixture）** | 一次性临时批准钥；fixture 重启失效 | 首次发送时 `owner_approval_key` / `owner_admission_key` 等写入 vault 并持久 | 1C0 发行者与 1C1 托管交接设计一致；**本冻结卡不实施 1C1**，但写集须避免新增第二套 app-local signer |
| **G7–G9 — 固定 origin、cookie、CSRF** | `localhost:7787`、`__Host-sfo_fixture_session`、严格 CSRF | UI 任意 `--port`；**无** session cookie；仅 Host + JSON Content-Type | 产品 UI 的 origin/会话/CSRF 合同（可不同于 7787，须在机制矩阵或等价证据中冻结） |
| **G8 — 机制矩阵** | 虚拟矩阵一行；真实矩阵空 = 机制未合格 | 无产品矩阵；[矩阵结论 A–C](owner-auth-mechanism-matrix.md#what-the-observations-mean-for-1c0) 适用于任何 loopback cookie 设计 | 在支持平台上冻结「准入 ceremony + 会话绑定」的观测行，或诚实排除该平台 |
| **G5 — 特性门控** | `owner-effect-fixture` 非默认 | 默认 `sovereign ui` 暴露完整发送链 | 产品变更在默认构建中关闭旁路；fixture 保持特性门控 |
| **G1 / G4 — 合成范围与标记** | 合成收件人/标记；产品 open 拒绝 | 真实 ERP 图与业务数据（正确产品域） | 保持 fixture 标记与产品根互斥；1C0 不把 fixture 目录当产品 workspace |
| **Broker / 单写者 / IPC** | Fixture broker、launch key、redb 在锁后 | 单进程 `Store` 文件锁；**无**产品 broker 客户端 | 1C0 须定义「本地原生调用方无法伪造 owner 证据」的边界（Program 退出门第 2–3 条）；是否与 fixture IPC 同形 **不在本卡范围**内预定，但不得保留 HTTP 无认证批准 |
| **精确效果 / 封闭 writer** | Fixture 状态机 + 封闭 writer 测试 | 产品已有 `OutboxBroker` + authority bundle 链；撤销 HTTP 仅删 outbox（见入口清单） | RP1-02/03 与 1C0 正交但 RP1-01 要求无 writer/密钥旁路；持久 `revoke_*` 暴露策略另列任务 |
| **F01（审查 P0）** | Fixture Tasks 5/6/9 等 | **F01 产品侧未关闭**：app-local signer | 本卡冻结的 1C0 产品准入即 F01 的产品切片 |

**已文档化的诚实旁路（产品）：** 未认证 loopback `POST /api/workspace/decide` 成功批准并产生 owner 签署证据 → 使用 `kernel_exec::owner_secret` 在 `vault/` 内自动创建 `owner_approval_key`（及发送链上的 `owner_admission_key`、`runtime_authority_key`）。详见 [受保护入口清单 — HTTP](runtime-phase-1-protected-entries.zh-CN.md#httpappsclisrcuirs--ui_mvprs)。

## 产品 1C0 admission 在 RP1-01 之前必须要求什么

下列条目摘自 [Program 1C0 退出门](../superpowers/plans/2026-08-13-security-architecture-program.md#1c0--owner-authenticatorsession) 与 [Phase 1 指南 RP1-01](runtime-phase-1-development-guide.zh-CN.md#acceptance)，合并为 **产品路径** 准入清单。全部满足且证据在 **同一支持配置** 上可复现后，方可将 RP1-01 标为产品资格通过（仍受 RP1-02–08 与其它合取门约束）。

### A. Owner 身份与 ceremony

- [ ] **A1** 支持平台上存在经准入的 owner authenticator（或管理员预置的等价物），能力标签诚实（非「loopback 即 owner」）。
- [ ] **A2** 首次/持续 owner 绑定不依赖「第一个写入 vault 的进程」或 `owner_secret` 静默 `put`；RFC 0006 G6 类抢占在威胁模型中有明确处理或残留风险披露。
- [ ] **A3** 批准发行者与 session 绑定为 **同一 1C0 authority plane**；禁止第二个 app-local owner signer（Program 1C0 退出门第 4 条）。

### B. Loopback UI 与会话

- [ ] **B1** 本地 UI 具备 **已认证、带过期、抗 CSRF** 的 owner 会话；突变路由（含 `/api/workspace/decide`）在无有效会话时 **拒绝**（invert `an_unauthenticated_local_post_can_approve_today_1c0_pin`）。
- [ ] **B2** 同 OS 账户下的其它本地进程 **不能** 合成该会话或等效批准头（compile/API 测试 + 部署级隔离证据；仅 Host 检查不够，见矩阵结论 A）。
- [ ] **B3** `GET` / status / open 保持非突变：不因 loopback 可达而注册、建钥或泄露受保护状态（Program 1C0 第 3 条）。

### C. 受保护读路径（与发送链同等门控）

- [ ] **C1** 业务价值读路径（含 `/api/workspace`、`/api/export` 及等价 `Store` API）需要 live session 或更窄的一次性 broker 授权。
- [ ] **C2** [受保护入口清单](runtime-phase-1-protected-entries.zh-CN.md) 中每一行「无认证即可触达密钥/批准/outbox」的产品行已关闭或降级为显式 fixture/legacy 配置。

### D. 证据与测试诚实性

- [ ] **D1** 产品构建的 owner 批准证据可追溯到 1C0 发行者，而非仅 `founder-owner` + 自动 vault 条目（当前 `kernel_exec::sign_owner_approval` 硬编码 subject）。
- [ ] **D2** HTTP 层与 `Store::decide` 直调路径 **一致** 门控（避免测试绕过 HTTP 后仍批准）。
- [ ] **D3** 更新 [受保护入口清单](runtime-phase-1-protected-entries.zh-CN.md) 基线 SHA 与「已知旁路」列为零未关闭项（或标明残余风险与 RP1 编号）。

### E. 明确 **不** 属于 1C0 准入、但 RP1-01 叙述时常被误并的项

| 项 | 归属 | 本卡 |
| --- | --- | --- |
| 1B1 clean restore 资格 | Program 1B1 | 阻塞 RP1-07；**不**阻塞本卡冻结的 1C0 写集定义 |
| 1C1 角色密钥托管/轮换 | Program 1C1 | 与 1C0 接口但 **本切片不实施** |
| 1D `ActiveV2` / SQLCipher 1B0 | Program 1B0/1D | **禁止** 在本切片声称 |
| RFC 0006 fixture 任务 3–16 默认二进制暴露 | Fixture | **保持** fixture-only |
| Wave D / 真实邮件 provider | 路线图外 Phase 1 | **非目标** |

## 写集冻结

### 产品 1C0 实现时 **必须** 触及（未来切片；本卡仅冻结范围）

| 区域 | 路径 / API | 变更性质 |
| --- | --- | --- |
| HTTP 面 | `apps/cli/src/ui.rs`、`apps/cli/src/ui_mvp.rs` | 会话、CSRF、突变路由门禁；`/api/workspace/decide` 等 |
| Workspace 决策 | `apps/cli/src/workspace/ops.rs`：`decide`、`request_send` | 拒绝无 session 的批准；与 HTTP 一致 |
| 发送链 | `apps/cli/src/workspace/kernel_exec.rs`：`execute_signed_approval`、`sign_owner_approval`、`owner_secret` | 移除「任意调用即建钥签署」；session_id 来自 1C0 而非每次 `Uuid::new_v4()` 无证明 |
| 只读信任 | `apps/cli/src/workspace/kernel_exec.rs`：`admission_trust` 及读路径调用方 | 与 1C0 准入公钥/注册表对齐；去掉隐式 demo+owner 双锚混淆（产品路径） |
| 前端 | `apps/cli/assets/*.js`（批准/会话 UI） | 携带会话与 CSRF；不新增平行批准 API |
| 测试 | `apps/cli/tests/ui_http_boundary.rs` 等 | **Invert** 1c0 pin；新增 session 正例与原生旁路负例 |
| 文档 | [受保护入口清单](runtime-phase-1-protected-entries.zh-CN.md)、本卡 | 每切片更新基线与清单 |

### **保持 fixture-only**（产品 1C0 不得拖入默认构建）

| 区域 | 路径 / 特性 | 说明 |
| --- | --- | --- |
| Fixture broker / IPC | `crates/authority` + `crates/owner` 的 `owner-effect-fixture` | RFC 0006 G5；`security-fixture --synthetic-only` |
| WebAuthn 适配器 workspace | `fixtures/owner-webauthn/` | 独立 workspace/CI；真实认证器资格另验 |
| 合成标记与常量 | `synthetic-owner-effect-fixture-v1`、`fixture-recipient@example.test` 等 | G1/G4；产品 open 继续拒绝 |
| Owner-effect 回归脚本 | `scripts/run-owner-effect-regression.sh`、`scripts/owner-effect-tests.tsv` | Fixture 证据；≠ 产品 RP1 |
| RFC 0006 约束门 | `scripts/check-owner-effect-rfc.sh` | 仅约束 RFC 正文，不授权产品 |

### **本阶段不修改**（合取门或其它 Program）

| 区域 | 原因 |
| --- | --- |
| Wave D、公共 egress、provider 发送 | Phase 1 指南 §2 排除 |
| `vault-v2-engine` / 1B0 SQLCipher FFI | RFC 0005 合取门；非 1C0 切片 |
| 1D `ActiveV2` 迁移状态机 | Program 1D；依赖 1B1/1C1 |
| `crates/authority` 产品 redb broker 替换 filesystem store | 可与 1C0 并行规划，但 **不在** 本冻结卡实施 |
| Desktop Tauri 壳 | 若暴露同等 HTTP/IPC，按入口清单 **另表**审查；本卡以 CLI `sovereign ui` 为产品 MVP 代表 |

## 失败 / 诚实性测试钉（当前）

| 测试 | 位置 | 证明什么 | 1C0 落地后 |
| --- | --- | --- | --- |
| `an_unauthenticated_local_post_can_approve_today_1c0_pin` | `apps/cli/tests/ui_http_boundary.rs` | 无凭证 POST 可批准并产生 owner evidence | **Invert**：断言拒绝 |
| `product_decide_mints_vault_keys_without_owner_admission_1c0_pin` | 同上 | 批准前无 `vault.key`；未认证批准后自动创建 vault 与 app-local 签署链 | **Invert** 或删除自动建钥路径后改为「无 vault 则拒绝」 |
| `revoke_during_bundle_commit_barrier_fails_closed` | `crates/authority/tests/subprocess_claims.rs` | F04 fixture/authority 屏障 | 产品 dispatch 分离时复验 |
| Owner-effect fixture 套件 | `scripts/owner-effect-tests.tsv` | RFC 0006 机制 | **不**替代上表产品 pin |

## 显式非目标（本冻结卡与紧随其后的 1C0 产品切片）

- **Wave D** 与真实邮件/provider 路径。
- **ActiveV2**、**1B0**、业务 DB SQLCipher 升级、whole-workspace 明文关闭。
- 宣称 **RP1-01–08 产品资格通过** 或「**产品 1C0 已发货**」。
- 在默认 `sovereign` 二进制中启用 `owner-effect-fixture` 或把 fixture broker 当作产品 admission。
- 用 RFC 0006 合成验收 **替代** 受保护入口清单上的产品行。
- 大规模重构 authority store、隐私编译或 model gateway（除非单切片明确领取且通过评审）。

## 冻结评审清单（Step 0 签字用）

评审人确认下列项 **已读且一致** 后，方可从 backlog 领取「1C0 产品 HTTP/session」类实现切片：

1. [ ] 已读 [Phase 1 指南](runtime-phase-1-development-guide.zh-CN.md) §3 合同与 §4 第 0 步。
2. [ ] 已读 [受保护入口清单](runtime-phase-1-protected-entries.zh-CN.md) 全文，承认当前 RP1-01 **未满足**。
3. [ ] 已读 RFC 0006 **Summary、G2、G5、G6**，理解 fixture ≠ product admission。
4. [ ] 同意本卡 **写集** 表：产品改动不默认化 fixture；fixture 不削弱产品门。
5. [ ] 同意 **非目标** 列表；切片 PR 不夹带 Wave D / ActiveV2 / 1B0。
6. [ ] 下一实现 PR 须先 **invert 或替换** 上节 1c0 pin 测试，并更新入口清单基线 SHA。
7. [ ] 不合取宣称：1C0 产品切片完成 **不等于** 1B1/1C1/1D/protected-payload 完成。

## 开放依赖（设计前置，不在本卡关闭）

| 依赖 | 说明 |
| --- | --- |
| RFC 0007 amendment | RP1-06；authority 代际与 audit 锚（指南 §4 实施前差异 §3） |
| RFC 0003 产品 dispatch 屏障 | 多进程产品 coordinator 时的 F04 复验 |
| 产品 origin / 端口策略 | 可与 fixture `localhost:7787` 不同，须在机制矩阵或产品等价文档中冻结 |
| [runtime-owner-http](backlog.md#runtime-owner-http) | HTTP 边界已建立；1C0 在其上扩展 |
| [runtime-owner-design](backlog.md#runtime-owner-design) | 机制设计已收敛到 RFC 0006 + plan；**产品** admission 以本卡为冻结 |

---

**变更记录：** 2026-09-16 初版（`d8ffec98` 基线）；对应 backlog 项「Phase 1 step-0 产品 1C0 owner-admission 设计冻结」。
