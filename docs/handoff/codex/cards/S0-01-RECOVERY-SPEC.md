# S0-01-RECOVERY-SPEC — 历史保留恢复规范落地

**Revision 3 · Spec Frozen（设计任务，按 v2 执行）· workerRole=architect。**
本卡只把已接受恢复设计落到唯一规范，不实现 v3，不执行 S0-01 恢复。
实际认领/完成唯一见 [backlog](../../../backlog.md)；Frozen 不代表准入已完成。

## 目标、依赖与冻结输入

依据提交 `24ecdd7529fbf6d06fcb6f5e08b785a07ca6282d` 中的
[恢复设计](../reports/S0-01-recovery-design.md)，一次完成 contracts v3、迁移流程
及受影响卡的修订。该设计 blob 为 `4ef02498ef4564e8ebbdee5d578180de5f8eb0af`；
它仍是历史设计证据，执行规范的唯一位置继续为 contracts/protocol。

分派快照使用 [contracts v2 §2](../contracts.md)，固定：

| 字段 | 值 |
| --- | --- |
| schemaVersion | `2` |
| taskId / revision / parentTaskId | `S0-01-RECOVERY-SPEC` / `3` / `null` |
| sourceCardPath | `docs/handoff/codex/cards/S0-01-RECOVERY-SPEC.md` |
| workerRole / reviewerRole | `architect` / `reviewer` |
| dependencies | `["S0-01-RECOVERY-REVIEW"]` |
| reportPath | `docs/handoff/codex/reports/S0-01-recovery-spec.md` |
| maxLunaFailures / attemptBudgetMs | `2` / `1800000` |

候选交付前最多三轮针对性修复，计时、预算和失败按本卡冻结的 v2 处理。
`expectedActors` 恰有五种角色，来自 Bspec 中
[models-and-goals §8.2](../models-and-goals.md#bootstrap-controller) 和真实可用分派路径；
controller/architect/reviewer 使用实际强模型快照，worker/fallback 也须完整冻结。
实际工具 ID、model/effort 参数、调用者与决策者分别留证；模型自报不是身份依据。

controller 在 Cspec 前以完整真实 Git object ID 填 `baseCommit=Bspec`、
`sourceCardBlob=Bspec:sourceCardPath`，并固定以下 `inputBlobs` 的 Bspec 内容。
不得预填假 SHA、从工作树即时读取替代冻结输入，或使用 Wspec 的 v3 解释本卡：

- `CLAUDE.md`、`docs/handoff/codex/README.md`、`docs/backlog.md`。
- `docs/handoff/codex/protocol.md`、`contracts.md`、`models-and-goals.md`
  （后两项同属 `docs/handoff/codex/`）。
- 下列 allowedWritePaths 中七张 S0 卡、`MANIFESTO.md`、`THREAT_MODEL.md`。
- `rfcs/0002-wasm-sandbox-and-plugin-capabilities.md`、
  `rfcs/0003-signed-approval-evidence.md`、`rfcs/0004-data-sovereignty-boundaries.md`、
  `rfcs/0006-synthetic-owner-session-exact-effect-fixture.md`、
  `rfcs/0007-audit-ledger-freshness-anchor.md`。
- `docs/handoff/codex/reports/S0-01-recovery-design.md`，须匹配上列已审设计 blob。
- `docs/handoff/codex/reviews/S0-01-recovery-design.md`：真实独立 reviewer 的
  accepted 结论，明确裁决行政关闭、保守双额度和 evidence-only W；在 Bspec
  前必须已提交，不能用本卡作者总结或未落盘聊天代替。
- `docs/handoff/codex/reports/S0-01-attempt-1.md`、
  `docs/handoff/codex/reports/S0-01-record-repair.md`、
  `docs/handoff/codex/tasks/S0-01/r2/contract.draft.json`：只读原始证据。

首次 claim 的本卡没有先前契约；若以后 revise，则按 v2 归档并引用全部真实
先前契约。依赖接受由 controller 和独立 reviewer 核对，不能以纯状态校验
冒充跨任务准入。**不依赖或声称 S0-01 已完成**；它仍需单独迁移授权及新 attempt。

## 精确文件所有权

`allowedWritePaths` 恰为以下十个完整仓库相对路径，无 glob：

```text
docs/handoff/codex/protocol.md
docs/handoff/codex/contracts.md
docs/handoff/codex/README.md
docs/handoff/codex/cards/S0-01.md
docs/handoff/codex/cards/S0-02.md
docs/handoff/codex/cards/S0-03.md
docs/handoff/codex/cards/S0-04.md
docs/handoff/codex/cards/S0-05A.md
docs/handoff/codex/cards/S0-05.md
docs/handoff/codex/cards/S0-06.md
```

唯一额外写路径是上述 reportPath，且在 Rspec 才新增。执行 architect 不改自己
的卡、backlog、任何 tasks/events/reviews、旧报告/draft、models-and-goals、RFC、
源码、配置或 gate；不创建恢复脚本。controller 独占本卡记录路径，reviewer
独立裁决，所有写任务串行，不覆盖别人的改动。

## 必须交付的规范与接口

以下是本设计卡的交付约束；精确类型、转换与错误的唯一正文写入 contracts，
执行/迁移次序写入 protocol，其他文件只引用，不建立第二份状态库或完整计划。

1. contracts 升 v3，保留普通 v2 规则和严格独立的 v2 重放；单次历史拒绝混版。
   普通 v3 只替换 schemaVersion，普通事件仍拒绝 legacy 字段。只为本次
   `S0-01/r3` 的首事件增加 `legacy_import`，不得建设通用导入器或迁移合法 v2 历史。
2. 按已审设计逐键落地 `BlobRef`、`LegacyAccounting`、`LegacyImport`、
   `LegacyImportEvent`、`LegacyPendingEvent`、`EventV3`、`ValidationV3`。
   accounting 固定 `closedThroughAttempt=1`，observedLunaFailures、
   observedFallbackStarts、historicalActor、historicalSpentMs 均为显式 null，
   lunaFailureReservations 与 fallbackReservations 均为 1；拒绝未知字段。
   ValidationV3 在原 Validation 全字段之外必有 `legacy: LegacyAccounting | null`。
3. 沿用以下接口的参数数量和职责，v3 输入采用对应 v3 类型，不改变其他返回形状：
   `validateTask(contract)`、`validateEvents(contracts, events)`、
   `inspectCandidate(contracts, repoDir, candidateCommit)`、
   `runChecks(contract, repoDir, candidateCommit, outputDir)`、
   `verifyEvidence(contracts, events, git, checks, review)`、
   `checkResume(contracts, events, observation)`；准确类型签名沿 contracts §3–5
   扩展。复用 `checkRecordKeys(value, keys)`、`isNonNegativeInteger(value)`、
   `isRepoRelativePath(value)`，不另建 schema helper。
4. import 只允许 `taskId=S0-01, revision=3, seq=1, attempt=1`；真实当前
   controller 为 actor，reason=`S0_01_HISTORY_NOT_REPLAYABLE`。spentMs=null，
   candidateCommit/checkRunPath/resumeFrom=null，children=[]；report/review 指向
   本次授权及独立审阅，不能成为 candidate binding。导入后 Retryable、
   nextAttempt=2、binding=null、lunaFailures=0，legacy 保留所有未知事实。
   有效 Luna 用量是已证实失败加 reservation；0 不代表旧失败为零。
5. 严格保留首次 start 前分支：import 后内部推导 pending、lastAttempt=1、
   spentMs=null；只允许 block、resume、revise 使用 null，以及 start attempt 2
   使用 0。Blocked 必须先 resume；revise 只在 Retryable 并按新契约核 actor/base。
   pending 的数字 spentMs（含 0）返回 BUDGET_RESET；普通任务/首次 start 后的
   null 返回 INVALID_TRANSITION。pending 拒绝 candidate/attempt_fail/fallback_start/
   reslice 等其他事件。首次 start 后恢复普通单调整数计时，永不重新进入 pending。
6. 冻结 `LEGACY_IMPORT_NOT_ALLOWED`、`LEGACY_ACCOUNTING_CHANGED`、
   `LEGACY_EVIDENCE_MISSING` 的触发范围；对象未知键沿用 UNKNOWN_FIELD。
   首事件非法返回初值，后续非法事件返回最后合法状态与 legacy，不部分采纳。
7. import 的 evidence 至少含原 attempt 报告、record-repair、r2 draft、已审设计、
   设计独立 review、迁移授权报告和授权独立 review 七个精确文件；每项必须在
   inputBlobs 同值出现。将已审设计列出的六个原始提交祖先检查、旧报告/draft
   固定版本核对交给 Git 层；pure validator 不声称验证 Git、停止或独立身份。
8. protocol 固定一次受审行政关闭：只结束旧 attempt 1 的继续执行权，不能将
   未知事实改为成功/失败/环境原因/0。双 reservation 跨 revise/block/resume
   永不降低；新 Luna attempt 2 失败即 NeedsArchitect，禁止第三次 Luna 和同任务
   fallback_start。新预算仅计新工作，无可计算的历史总耗时或原预算余额。
9. README 修正入口/依赖指向；七张 S0 卡逐张从 revision 2 升 3，同步契约引用、
   planned tests 和准入。S0-01 保留原五配置写集，唯一报告为 attempt-2.md，
   明确无需配置变动时允许父为新 B 的 `--allow-empty` W，仍完整审阅五份配置与
   新分派证据；不能把旧 smoke 当新通过。S0-02 仅在新 S0-01 Accepted 且 reviewer
   指定后启动，原两文件写集和 attempt-1 报告不扩张。S0-03/04/05A/05 原模块
   所有权不变；S0-06 仍为 reviewer-only，不制造 worker attempt。
10. 后续准入顺序固定：本规范 accepted → S0-01-RECOVERY-AUTHORISE 的真实
    停止/锁/分派观察与独立授权 review → S0-01 r3 attempt 2 → S0-02 r3。
    本卡只在十个规范文件中描述这些依赖，不额外创建 AUTHORISE 卡/报告/事件。
    models-and-goals §8.2 不变，产品 RFC、安全边界与凭据规则不变。

## 本卡 Bspec/Cspec/Wspec/Rspec/Mspec

以下字母是待产生的提交身份，执行时用真实完整 ID 替换，不是预先存在的 SHA。

1. **Bspec**：controller 提交本卡、独立设计 review 与准入队列记录后的干净
   integration HEAD。现行 v2 未变；依赖已接受且没有其他 writer/锁冲突。
2. **Cspec（父 Bspec）**：controller 仅提交本卡 r1 contract、本卡 events 与
   backlog；contract 固定上列 v2 值与输入，events 首项是真实 claim，seq=1、
   attempt=0、spentMs=0。通过适用门并提交 Cspec 后才分派。architect 从 Bspec
   独立分支写入，只读 Cspec 契约。真实 dispatch 返回 thread 后追加 v2 start
   attempt=1，计时自实际调用起始；身份不明先查询原任务，不重复分派或造 start。
3. **Wspec**：architect 完成十文件规范候选及初检后提交；Bspec→Wspec 只含
   allowedWritePaths。**Rspec（父 Wspec）**仅新增指定报告并引用 Wspec；修正报告
   的后续记录也只准修改该路径。交付时核对 untracked，工作树干净。
4. controller 在干净独立 worktree **HEAD=Wspec** 检查冻结输入、实际 diff 和门；
   日志仅入 `.harness/codex/S0-01-RECOVERY-SPEC/r1/attempt-1/`，CheckRun 归档
   为下列 checks.json。独立 reviewer 核 Bspec→Wspec、Rspec、实际日志及冻结
   v2 契约，返回绑定 Wspec 的正式 v2 Review。candidate/review_start/review_pass
   只追加本卡事件；reviewer thread 不得等于作者，修改候选者不能自行审阅。
5. accepted review 后 controller 保留原始 Wspec 身份合并，再合入核准 Rspec。
   **Mspec 相对 Wspec 仅允许以下七个精确记录路径**：

   ```text
   docs/handoff/codex/tasks/S0-01-RECOVERY-SPEC/r1/contract.json
   docs/handoff/codex/events/S0-01-RECOVERY-SPEC.json
   docs/backlog.md
   docs/handoff/codex/reports/S0-01-recovery-spec.checks.json
   docs/handoff/codex/reviews/S0-01-recovery-spec.json
   docs/handoff/codex/reviews/S0-01-recovery-spec.md
   docs/handoff/codex/reports/S0-01-recovery-spec.md
   ```

6. 冲突或其他差异须停止并受审修复；不 cherry-pick 后复用旧身份审阅，不在
   merge 时顺手改规范。Mspec 运行必需集成门、逐项核路径与记录绑定后，controller
   另一个记录提交追加 v2 accept/Accepted backlog。此时 v3 才能作为后续迁移
   冻结规范；本卡全部历史仍按 Cspec 的 v2 重放，不能混版或提前宣布 S0-01 完成。

protocol 中还须按已审设计冻结后续 S0-01 的新 B/C/W/R/M：新 B 包含规范接受
及授权证据；C 仅 r3 contract、首 legacy_import 事件及 backlog，先过门再
dispatch；worker 从 B，W 后单独 R；只在 HEAD=W 重检，正式独立 review 后保留
W 合并；M 相对 W 只允许该任务逐项列出的 contract/events/backlog/checks/review/
attempt-2.md 六记录路径，集成门后再 accept。不得拿 C/R 当 W 或追认旧 B/W。

## 验收、拒绝路径与失败计数

本卡在 Cspec 冻结的 `checks` 精确为（无新增运行工具）：

| id | argv | kind / completion | minimumExecuted | timeoutMs |
| --- | --- | --- | --- | --- |
| diff-check | `["git", "diff", "--check"]` | command / exit | null | 60000 |
| scoped-gate | `["./scripts/test_changed.sh"]` | command / scoped-gate | null | 1800000 |

运行 scoped gate 必须以 Bspec 为 TEST_CHANGED_BASE、GATE_SELFTEST_RUNNING=0、
详细日志放入获准 outputDir；runner 沿 v2 固定环境规则。不声称尚未实现的 S0
工具已经强制验证；bootstrap controller 手工核同一 trace、真实 argv/HEAD/退出与
完成证据。超时、启动错误、必需门 skipped、零输入、缺日志或完成标记均拒绝。

controller/reviewer 另核 `git status --short`、`git ls-files --others --exclude-standard`、
Bspec→Wspec 与 Wspec→Rspec/Mspec 的逐路径 diff、链接/任务 ID、全部输入 blob、
七卡 revision、v2/v3 文本一致性和 RFC 不变量。新增文件不由普通 diff 替代检查。
只运行实际存在且适用的门；本卡是文档工作，不执行或预称不存在的 Node 测试通过。

以下测试名称必须落入 S0-02 r3，均标 **planned**，本卡不写测试实现：

```text
legacy_import_is_single_first_event_for_s0_01_r3_only
legacy_import_preserves_unknown_facts_and_reserves_both_allowances
legacy_import_requires_frozen_evidence_refs
legacy_import_starts_next_attempt_at_two_without_candidate_binding
legacy_reservations_survive_revision_block_and_resume
legacy_import_block_resume_start_preserves_unknown_until_attempt_two
legacy_import_revise_start_uses_next_contract_without_resetting_unknown
legacy_pending_numeric_spent_including_zero_is_rejected
legacy_pending_requires_resume_before_start_or_revise
legacy_pending_null_is_rejected_after_first_start_and_in_ordinary_tasks
legacy_import_then_one_luna_failure_requires_architect
legacy_import_never_reopens_attempt_one_or_fallback_allowance
legacy_import_cannot_accept_without_new_candidate_and_independent_review
ordinary_events_reject_null_spent_and_legacy_fields
v2_rejects_legacy_import_and_mixed_schema_history
invalid_import_is_not_partially_replayed
```

普通事件拒绝 null 的测试须排除严格合法 pending 分支。S0-03 卡追加缺/改旧
报告/draft、缺祖先、输入 blob 不匹配、假 review、HEAD=C/R 的拒绝 fixture，并
覆盖 evidence-only W 的完整配置读取；S0-04 覆盖恢复保留 reservations、旧 writer
停止未知即拒绝；S0-05A 覆盖完整 v3 replay/错误退出；S0-05 覆盖隔离的 import→
attempt 2→review→accept 及失败停机。S0-06 依真实结果汇总，不提前接管 controller。

本卡的 architect attempt 失败按冻结 v2 直接进入 NeedsArchitect，不增加 Luna
失败；环境 block 不伪装 attempt_fail，恢复保留编号和累计工作时间。禁止用新 ID、
revision 或会话反复重开。**本卡计数完全不消费、归零或重分类 S0-01 旧 attempt、
未知失败或未知耗时**，其历史 reservation 仅由后续已接受 v3 迁移决定。

必要未决事实由后续 AUTHORISE 收敛：旧 writer/等待审批/检查进程实际停止、
common-dir 锁归属与无冲突、可用强模型及实际嵌套或 §8.2 转发路径。不足则保留
`S0_01_PRIOR_WRITER_STOP_UNKNOWN` 等具体阻塞，不重复审计空转。发现额外旧
attempt、更多失败或 fallback 使用证据与单一实例假设冲突时，停止导入并更新
审计、重新独立审阅；本卡无权自行扩大迁移例外。若接口仍需超出已审设计的
决定，报告具体 design blocker，不让后续 worker 自行选择行为。

所有旧 Git 提交、claim、配置、attempt-1/record-repair 报告与 r2 draft 原样保留。
不 reset、删除、改写历史，不将 draft 改成 contract.json，不补造旧 actor/计时/
claim/start/failure，不创建或触碰 `events/S0-01.json`。本卡不授权产品网络、
读取凭据、真实客户操作或突破任何现有安全门。

## 交接与工具清单

报告须含本卡 revision/attempt/Bspec/Wspec、diff --stat、新文件、实际门与未运行项、
复用/新增工具、规范变更与未决事实、恢复摘要，随后停止并请求独立候选审阅。
controller 在正式 accepted、集成门和记录完成后才发布后续可执行卡；不能把本卡
产出的 v3 文本或 reviewer 的单一设计结论当作 S0-01 运行接受证据。

已核对的复用清单：

- WorkflowRunner：`crates/workflow/src/lib.rs:112`。
- AuthorityStore：`crates/authority/src/lib.rs:167`。
- AuditLedger / verify_chain：`crates/audit-ledger/src/lib.rs:42,135`。
- ModelProvider：`crates/model/src/lib.rs:118`。
- policy_decision_digest：`crates/capability/src/v2.rs:802`。
- evaluate_prepared：`crates/policy/src/lib.rs:281`。
- consultant-playground fixtures：`crates/consultant-playground/tests/support/`。
- `scripts/test_changed.sh`；Git object/blob/ancestor/diff 与既有 S0 拟议共享接口。

本卡无需调用或重写 Rust 产品工具；私有 persist/签名编码不可复制。不存在的
Python 工具不宣称复用；不新增 runtime、schema、canonical JSON、数据库、
恢复 CLI 或 `recovery.mjs`。新增工具为无。

> 复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。


## Owner-authorised one-time continuation (revision 3)

The owner's instruction “怎么影响小怎么去修，然后快点开发” authorises this bounded
correction. Earlier r1/r2 records and all Git commits remain unchanged as
imperfect historical evidence. This continuation does not assert valid v2 replay
of that history. The original task scope and ten-file ownership remain unchanged.

For this continuation only, freeze a new r3 contract from the committed revision 3
card and all required inputs, including r1/r2 contracts and the historical events.
Store it at `docs/handoff/codex/tasks/S0-01-RECOVERY-SPEC/r3/contract.json`.
Use `docs/handoff/codex/tasks/S0-01-RECOVERY-SPEC/r3/continuation.json` for explicit
owner authority, frozen base, actual dispatch evidence and new segment timing;
this is a one-off record, not an event accepted by the v2 validator. Historical
elapsed time remains unknown; do not insert retrospective start events. The new
segment budget is 1800000 ms, measured from the actual continuation dispatch.

The controller must compare every input with `base:path`, check the complete
required input set and obtain independent freeze review before dispatch. Worker
uses an isolated checkout at that base, commits W before the report-only R, and
is reviewed using the frozen v2 requirements for the new candidate. Include the
r3 contract/continuation instead of the r1 contract in the enumerated integration
record paths; preserve the old event file. No further recovery cards are permitted.
Another record mismatch stops execution. This exception does not accept S0-01,
reset any known failure, enable v3 early or change any product security boundary.
