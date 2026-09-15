# Runtime Phase 1 开发与验收指南

日期：2026-09-16。事实基线：`b6dc508e`（RFC 0005 Accepted，Amendments 1–2）。状态：**Target；开发入口与验收目标，产品尚未通过本阶段验收。**

## 1. 用途与文档分工

本指南把[安全内核审查][review]转成现有规划的实施路径。Sovereign Runtime 先作为 **SFOS 仓内的安全架构边界**；先稳定嵌入 API 和实际调用合同，再评估提取。Phase 1 聚焦一个精确本地 outbox 效果。

| 文档 | 职责 |
| --- | --- |
| [审查文][review] | 解释风险、设计理由和长期方向；其中建议不等于已接受规范 |
| RFC [0002][rfc2]、[0003][rfc3]、[0004][rfc4]、[0005][rfc5]、[0006][rfc6]、[0007][rfc7] | 定义各自范围内的机制、版本和准入要求；分别检查状态 |
| [Roadmap][roadmap]、[Security Architecture Program][program] | 保留既有里程碑和依赖顺序 |
| 本指南 | 串联范围、合同、实施顺序、验收和现有任务 |
| [backlog][queue] | 唯一任务队列；复用既有任务及 plan Task，不重复开实现票 |

本文不构成 RFC 接受记录、接口冻结记录、平台资格或产品启用授权。遇到代码与文档不同步，记录当前行为及差异；遇到规范冲突，先按 [Roadmap governance][governance] 修订相应 RFC，再实施。不能用当前缺口降低目标，也不能用目标描述当前保护。

## 2. 第一阶段范围

### 总体验收承诺

> 在明确威胁模型与受支持部署下，即使 agent 已被控制，也无法产生超授权的本地效果；授权的消费与撤销状态在崩溃恢复后不倒退；结果不确定的操作不会自动重试；owner 能独立验证授权和执行证据，并识别仍未知的结果。

### 部署与攻击者

- 首个验收配置覆盖单机、单 owner、单 workspace、一个可信单写者 coordinator，以及一种受保护效果：写入精确 `.eml` 本地 outbox。写入文件不表示邮件已经发送。
- 将模型输出、agent 运行进程、工具参数、文档与外部内容视为可被攻击者完全控制；测试允许它们伪造请求、重放引用、竞争撤销、尝试直接访问文件和本地端口。Owner admission 还必须处理 RFC 0006 已披露的同账户原生进程抢先注册问题。
- 可信边界包括 owner 确认入口、授权与效果 coordinator、密钥保护、受控 broker、验证/恢复工具及其 OS、密码和存储依赖。已攻陷这些可信组件、宿主管理员完全控制系统、owner 明确批准过宽权限，须在具体威胁模型中单列，不能声称本阶段解决。
- 产品要求无旁路 API **和有效的进程/权限隔离**。被控制的 agent 不能读签名密钥、改授权/撤销记录、写受保护 outbox，或继承等效文件句柄与凭据。仅有另一个 PID、同 UID 文件权限或 loopback 地址不足以证明隔离。
- 先选择一个能实际运行隔离与故障测试的平台配置。记录 OS、文件系统、运行身份、认证 IPC、可访问目录/句柄、网络与子进程权限；未验证的平台保持未资格化。VM、多租户和分布式 authority 留给后续有需求的阶段。

### 三种状态必须分开

| 状态 | 可以证明什么 | 后续门 |
| --- | --- | --- |
| 当前产品 / Experimental | 已有局部原语与可运行流程；完整链路仍有审查缺口 | 不能引用本指南宣称已抵御被控制的 agent |
| RFC 0006 synthetic fixture | 在已建立 fixture credential 的前提下验证 session、reservation、效果与故障机制 | 不能建立产品 owner，不能读产品 root，不能取消既有产品门 |
| 目标产品 Phase 1 | 在指定部署下通过下文全部适用验收 | 真实产品路径遵守 1C0、1C1、1B1、1D `ActiveV2` 与 protected-payload review 等合取门 |

固定合成数据的 RFC 0006 机制工作可以先推进。产品 protected-payload、持久 authority/freshness 或真实 dispatch 接入遵守 [Program 的依赖图][dependencies]。**Vault 门不能因“效果只是本地文件”而被跳过。** RFC 0005 已为 **Accepted**（`b6dc508e`，Amendments 1–2）：可按其 acceptance gates 推进所列 **非产品 Program 1A** 引擎工作；**不等于** 1B0 实施（SQLCipher 绑定准入仍阻塞）、产品 enrollment/migration/selection/dependency、或 1D `ActiveV2` 资格——这些仍受 RFC 正文中的合取门约束。

本阶段不开放真实邮件/provider、通用 shell/网络、公共模型或多租户。受管 agent 的基本权限隔离是本阶段必需项；若把真实本地模型纳入本阶段支持配置，其 [Program 4][local-model] 身份与隔离门也必须通过。独立 Ollama server 的 loopback 配置不能替代这些证据。

<a id="contracts"></a>

## 3. Runtime 合同：开发时必须共同成立的语义

下面的调用名表达职责，**不新增一套 wire format，也不冻结 Rust API**。复用 Program 2 的 `EffectIntentV1`、现有签名类型和 opaque proof/handle；新字段或边界变化先进入对应 RFC。Mission → Invocation → Effect 是授权收窄关系；首期沿用最小单父授权结构，复杂委派与预算系统不扩大本期范围。

| 边界 | 必须满足的合同 |
| --- | --- |
| 认证与读取 | 主体/workspace/session 来自可信认证上下文。业务 read/list/preview/decrypt/export 需要 live owner session 或更窄的 broker 授权；普通 GET/status 不隐式创建身份、密钥或批准 |
| `prepare` | 不可信提案经可信 broker 解析，先生成随机 `effect_intent_id`，再封存精确收件人、最终 RFC 5322 字节、操作、资源版本、policy、期限与保留规则；预览来自同一份封存状态 |
| `authorize` / reservation | 消费精确绑定且未过期的一次性 owner evidence；适用的已有任务授权必须显式覆盖本动作。Approval、token、幂等键、authority node、effect intent 及适用限额在一个权威事务中预留；失败不能产生可执行 handle |
| `execute` | 只接受已认证调用者和其绑定的 opaque handle；不额外接受可替换的 recipient/body。Coordinator 复核撤销、期限、资源版本与状态，持久进入 `Dispatching` 后，由封闭 writer 读取同一封存 payload |
| `revoke` | 与开始派发共用先后判定点。撤销先赢则阻止开始；派发先赢则返回已经开始/需核实的准确结果。已发出的效果不能承诺被追回；失效状态不因清理、重启或恢复重新有效 |
| `status` / `reconcile` | 状态读取受授权且不触发执行。核实只检查可信 intent、文件身份、实际内容与持久记录，并追加结论；不能因路径存在就认定成功，不能通过重发来探测是否成功 |
| 证据 | 操作事务是权威状态，audit ledger 是证据投影。普通签名证据只含随机 intent ID、闭合 outcome 与独立批准公开的字段；收件人、内容及可枚举的确定性摘要留在受保护状态 |
| 重启与恢复 | 保留已消费、撤销和未知记录；恢复协议隔离旧 session/grant/epoch。恢复业务数据不自动恢复执行权；依赖不可用时停止新效果，保留授权的诊断与恢复入口 |

### 撤销与崩溃的状态规则

遵循现有 [Program 2][exact-effect] / [RFC 0006][rfc6] 的严格边界：

```text
Prepared → AuthorityReserved → Dispatching → Succeeded | Indeterminate
Prepared / AuthorityReserved → FailedBeforeDispatch
```

`FailedBeforeDispatch` 只允许从 `Prepared` 或 `AuthorityReserved` 到达。

- `AuthorityReserved → Dispatching` 与撤销串行裁决，并先持久化开始记录。不会用“最后再检查一次”代替共同裁决点。
- `Dispatching` 后发生超时、进程死亡、发布/同步/回执不确定，保守落为 `Indeterminate`；本期不引入“进入 Dispatching 后直接报未执行并安全重试”的新分支。
- `rename` 成功但目录同步失败，或效果已落地但回执未写入，均不得报告确定的未执行。Process-kill 测试不能代替断电持久性资格。
- `Indeterminate` 不自动 retry、failover、重新签名或换新 ID 执行。核实可追加确认结论，保留原始未知记录；后续新尝试遵守另行明确的重新授权规则。
- 已预留/消费的授权因撤销或故障未产生效果，可以保持不可复用。不能承诺“没有效果就一定退还授权”，也不能把本地事务描述为跨系统 exactly-once。

### 不允许存在的旁路

逐项登记 CLI/UI、desktop native bridge、后台任务、Workspace 方法及旧 effect API 的调用路径，列出主体、授权源、持有的文件/密钥权限、失败语义和所属 broker（当前快照见 [受保护入口清单][protected-entries]）。产品迁移完成后，旧的任意字节 writer、app-local owner signer 和第二套 session/approval 入口必须不可达。Fixture 的单进程机制合同保持其独立范围；产品进程/IPC 边界依照 [Program 的变更规则][reference-slice] 单独审查，不借本指南直接改造 fixture。

Authority、approval、audit、credential 与业务 Vault DBK 是不同密钥/状态域。统一事务裁决不能通过把这些秘密或 effect authority 塞入业务 DB 来实现；存储边界与恢复协议必须符合 [RFC 0005][rfc5]。

<a id="sequence"></a>

## 4. 实施顺序与现有任务对应

下表是执行索引，不是第二份 backlog。开始一项工作前检查当前代码、既有完成记录与活动任务；旧条目文字可能滞后，不能根据旧措辞重新实现已落地能力。

| 顺序 | 工作与完成物 | 复用的规范 / 任务 | 开始下一步之前 |
| --- | --- | --- | --- |
| 0 | 冻结首个部署、[受保护入口清单][protected-entries]、产品 owner admission、权限隔离及状态转换差异 | Program 1C、Program 2；[已有 1C0 设计记录][q-owner]；[产品 1C0 owner-admission 设计冻结卡][product-1c0-freeze]；RFC 0003 / 0006 / 0007 的差异见下节 | 将缺失的产品设计与必要 amendment 放回原 Program 归属；经所需审查后冻结写集和失败用例 |
| 1 | 完成/核实 fixture 的 owner session、单写者与认证器机制 | [16-task owner/exact-effect plan][owner-plan]：Tasks 3–6；[Task 4 remainder][q-task4]；[HTTP boundary][q-http] | Fixture credential 不能作为产品 admission；WebAuthn adapter 已位于独立 workspace，真实 authenticator 资格另验 |
| 2 | 验证原子预留、精确 payload、撤销裁决与封闭 writer | 同一 plan Tasks 7–11；[跨 crate 事务/撤销项][q-authority]；Program 2 | 保留 legacy store 与 fixture coordinator 的区别；产品实现先具备第 0 步的合同和适用存储门 |
| 3 | 完成证据核实、浏览器攻击、产品/fixture 分离、恢复、freshness 与各崩溃窗口 | 同一 plan Tasks 12–15；[journal 恢复][q-recovery]、[真实进程强杀][q-kill]、[进程竞争][q-race]、[锚点][q-anchor]、[open-time 检查][q-freshness-open] | 先修订 freshness 的配对回滚边界；不得以旧 checkpoint 重执行行为代替 unknown 规则 |
| 4 | 完成真实产品的 owner/key custody、受保护持久化和 clean restore，再连接产品路径 | RFC 0005；[Vault 1A plan][vault-plan]；[已有 qualification 项][q-vault]；Program 1B/1C/1D | RFC 0005 接受记录、精确依赖/平台资格、1B1、1C0、1C1、ActiveV2、protected-payload review 全部适用门成立 |
| 5 | 在同一产品构建和同一支持配置上运行组合攻击与故障验收，发布限制与证据 | 下文产品 RP1 验收表；owner plan Task 16 提供独立的 fixture 证据；复用已有 checked runners、fault-testing 和 adversarial suite | 全部必需项通过；条件项明确适用性；独立评审完成，才更新产品成熟度主张 |

产品路径的前置门不会因为上述行号而后移：第 1–3 步可以推进隔离机制或 legacy 修复；任何真实产品集成仍以前置审查和启用条件为准。

Owner plan 的 Task 编号与前置条件仍按原计划执行；已完成的 Tasks 1/2 和后续局部实现先核对剩余验收。Task 16 的 fixture 记录可在合成验证齐备时先产出，不能作为第 5 步产品资格的替代。

### 实施前需要消除的差异

1. **RFC 0003 与撤销竞争：** 审查 F04 原先依赖线程交错推导。`sovereign-authority` 的 `fault-injection` 屏障 `AfterBundleApprovalClaimBeforeCommitRecheck` 与 `revoke_during_bundle_commit_barrier_fails_closed`（`subprocess_claims`）强制“撤销在 commit 重检之前落地”的窗口；workspace 路径仍保留 `a_workspace_revoke_vs_dispatch_race_*` 作为补充 soak。若产品 dispatch 与 authority 分离到不同进程，仍需在共享裁决点重复同类屏障证据。
2. **审查状态草图与既有 Program 2：** 审查有 `Dispatching → FailedNoEffect` 的通用候选分支；本期执行上节更严格的既有合同。任何放宽需要先审查、修订规范并给出未产生效果的可验证证据。
3. **RFC 0007：** 账本与同目录有效旧锚点一起回滚仍是 workspace 回滚；独立保护签名 key 不阻止旧签名 replay。对应任务需要先纳入受保护的最新 head/generation、enrolled 状态和丢失处理的 amendment。Authority 消费/撤销状态也必须由可信代际或恢复失效规则保护，不能仅凭 audit 链新鲜就假定授权不会复活。
4. **旧验收措辞：** [checkpoint double-burn][q-checkpoint] 是 legacy 行为刻画，不是新 Runtime 可自动重执行的许可。跨 crate 任务中“效果拒绝时不得烧权限”的笼统措辞，应在原任务收紧为具体阶段的不变量，允许已消费授权在安全失败后保持失效。
5. **代码先于旧记录：** WebAuthn adapter、durable revocation、bundle 与 `LocalVouch` 已有实现。先核对剩余验收；[model 文档项][q-model] 的旧 cloud-hole 预期不能用于重新开放已封闭的 raw request 路径。

<a id="acceptance"></a>

## 5. Phase 1 验收表

RP1 是验收编号，不是新开发票。所有行当前均为 **未完成产品资格**；局部代码或 fixture 测试通过不能单独勾选。每行最终附代码版本、支持配置、实际测试名、命令、结果、审查记录和残余风险。

| 编号 | 要证明的性质 | 必须覆盖的反例 / 故障 | 完成依据 |
| --- | --- | --- | --- |
| RP1-01 | Owner 身份与受保护入口无旁路 | 原生未认证 POST/GET、跨端口页面、空注册表抢占、旧 session、错误 workspace、直接 writer/密钥/句柄访问 | 1C0 产品证据 + 完整入口清单 + 实际 OS 权限拒绝；合法 owner 的正常流程也成功 |
| RP1-02 | 批准绑定最终本地效果 | 批准后换收件人/正文/附件/目的地/对象版本、错主体复用 handle、越权或未知 action | 预览、批准、reservation、文件内容属于同一封存 intent；所有替换拒绝；无 grant 无文件 |
| RP1-03 | 一次消费、原子预留和撤销有确定先后 | 并发提交同一 approval/token/幂等键；撤销在 reserve/dispatch 屏障两侧发生；关键目录 sync 失败 | 同一操作最多一次启动；成功撤销阻止后续开始；持久性错误不能产生成功凭证；重启后已消费/撤销授权仍不可用 |
| RP1-04 | 故障结果准确且不自动重试 | 发布前 kill、rename 后 sync 失败、receipt 前 kill、重启、同名错误内容、断连 | 重启只核实；按合同得出成功、派发前失败或未知；不重新签名、不换 ID、不重复效果 |
| RP1-05 | Owner 能独立核验证据并看见未知 | 停掉模型/框架，篡改证据、替换验证根、缺失回执，尝试从公开证据猜收件人/正文 | 独立 verifier 使用预先受信任的根/最新状态；核验授权和记录，明确未知；精确内容核实走 owner 授权的受保护路径 |
| RP1-06 | 恢复不接受旧权力或静默降级 | 旧 ledger + 旧 anchor、删除 anchor/enrolled 状态、跨 workspace 替换、旧 backup/authority/session、恢复途中崩溃 | 修订后的 freshness/恢复合同通过；丢失可信最新状态进入显式受限恢复；旧 grant 不复活；全设备回滚另列未保证范围 |
| RP1-07 | 真实产品数据与密钥满足适用 Vault 门 | 复制 workspace 目录、设备保护器不可用、旧 wrapper、角色混用、新设备恢复、legacy writer fallback | RFC 0005 接受与平台资格、1B1 clean restore、1C1、1D ActiveV2、protected-payload review；仅合成 fixture 可记“不适用”，不能据此通过产品验收 |
| RP1-08 | 被控制的 workload 无法取得环境权限 | 读宿主文件、开 socket、继承句柄、启动子进程、替换 worker/artifact、隔离 backend 缺失 | 支持配置实际拒绝旁路且失败不降级；若纳入真实模型/不可信编译，则对应 Program 4 / RFC 0002 测试一并通过 |

RP1-05 不声称已提供公开的“精确内容承诺证明”。Program 2 将该能力留给独立 commitment RFC；公开 value-free evidence 与 owner 授权的本地内容核实各自保留边界。

<a id="findings"></a>

## 6. 十项发现的优先级与归属

这里的 **review:P0** 表示首个可信产品闭环前必须关闭，**review:P1** 表示启用对应能力前必须关闭；这是审查分级，与 backlog 的 P1/P2/P3 排队标签分开。本表不改变 Roadmap 版本号：相关基础工作横跨 v0.1/v0.2 等既有门。

| 发现 | 审查分级 | 既有工作归属 | 本阶段处理 |
| --- | --- | --- | --- |
| F01 owner 根 | review:P0 | Program 1C0；[1C0 记录][q-owner]、[HTTP boundary][q-http]；owner plan Tasks 5/6/9/13 | RP1-01；fixture 验证之后仍需产品 admission 与可信确认入口 |
| F02 key custody | review:P0 | RFC 0005，Program 1C1/1D；[Vault plan][vault-plan]、[qualification][q-vault] | RP1-01/07；authority/owner 密钥隔离不能推迟到一般业务加密之后 |
| F03 精确效果 | review:P0 | Program 2；owner plan Tasks 8/10/11 | RP1-02；保留现有 intent/opaque handle 方向，封闭产品 raw writer |
| F04 原子撤销 | review:P0 | RFC 0003 Amendment 1；[跨 crate 事务/撤销][q-authority]；owner plan Tasks 3/10/15 | RP1-03；先复现交错，再修复共同裁决点与 durability 传播 |
| F05 dispatch 结果语义 | review:P1 | owner plan Tasks 11/15；[journal 恢复][q-recovery]、[kill][q-kill] | RP1-04；若复用该 dispatcher，升级为本阶段必过项 |
| F06 policy 默认拒绝 | review:P1 | Program 2；owner plan Tasks 7/10；RFC 0002 的授权边界 | RP1-02/03 必须拒绝未知动作与伪造上下文；通用策略扩展另按原 Program 冻结设计 |
| F07 本地模型真边界 | review:P1 | [Program 4][local-model]；[model 旧文档项][q-model] | RP1-08 的基础 workload 隔离必过；真实模型纳入支持配置时，其身份与零出网测试必过 |
| F08 privacy 来源/自由文本 | review:P1 | RFC 0004、Program 3；[已有 cloud adapter 阻塞项][q-cloud] | 本期保持公共 egress 关闭；开放公共计算前关闭该缺口，公开证据仍须遵守 RP1-05 |
| F09 freshness | review:P0 | RFC 0007；[锚点][q-anchor]与[open-time 检查][q-freshness-open] | RP1-06；先完成 amendment，不把 key 保护等同于最新值保护 |
| F10 可选运行保障 | review:P1 | RFC 0002；Program 4；既有 sandbox/worker gates | RP1-08；本期 profile 所需 journal、存储、隔离缺一则不能启动；扩展 backend 单独准入 |

部分归属目前只有 Program 目标、尚无完整产品卡片。下一次设计轮在相应归属下补齐或收紧一张有明确写集和验收的卡片；本表不把它们伪记为已排队实现，也不复制 owner plan 的 16 个 Task。

<a id="verification"></a>

## 7. 每个开发切片的交付方法

1. **领取一项。** 从 backlog 领取，读对应 RFC 和 plan Task；确认当前实现、前置门、允许写集和禁止改变的签名/持久格式。产品安全边界变化先具备所需设计和审查。
2. **先写失败用例。** 行为修改先钉住非法效果或权限状态；故障窗口用已有 barrier/fault/subprocess 工具确定性触发。随机 sleep soak 可补充，不能代替精确窗口。文档切片检查引用、状态和合同一致性。
3. **复用内核原语。** 沿用 `contracts` 的签名字节顺序、artifact admission、opaque proof、authority store、`fault-testing` 和现有 runner。不要新增平行 session、通用 signer、重复序列化/原子写工具或第二套业务权威状态。
4. **按范围验证。** 修改涉及哪些 crate/profile，就运行其有意义的测试；所有提交前运行 scoped gate。安全链路集成还需全工作区与所支持平台的故障/隔离资格。
5. **记录和评审。** 提交包含行为、测试证据、失败/未知语义、影响的验收行与残余风险；遵守现有 plan/handoff 的独立评审规则。核对其他调用方后才关闭旧入口。

仓库根目录的基本命令：

```bash
./scripts/test_changed.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

Owner/authority 非默认 fixture 的回归须使用现有 checked runner，例如：

```bash
./scripts/run-owner-effect-regression.sh -- cargo test \
  -p sovereign-authority -p sovereign-owner \
  --no-default-features \
  --features sovereign-authority/owner-effect-fixture,sovereign-authority/fault-injection,sovereign-owner/owner-effect-fixture \
  --locked
```

这条命令只覆盖所列包与特性；CLI 的 debug/release/fault 矩阵沿用 [owner-effect-tests.tsv][manifest] 与已有计划。`fixtures/owner-webauthn` 有独立 workspace 和 CI/gate，核心 `--workspace` 的绿色不能证明它已测试，更不能证明真实认证器资格。Vault 使用其计划规定的精确平台/依赖 qualification 入口。

每次验收记录至少包含：提交 SHA、命令、包/feature/profile、实际执行的测试、平台/文件系统、故障屏障、合法操作对照、非法效果观察结果、未知结果和限制。测试数不等于覆盖率；无法运行或零项执行不能记为通过。可公开的日志先检查敏感值，正式资格证据使用仓内受版本管理的记录，不依赖临时路径。

## 8. 本指南完成后的下一步

从第 4 节第 0 步开始，在既有 Program 1C/2 归属下冻结**首个平台的产品 owner admission、受保护入口与进程权限边界**，复用 RFC 0006 的机制证据。并将本指南列出的 RFC 0003/0007 差异纳入对应原任务的设计前置项。每次只释放一个可独立验收的实施切片。

本指南入库只完成开发入口整理；RP1-01–08 保持未完成产品资格，直到同一支持配置上的证据和评审齐备。

[review]: 2026-09-16-agent-security-kernel-review.zh-CN.md
[rfc2]: ../../rfcs/0002-wasm-sandbox-and-plugin-capabilities.md
[rfc3]: ../../rfcs/0003-signed-approval-evidence.md
[rfc4]: ../../rfcs/0004-data-sovereignty-boundaries.md
[rfc5]: ../../rfcs/0005-dual-root-vault-and-recovery.md
[rfc6]: ../../rfcs/0006-synthetic-owner-session-exact-effect-fixture.md
[rfc7]: ../../rfcs/0007-audit-ledger-freshness-anchor.md
[roadmap]: ../../ROADMAP.md
[governance]: ../../ROADMAP.md#roadmap-governance
[program]: ../superpowers/plans/2026-08-13-security-architecture-program.md
[dependencies]: ../superpowers/plans/2026-08-13-security-architecture-program.md#hard-dependency-graph
[reference-slice]: ../superpowers/plans/2026-08-13-security-architecture-program.md#chosen-target-reference-slice-design
[exact-effect]: ../superpowers/plans/2026-08-13-security-architecture-program.md#program-2-authority--exact-effect-protocol-v1
[local-model]: ../superpowers/plans/2026-08-13-security-architecture-program.md#program-4-verified-real-local-model
[owner-plan]: ../superpowers/plans/2026-08-14-owner-session-exact-effect-v1-implementation.md
[vault-plan]: ../superpowers/plans/2026-08-13-dual-root-vault-v2-implementation.md
[manifest]: ../../scripts/owner-effect-tests.tsv
[queue]: ../backlog.md
[q-owner]: ../backlog.md#runtime-owner-design
[q-task4]: ../backlog.md#runtime-owner-task4
[q-http]: ../backlog.md#runtime-owner-http
[q-authority]: ../backlog.md#runtime-authority-invariants
[q-recovery]: ../backlog.md#runtime-journal-recovery
[q-checkpoint]: ../backlog.md#runtime-checkpoint-gap
[q-kill]: ../backlog.md#runtime-process-kill
[q-race]: ../backlog.md#runtime-process-race
[q-anchor]: ../backlog.md#runtime-freshness-anchor
[q-freshness-open]: ../backlog.md#runtime-freshness-open
[q-model]: ../backlog.md#runtime-model-boundary
[q-vault]: ../backlog.md#runtime-vault-qualification
[q-cloud]: ../backlog.md#runtime-cloud-adapter
[protected-entries]: runtime-phase-1-protected-entries.zh-CN.md
[product-1c0-freeze]: runtime-phase-1-product-1c0-owner-admission-freeze.zh-CN.md
