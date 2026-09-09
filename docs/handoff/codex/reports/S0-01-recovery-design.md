# S0-01 历史保留恢复设计候选

2026-09-09 · 结论：**design_candidate**。执行准入仍为 **blocked**，诊断
`S0_01_HISTORY_NOT_REPLAYABLE`。本报告供独立 reviewer 审阅，尚未批准恢复，
不发布实现卡，不创建 `events/S0-01.json` 或任何 `contract.json`。

## 结论与范围

现有 v2 无法无改规范地恢复。最小可行方向是：保留原始 Git 历史和未知事实，
通过一次明确、受审的开发协议迁移行政关闭旧 attempt 1，以保守额度占用约束
之后的新 attempt 2；新一轮严格采用新的 B/C/W/R/M。不是重放修补，也不是把
旧代码重新标成已验收。需要 contracts v3、protocol 的迁移条款及受影响卡的新
revision；不需要产品 RFC 变更，不改任何 Rust 安全接口。

建议只准入这一个已审计的 S0-01 遗留实例，不建设通用导入器、数据库、签名层、
恢复 CLI 或第二份状态库。若 reviewer 不接受下述保守额度/行政终结语义，保持
具名阻塞；不能直接按 v2 补写 claim/start/block/attempt_fail。

本次实际写集仅本报告。后文的卡、路径、类型、命令都是拟议后续工作，未经独立
审阅和规范冻结不得执行。完整目标仍见 `models-and-goals.md` §13.1；一周是推进
优先级和时间约束，不豁免 S0 准入，也不能据此承诺一周内全量 MVP 已能完成。

## 已核对基线与历史

实际 repo：本仓库工作树；分支
`docs/founder-os-execution-blueprint`；本报告写入前工作树干净，HEAD 为
`38544604a78332e22b165ea5b79980396ea328f5`。Git object format 为历史报告所述
SHA-1，以下均为本次 Git 读取到的完整提交。

| 事实 | Git commit / 原始证据 |
| --- | --- |
| 旧 B | `34e307beda8a9498d410b78d3e617d493009a573` |
| 旧 backlog claim，缺 contract/event | `a3d1664e963809172c0a83b4ba6ef6cf72e222c4`，父为旧 B |
| 五份配置实现，非合法 W | `ebceebf8cdf9d59ecfc2b8a2ebf9210de6160499`，父为旧 claim |
| attempt 1 报告与 backlog 混合提交 | `1a0f2d4e55e91776ed70bd1d7efccf0e45b4d460` |
| 报告空白修正 | `780975956f14ba24fcbe64e66a63010cde23684f` |
| 已提交 blocked 审计及非执行 draft | `38544604a78332e22b165ea5b79980396ea328f5` |

本次复核旧 B→配置提交的路径确实包含 `docs/backlog.md`。tasks/events 的 Git
历史只有最后一项中的 `tasks/S0-01/r2/contract.draft.json`，无合法契约或事件。
因此不得把旧 claim 选作追认 B，或以旧配置提交作 B 来隐藏原始越界。

旧 claim、所有旧提交、`reports/S0-01-attempt-1.md`、
`reports/S0-01-record-repair.md`、`tasks/S0-01/r2/contract.draft.json` 原样保留。
不能改名成 contract.json、补全未知字段后冒充旧冻结契约，不能合成 r2 契约
加入 validateEvents 历史。已提交 draft 仍只是示意，不具有执行授权。

读取了唯一入口、contracts v2、protocol §9.1–9.7/10、models-and-goals §8.2/
13.1、S0-01/S0-02 卡、backlog、两份原始报告和 draft；已验收的 scaffold/kickoff
review 仅证明文档就绪。backlog 对 record-repair 的 accepted-as-blocked 表述不
证明 S0-01 运行通过。RFC 0004 的真实 egress 边界、RFC 0006 的 fixture-only
限制及 RFC 0007 对“内部链一致不等于历史完整”的限制继续适用；此开发记录
迁移不触碰 RFC 0002/0003 的签名和授权边界，不提供产品权限或凭据读取理由。

## 为什么不能直接写 v2 events

v2 要求非空事件从唯一 claim 开始；该 claim 使用实际 controller Actor，随后
start 的实际作者、预冻结预算、计时和合法 B 必须可验证。旧历史缺这些事实，
且不存在合法来源状态可用于 block/resume/revise/reslice。用当前 actor 填旧事件、
把未计时填成 0、写一个并未发生的 attempt_fail、生成新零失败 claim，都不合法。

只加文档解释不能改变 `validateEvents(contracts, events)` 的输入语义。
直接创建 r3 contract 而忽略旧 claim 也违背 v2 的历史保存要求。因此规范迁移
必须先经独立审阅、提交，再生成任何可执行记录；本报告本身不是迁移授权。

## 最小 v3 接口草案

唯一规范仍是 contracts.md：沿用 v2 所有普通形状和函数名，schemaVersion 改为
3；v2 历史按原规则继续验证，不原地改写。单次重放禁止混合 v2/v3；既有合法 v2
历史的迁移不在此方案范围内。S0-01 旧资料没有合法 v2 契约，故不属于混用。
普通 v3 流程与 v2 行为一致；仅新增受限首事件及其首次 start 前的严格计时分支。

```typescript
// BlobRef 使用已有 inputBlobs 的相同字段与路径规则；不是新摘要算法。
type BlobRef = { path: string; blob: string };
type LegacyAccounting = {
  closedThroughAttempt: 1;
  observedLunaFailures: null;
  observedFallbackStarts: null;
  historicalActor: null;
  historicalSpentMs: null;
  lunaFailureReservations: 1;
  fallbackReservations: 1;
};
type LegacyImport = {
  accounting: LegacyAccounting;
  evidence: BlobRef[];
};
// OrdinaryEventV3 为 v2 Event 的原有精确字段，只改 schemaVersion=3。
// 不给普通事件新增可选字段；它们仍拒绝 legacy 字段。
type LegacyImportEvent = Omit<OrdinaryEventV3, "kind" | "spentMs"> & {
  kind: "legacy_import";
  spentMs: null;
  legacy: LegacyImport;
};
// 无新增字段：只在成功 import 后、首次 start 前允许这三个事件使用 null。
// 与 import 不同，LegacyPendingEvent 不携带 legacy 字段；状态由重放保留。
type LegacyPendingEvent = Omit<OrdinaryEventV3, "kind" | "spentMs"> & {
  kind: "block" | "resume" | "revise";
  spentMs: null;
};
type EventV3 = OrdinaryEventV3 | LegacyImportEvent | LegacyPendingEvent;
// ValidationV3 = 原 Validation 的全部字段 + 必填：
// legacy: LegacyAccounting | null
// validateTask/validateEvents/checkResume/verifyEvidence 沿用原函数参数数量。
// v3 相关输入和返回使用上述 v3 类型；其余返回形状不变。
```

限定规则必须全部进入规范，而非让工具从本报告猜测：

1. `legacy_import` 只允许 `taskId=S0-01, revision=3, seq=1, attempt=1`，
   只出现一次，替代普通首 claim；同一流不得再出现 claim/import。contract
   history 从真实 r3 开始。r2 draft 不算前契约，但必须作为 evidence/inputBlob
   固定。其他 taskId 只能普通 claim；不得以改 ID 或新 revision 再导入。
2. 导入事件 actor 是**实际作出本次导入的强模型 controller**，匹配新契约。
   它不是旧 actor。candidateCommit/checkRunPath/resumeFrom 均 null，children
   为 []，reason 固定为 `S0_01_HISTORY_NOT_REPLAYABLE`。reportPath 指向本次
   迁移授权报告，reviewPath 指向其独立审阅；不是旧 candidate/review 绑定。
3. `legacy.accounting` 的键和值恰如上，不能填估算数或模型自报身份。
   `spentMs=null` 仅对导入及下述首次 start 前的 LegacyPendingEvent 合法；
   OrdinaryEventV3 仍须有限非负整数，不能单凭 kind 或 null 绕过重放前提。
   导入产生 `Retryable, nextAttempt=2, binding=null, lunaFailures=0`，并返回
   完整 legacy 对象。此处 lunaFailures 只统计新流中已证实的失败；界面/报告
   必须同时显示未知旧失败与占用额度，禁止把 0 叙述成历史零失败。
4. 有效 Luna 额度使用 `lunaFailures + legacy.lunaFailureReservations`。
   一份已知旧 attempt 即保守占用一次；因作者未知也保守占用唯一 fallback
   槽。它们是新决策下的准入保留值，**不是宣称旧 Luna/fallback 真失败过**。
   新 attempt 2 若失败，进入 NeedsArchitect，禁止第三次 Luna；同任务不得
   再 fallback_start。强模型只可诊断并实质拆卡或保留阻塞，不可无限修复。
   保留值跨 revise/block/resume 永不降低，不能用发现旧“0 失败”的摘要清除。
5. 行政关闭只终结“旧 attempt 的继续执行权”，不证明成功、实现失败、环境
   故障或旧耗时；不得复活 attempt 1。明确批准后才能 start attempt 2，新的
   start spentMs=0 是新工作的计时起点。旧耗时一直 null；没有可计算的历史
   总耗时，不能宣称总预算未超。新预算只约束新 attempt，不冒充原预算余额。
6. `legacy.evidence` 路径唯一且非空，至少固定以下七个已提交文件在新 B 的
   blob：旧 attempt 报告、record-repair、r2 draft、本设计、设计独立 review、
   迁移授权报告、迁移授权独立 review。全部须在 contract.inputBlobs 同值
   出现。sourceCardBlob、新 protocol/contracts/models/CLAUDE/backlog 也固定
   于 B；r3 后的普通 revise 继续引用所有真实先前 contract.json。
7. Git 检查另核上表六个原始提交仍是 B 的祖先，旧报告/draft 内容与审计固定
   版本一致；不把文件仍存在当成未篡改证明。纯状态机只检查形状和绑定关系，
   不自称查过 Git、review 独立性或进程停止。verifyEvidence 仍拒绝任何缺少
   新 candidate→review_start→review_pass 的 accept。导入不能产生 candidate。
8. `validateTask` 对 v3 仍返回 state=null、nextAttempt=1、lunaFailures=0、
   binding=null，另 legacy=null；它不能代表准入。validateEvents 首事件错误
   返回相同初值；后续非法事件返回最后合法状态和 legacy，不部分消费事件。

首次 start 前的合法中间表示也须冻结到 contracts §3：状态保持现有 `Retryable`
或 `Blocked`，不新增对外 Status。重放导入后，内部 `legacyPendingStart=true`、
`lastAttempt=1`、`currentAttemptSpentMs=null`；1 已行政关闭，没有活动 worker，
null 表示旧工作耗时未知，不能以 0 表示“尚无新工作”替换它。内部字段从事件
派生，不新增可由调用者提交的状态字段；ValidationV3.legacy 保留上述未知事实。

| 从 import 后、首次 start 前的事件 | 合法结果及计时 |
| --- | --- |
| block | Retryable→Blocked；attempt=1、spentMs=null、resumeFrom=Retryable，真实 controller、原因/解除证据沿原规则。 |
| resume | Blocked→Retryable；attempt=1、spentMs=null、resumeFrom=null；先按原恢复规则核停止及解除观察。 |
| revise | 仅 Retryable→Retryable；attempt=1、spentMs=null；切到下一份真实更高 revision 的契约，以新契约核 actor/base，归档全部先前契约，保留 legacy 和 nextAttempt=2。 |
| start | 仅 Retryable→Running；attempt=2、spentMs=0，实际 worker；首次创建新计时，内部 legacyPendingStart=false。 |

Blocked 必须先 resume，不能直接 start/revise。尚未 start 时除表内事件外均拒绝，
包括 attempt_fail/candidate/fallback_start/reslice；不得用虚构终结或拆卡绕过
未知耗时。需要不同恢复行为须另作受审规范变更。前述三个 pending 事件的其他
键/actor/路径/seq/当前 revision/base 全沿普通规则，仅 spentMs 分支有例外。
传入数字（包括 0）代替 pending null 返回 BUDGET_RESET；其他阶段/任务使用
LegacyPendingEvent 的 null 返回 INVALID_TRANSITION。错误保留最后合法状态。
首次 start 后的所有事件恢复有限非负整数和单调计时要求，即使再次 Retryable、
Blocked 或 revise，也绝不重新进入 pending 分支；legacy.historicalSpentMs 则
永久保留 null。这保证 import→block→resume→start 和 import→revise→start
无需杜撰旧耗时，又不把所有普通 block/resume/revise 都放宽为可空。

建议新增稳定错误：`LEGACY_IMPORT_NOT_ALLOWED`（task/revision/位置不符）、
`LEGACY_ACCOUNTING_CHANGED`（额度/未知事实被修改）、
`LEGACY_EVIDENCE_MISSING`（冻结引用缺漏）。未知键复用 UNKNOWN_FIELD，路径和
对象校验复用三个 state helper；不要加第二套 schema/JSON 工具。v2 输入带导入
事件必须拒绝；未来 S0-03 负责真实 Git 绑定，S0-04/CLI 透传历史不清零。

这是最小的例外授权，而不是默认通用恢复途径。若随后查到另一个旧 attempt、
实际更高失败次数或 fallback 使用记录，与限定“一份旧 attempt”前提冲突，
必须停止本次导入，更新审计并重新审阅；不得把新增证据塞入现有固定占用值。

## 必须先解决或保留 blocked 的证据

| 项目 | 决策 / 解除证据 |
| --- | --- |
| 旧 actor、失败分类、spentMs、预冻结预算 | 保留未知；不能靠再跑 smoke 证明过去。只有上述受审迁移可解除历史重放阻塞，未知本身仍展示。 |
| 旧 writer、等待审批任务、检查进程是否停止；common-dir 锁 owner | 当前均未由本报告核验。实际线程/工具状态及 controller 观察要归档；未知就 `S0_01_PRIOR_WRITER_STOP_UNKNOWN`，不得只凭旧报告或文件干净下结论。 |
| 真实 controller / worker / reviewer 模型可用性与实际调用路径 | 依 §8.2 真实分派；错误模型/工具拒绝保持 blocked。caller 和 decision author 分别记录。 |
| 配置自动加载、role 覆盖、只读 sandbox 强制 | 可诚实保持未验证，但须按卡展示限制并证明可用的显式分派路径。不能把只读行为说成权限隔离。 |
| 嵌套分派与槽位生命周期 | 工具可观察的直接嵌套或 §8.2 转发完整路径必须有新证据；没有槽位不可重复开任务。 |
| 当前 checkout 之外的旧 lane writer/自动化 | controller 检查真实占用和写集冲突；本报告不读取凭据，不暂停或删除未知自动化。 |

## 拟议后续卡与精确写集（尚未发布）

1. **S0-01-RECOVERY-REVIEW**：独立 reviewer 只读本设计和原证据，输出
   `docs/handoff/codex/reviews/S0-01-recovery-design.md`。职责仅为接受/要求修改/
   阻塞设计，必须裁决行政关闭、保守双额度与 evidence-only W。controller
   归档其结论；当前 architect 不审自己的设计。
2. **S0-01-RECOVERY-SPEC**：依赖上项 accepted；这是按现行 v2 冻结的独立
   设计卡，`workerRole=architect`，不是 reviewer-only 或无 attempt 的记录任务。
   合法卡路径为 `docs/handoff/codex/cards/S0-01-RECOVERY-SPEC.md`，revision 1，
   taskId=`S0-01-RECOVERY-SPEC`、parentTaskId=null。architect 将本草案落到唯一
   规范，精确写集为 `docs/handoff/codex/protocol.md`、`contracts.md`、
   `README.md`、`cards/S0-01.md`、`cards/S0-02.md`、`cards/S0-03.md`、
   `cards/S0-04.md`、`cards/S0-05A.md`、`cards/S0-05.md`、`cards/S0-06.md`
   （除首项外均相对 `docs/handoff/codex/`）；唯一报告
   `docs/handoff/codex/reports/S0-01-recovery-spec.md`。contracts 升 v3；受影响
   卡逐张增 revision，不能只换标题引用。models-and-goals §8.2 无行为变化，
   不改。不得写配置、源码或 events。由独立 reviewer 输出
   `docs/handoff/codex/reviews/S0-01-recovery-spec.md`，controller 在确切候选
   被接受后集成并登记 backlog。必须产生这张独立设计卡的真实 architect
   attempt/candidate/check/review；其提交与记录顺序详见下一节。
3. **S0-01-RECOVERY-AUTHORISE**：强 controller 的记录任务，依赖规范候选
   accepted、旧 writer 停止/无锁冲突/实际分派能力的当前观察。唯一写入
   `docs/handoff/codex/reports/S0-01-recovery-authorisation.md`；实际强 architect
   决定行政关闭并保留额度，独立 reviewer 输出
   `docs/handoff/codex/reviews/S0-01-recovery-authorisation.md`。报告需包含实际
   actor/tool ID、证据路径、未知事实、额度保留、停止/锁观察、配置原始 blob。
   不提前写导入事件。缺观察只产出具名阻塞，禁止再安排重复审计轮。
4. **S0-01 r3 attempt 2**：仅在上述依赖成立后按下节冻结并派发。worker 精确
   写集仍为 `.codex/config.toml`、`.codex/agents/founder-worker.toml`、
   `.codex/agents/founder-reviewer.toml`、`.codex/agents/founder-fallback.toml`、
   `.codex/agents/founder-architect.toml`；唯一额外报告为
   `docs/handoff/codex/reports/S0-01-attempt-2.md`。配置存在则核对并保留。
   若无需配置修改，r3 卡应明确允许一个 `--allow-empty` 的 evidence-only W，
   仅为固定新 attempt 的候选身份；不得做无意义配置改动制造 diff。W 的 config
   内容需要完整审阅，报告交付新运行证据，旧 smoke 不计本次通过。
5. **S0-02 r3**：仅依赖 S0-01 新 Accepted 和 reviewer 指定下一卡后启动；
   写集仍仅 `scripts/codex-control/state.mjs`、`state.test.mjs`，另唯一报告
   `docs/handoff/codex/reports/S0-02-attempt-1.md`。首次失败历史不因这张卡
   开始而清除。S0-03/04/05A 维持各自原文件所有权，增加下节对应验证义务。

以上 controller 的队列更新只能写 `docs/backlog.md`；不得授权 worker 更新。
review 文件由独立 reviewer 结论生成，controller 可归档原文，不能代其裁决。
所有写任务串行，没有共享文件并发写入。上述前置步骤是一次恢复决定的审阅、
规范落地与事实准入，不允许各自再产生无止境的“恢复设计卡”。若通过后仍无
法派发唯一 attempt 2，终止重复文档轮并报告具体工具/观察阻塞。

## RECOVERY-SPEC 自身的 v2 冻结与提交顺序

不能用尚未生效的 v3 给这张规范变更卡授权。先由 controller/architect 准备上述
revision 1 卡（不是执行者修改自己的卡），独立 reviewer 核准范围和本设计，
controller 提交该卡与准入队列记录。该准备提交保持现行 v2 规范不变；它成为
独立任务的干净基线 **Bspec**。卡的 allowedWritePaths 恰为上列十个规范文件，
不含自己的卡、backlog、S0-01 旧报告/草案、events 或以下 controller 记录。
reportPath 恰为 `docs/handoff/codex/reports/S0-01-recovery-spec.md`。

1. **Cspec（父 Bspec）**：controller 仅提交
   `docs/handoff/codex/tasks/S0-01-RECOVERY-SPEC/r1/contract.json`、
   `docs/handoff/codex/events/S0-01-RECOVERY-SPEC.json`（真实 v2 claim，seq=1，
   attempt=0，spentMs=0）、`docs/backlog.md`。contract.schemaVersion=2、
   revision=1、baseCommit=Bspec、workerRole=architect；sourceCardBlob 指向
   Bspec 中上述卡。inputBlobs 固定现行 v2 protocol/contracts、模型规范、
   CLAUDE、已审设计及其独立 review。dependencies 固定已接受的恢复设计审阅
   准入证据，dependencies=["S0-01-RECOVERY-REVIEW"]；不把 S0-01 完成写成
   依赖，更不声称 S0-01 已完成。
2. 实际强 controller/architect/reviewer 快照和这张卡自己的工作预算在 Cspec
   前冻结；Cspec 通过门后，architect 独立写入分支从 **Bspec** 创建。实际
   dispatch 返回 author thread 后追加该任务的 v2 start（attempt=1），计时
   从调用起始；不把当前设计报告作者自报当新 actor 工具证据。
3. architect 仅修改十个获准规范文件，提交 **Wspec**；之后 **Rspec（父 Wspec）**
   仅新增指定 recovery-spec 报告。Wspec 可以产出 v3 文本，但这张任务仍按
   Cspec 冻结的 **v2** 验证执行和验收，不拿 Wspec 新规则解释自己的历史。
4. controller 在 HEAD=Wspec 的干净 worktree 检查真实 diff/冻结输入/适用门，
   记录 `docs/handoff/codex/reports/S0-01-recovery-spec.checks.json`；独立
   reviewer 只读 candidate 并给出正式 v2 Review，归档
   `docs/handoff/codex/reviews/S0-01-recovery-spec.json`，人类可读说明沿用
   `docs/handoff/codex/reviews/S0-01-recovery-spec.md`。candidate/review_start/
   review_pass 只追加到 **RECOVERY-SPEC** 自己的事件数组。
5. 通过后保留原始 Wspec 身份合并，再合入核准 Rspec。**Mspec** 相对 Wspec
   只允许上述 contract、events、backlog、checks、review JSON/Markdown 和
   worker report 七个精确记录路径；发生其他差异或冲突停下受审修复。运行
   必需集成门后另行追加 v2 accept/Accepted backlog。此后 v3 文本才可作为
   S0-01 迁移的冻结规范；RECOVERY-SPEC 仍保留完整 v2 历史，不混版重放。

该设计卡是真实不同的问题与写集，不是 S0-01 实现重试、改名、子任务 reslice
或其替代验收。其 attempt 1/计时只描述本次 architect 规范工作；不能消费、
归零或重分类 S0-01 的旧 attempt/未知失败/未知耗时。它失败时按现行 v2 的
architect 失败进入 NeedsArchitect，不能以新卡重开来无限循环。controller
仅在其自己的事件/契约路径写记录，绝不提前创建或触碰 `events/S0-01.json`。

## 新 B/C/W/R/M 的确切次序

字母仅表示尚不存在的提交，执行时必须替换完整真实 Git object ID；不得预填
假 SHA。新 B 必须是上述审阅和授权全部已提交、保留旧历史的干净 integration
HEAD；旧历史无需重写、reset、删除或 cherry-pick。五份已有配置留在 B。

1. 在 B 冻结 r3 卡 blob、规范/全部证据 inputBlobs、实际 actors、写集、报告和
   新 attemptBudgetMs。建议新 attempt 30 分钟、最多三轮针对性修复；不是旧
   花费估计。固定 `diff-check` 60 秒和 scoped-gate 30 分钟的 timeout，检查
   命令沿卡，真实分派与角色加载证据仍须独立核对。再次观察停止和锁无冲突。
2. **C（父 B）**只增加 `tasks/S0-01/r3/contract.json`、
   `events/S0-01.json`（初始唯一 legacy_import）、更新 `docs/backlog.md`。
   通过适用 gate 并提交 C 后才准派发。contract/事件文件均相对 codex 目录。
   当前 import actor、独立授权 review 和全部历史引用必须通过手工核对。
3. worker 分支从 **B** 分叉。C 已提交后只调用一次实际 dispatch；取得工具返回
   的真实 worker ID 后，controller 在记录分支追加 `start`（无中间事件时 seq=2，
   有 pending block/resume/revise 时使用紧接前项的 seq；attempt=2,
   spentMs=0），actor 使用该实际 thread，计时从实际 dispatch 开始。事件落盘
   晚于调用不把工作计时起点后移。调用结果或崩溃后身份未知时，保持具名诊断
   并查询原任务，不造 start、不重复分派。worker 读取 C 的只读契约，不以 C
   为 Git 基线；同一时刻一个 writer。
4. worker 实作/核对配置并完成初检，提交 **W（B 的后代）**；若证据型无改动，
   W 是父为 B 的允许空提交。然后 **R（父 W）**仅新增 attempt-2 报告。报告
   含 W、真实分派参数和 ID、调用者/决策者、计时、限制及 diff --stat。
5. controller 单独干净 worktree HEAD=W 重新 inspect/checks；输出只在
   `.harness/codex/S0-01/r3/attempt-2/`。把确切 CheckRun 归档为
   `docs/handoff/codex/reports/S0-01-attempt-2.checks.json`，把 candidate 事件
   追加到事件流。CheckRun 中 local 日志路径须可供 reviewer 读取；最终报告
   入库保留命令、退出/完成与必要输出摘要，不能只有会消失的日志路径。
6. 独立 reviewer 读取 B→W **及 W 全部五份配置**、R 的精确路径差异、实际
   日志和冻结契约。controller 追加 review_start；review 输出归档
   `docs/handoff/codex/reviews/S0-01-attempt-2.json`，匹配 actor/W/checkRun，
   accepted 后追加 review_pass。只读 smoke reviewer 不自动成为候选 reviewer。
7. controller 原始合并 W 到 integration，再合入核对过的 R，保留 W 身份。
   **M** 相对 W 只允许逐项列出的五个 controller 记录路径（contract、events、
   backlog、checks JSON、review JSON）和 attempt-2.md；先前迁移文档已在 B，
   不属于新差异。任何冲突/其他差异停止并受审修复，不在合并时顺手编辑。
8. 在 M 执行必需集成门、核对 W 身份和路径后，controller 另一个记录提交
   追加 accept/Accepted backlog，工作树干净才成为 S0-02 的 B。若集成改变
   配置/测试/规范，则旧 review 失效，必须重验；不提前宣称 M 等于 Accepted。

C 及其后事件提交可因真实时序增加记录提交，但不能颠倒 B→C→dispatch、
B→W→R 或 review_pass→merge→集成门→accept。任何新 attempt 失败/阻塞按
v3 普通状态转换追加；不覆写 import 和旧报告。

## 验证卡要求与命令

当前报告只做静态设计核查。后续规范冻结前检查：接口字段无双重定义，import
例外只限首事件/固定任务，旧 v2 保持严格拒绝；所有受影响卡 revision 和
planned tests 同步；七个证据路径和 controller/worker 写集无重叠。

S0-02 在现有命名测试之外新增（均为 planned）：

- `legacy_import_is_single_first_event_for_s0_01_r3_only`
- `legacy_import_preserves_unknown_facts_and_reserves_both_allowances`
- `legacy_import_requires_frozen_evidence_refs`
- `legacy_import_starts_next_attempt_at_two_without_candidate_binding`
- `legacy_reservations_survive_revision_block_and_resume`
- `legacy_import_block_resume_start_preserves_unknown_until_attempt_two`
- `legacy_import_revise_start_uses_next_contract_without_resetting_unknown`
- `legacy_pending_numeric_spent_including_zero_is_rejected`
- `legacy_pending_requires_resume_before_start_or_revise`
- `legacy_pending_null_is_rejected_after_first_start_and_in_ordinary_tasks`
- `legacy_import_then_one_luna_failure_requires_architect`
- `legacy_import_never_reopens_attempt_one_or_fallback_allowance`
- `legacy_import_cannot_accept_without_new_candidate_and_independent_review`
- `ordinary_events_reject_null_spent_and_legacy_fields`（不包括严格满足上述前提的 LegacyPendingEvent）
- `v2_rejects_legacy_import_and_mixed_schema_history`
- `invalid_import_is_not_partially_replayed`

S0-03 追加 Git fixture：缺/改旧 report/draft、缺祖先、输入 blob 不匹配、传假
review、在 C/R 而非 W 检查全部拒绝；evidence-only W 的配置读取仍受审。
S0-04 追加 import 后 resume 不删除 reservations、未知旧 writer 不准恢复。
S0-05A 追加 CLI 完整 v3 replay/错误退出，S0-05 追加一条隔离迁移 fixture
从 import→attempt 2→review→accept 的端到端试演及失败停机分支。复用现有
拟议模块，不新建 recovery.mjs；S0-06 汇总这些实际结果再接管 controller。

```bash
# 本报告 / 规范文档候选：另核 untracked，diff 本身看不到新文件。
git status --short
git diff --check
git diff --no-index --check /dev/null docs/handoff/codex/reports/S0-01-recovery-design.md
# controller 对本轮文档候选使用实际 frozen base；此处为本报告检查基线。
TEST_CHANGED_BASE=38544604a78332e22b165ea5b79980396ea328f5 GATE_SELFTEST_RUNNING=0 ./scripts/test_changed.sh
# 仅 S0-02 等测试文件真的实现后执行；不得预称通过。
node --test --test-reporter=tap scripts/codex-control/state.test.mjs
# 后续 git/checks/lock/cli/rehearsal 使用各自冻结卡命令与当前基线 gate。
```

M 的门按 protocol §10.1 与实际改动运行，执行参数记录到新 CheckRun/报告。
此文档任务不声称跑过不存在的 Node 测试或全量 Rust 测试。若后续增加测量脚本，
必须先入库再产生可引用数字；不得用一次性脚本数字写 DECISIONS。

## 工具清单、复用与恢复摘要

本次实际验证：`git diff --check` exit 0；对本报告这个 untracked 文件运行
`git diff --no-index --check` 实际 exit 1，但无空白诊断；不将该退出码称为 0。
`git status --short` 仅显示本报告为新文件，未提交。controller 已运行 scoped gate
命令 `./scripts/test_changed.sh`；实际详细日志路径为 `.harness/test_changed.log`，
其中记录 gate-self-test、file-size、fmt。

实际复用：Git object/parent/history/diff、文档读取；controller 后续使用
`scripts/test_changed.sh`。本报告没有新增工具或运行时依赖。接口扩展在现有
计划 state/git/checks/lock/CLI 内实现；新的 legacy_import 是数据/状态语义，
不是独立运行时工具。工具清单已核对：

- WorkflowRunner：`crates/workflow/src/lib.rs:112`；私有 persist 不跨模块复刻。
- AuthorityStore：`crates/authority/src/lib.rs:167`。
- AuditLedger / verify_chain：`crates/audit-ledger/src/lib.rs:42,135`。
- ModelProvider：`crates/model/src/lib.rs:118`。
- policy_decision_digest：`crates/capability/src/v2.rs:802`。
- evaluate_prepared：`crates/policy/src/lib.rs:281`。
- consultant-playground fixtures：`crates/consultant-playground/tests/support/`。

这些 Rust 工具无需为开发恢复调用或重写。用户列的 benchmark/artifacts.py、
report.py、metrics.py、matcher.py、review/executor.py 不存在，不声称复用。
**复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。**

恢复摘要：完整 Goal 在 models-and-goals §13.1；当前 S0/S0-01 r2，旧 attempt 1
历史不可重放，无合法 W、CheckRun 或 Accepted。实际 HEAD 见上；失败数、旧
spentMs、controller/锁 owner 未核验。当前角色为受派 architect，唯一产物为本
设计候选。下一个动作仅独立审阅此候选；之后一次规范落地和实际准入观察，才
允许新 r3/attempt 2。S0-02 须等新 S0-01 被 reviewer 接受、controller 集成门和
Accepted 记录完整且 reviewer 指定其为下一卡。没有读取凭据、调用 Anthropic、
启动产品或改 Goal；原历史和所有安全门保留。一周目标下不另建恢复基础设施，
若必要停止/分派证据不可获得，只输出具体具名阻塞，不继续文档空转。
