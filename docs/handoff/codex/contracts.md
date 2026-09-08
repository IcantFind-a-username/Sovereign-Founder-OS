# S0 开发校验器契约 v1

**规范冻结；代码尚未实现。**本文件只定义开发工具，不提供产品授权、加密存储或恶意 Agent 隔离。状态语义的唯一上位规范是 [protocol.md](protocol.md)。修改此契约必须增加版本，并重新审阅受影响卡。

本状态机处理会交付新 candidate 的实现/设计卡。S0-06、S1-11 这类 reviewer-only 里程碑验收卡按 [protocol §9.4.1](protocol.md) 汇总既有候选证据，不生成虚构的 worker attempt，也不将 reviewer 伪装为 worker。

## 1. 实现边界和工具清单

运行时采用 **Node 22.x**，当前本机已观察到 v22.23.1。ES modules，Node 标准库，无 npm 运行依赖，不导入产品 crates。不修改 scripts/test_changed.sh 或已有安全扫描器以跳过检查。

新工具已申报：任务状态校验、Git/检查证据收集、单 controller 锁。它们解决开发交接问题；现有 Rust WorkflowRunner 不提供模型分派、Git candidate 绑定和此状态机，且私有 persist 不能从 Node 复用。不得因此重建产品工作流、签名、canonical JSON、数据库或长期 daemon。

文件分工：

~~~text
scripts/codex-control/
  state.mjs             # 纯状态/字段验证；共享错误和结构检查只在这里
  state.test.mjs
  git.mjs               # 唯一 Git subprocess 包装
  git.test.mjs
  checks.mjs            # 仅执行冻结 argv；收集真实 exit/completion
  checks.test.mjs
  lock.mjs              # 独占与恢复检查
  lock.test.mjs
  rehearsal.test.mjs    # 隔离的状态/Git/lock 集成 fixture
  cli.mjs               # 单一 controller 调用入口，S0-05A
  cli.test.mjs
~~~

不提前创建空实现、通用框架或 re-export 模块。每张卡只新增其负责的文件。JSON 输出使用 JSON.stringify；不得另建规范化序列化器。Git/blob 标识由 Git 提供；不以重新实现摘要替代它。

## 2. 数据形状

下述类型是语义契约，采用普通 JSON 可表示的数据。对象拒绝未知字段；整数字段必须为有限整数；路径不得为空、绝对、含 .. 或 NUL。可空字段必须显式为 null，不使用缺失值猜测状态。

~~~typescript
type Role = "controller" | "worker" | "reviewer" | "architect" | "fallback";
type Status = "Ready" | "Running" | "Candidate" | "InReview"
  | "Retryable" | "NeedsArchitect" | "Blocked" | "Accepted" | "Superseded";
type EventKind = "claim" | "start" | "candidate" | "review_start"
  | "review_pass" | "attempt_fail" | "block" | "resume"
  | "fallback_start" | "fallback_candidate" | "accept" | "reslice";

type Actor = {role: Role; thread: string; model: string; effort: string};
type CheckSpec = {
  id: string; argv: string[]; kind: "test" | "command";
  minimumExecuted: number | null; timeoutMs: number;
};
type TaskContract = {
  schemaVersion: 1; taskId: string; revision: number;
  sourceCardPath: string; sourceCardBlob: string;
  inputBlobs: {path: string; blob: string}[];
  baseCommit: string; dependencies: string[];
  allowedWritePaths: string[]; reportPath: string;
  workerRole: "worker" | "architect";
  reviewerRole: "reviewer";
  expectedActors: Record<Role, {model: string; effort: string}>;
  maxLunaFailures: 2; attemptBudgetMs: number;
  checks: CheckSpec[];
};
type Event = {
  schemaVersion: 1; seq: number; taskId: string; revision: number;
  kind: EventKind; attempt: number; actor: Actor;
  baseCommit: string; candidateCommit: string | null;
  reportPath: string | null; reviewPath: string | null;
  checkRunPath: string | null; reason: string | null;
  resumeFrom: Status | null; spentMs: number;
  children: string[];
};
type CheckResult = {
  id: string; argv: string[]; candidateCommit: string;
  exitCode: number | null; completed: boolean;
  executed: number | null; skipped: boolean;
  stdoutPath: string; stderrPath: string;
};
type CheckRun = {
  schemaVersion: 1; taskId: string; revision: number;
  candidateCommit: string; results: CheckResult[];
};
type Review = {
  schemaVersion: 1; taskId: string; revision: number;
  candidateCommit: string; actor: Actor;
  outcome: "accepted" | "changes_requested" | "blocked";
  checkRunPath: string; findings: string[];
  nextTaskId: string | null;
};
~~~

taskId 使用稳定的 S0-01/S1-G01 等 ID；commit/blob 使用 Git 完整 object ID，支持仓库实际 object format，不只检查字符串长度后声称对象存在。allowedWritePaths 是精确仓库相对文件集合，不支持 glob。Worker 的 reportPath 是唯一额外可写文件；controller 的 backlog/event/review 提交与 worker 写集分开核验。

TaskContract 是 controller 对已提交任务卡生成的分派快照，sourceCardBlob 必须对应那份卡；它不成为第二份可自由编辑的任务规范。TaskContract 在 claim commit 中冻结。修改卡/参数需新 revision，但不能抹掉同任务失败历史。

inputBlobs 固定本卡读取的 protocol/contracts/models、CLAUDE.md 与适用规范的 Git blob；expectedActors 是从角色规范及实际可用分派路径生成的快照。它们不能在执行途中随文件更新而改变。事件 actor 的 model/effort 必须匹配该快照；异步恢复需重核输入 blob。

JSON 记录不作为强身份认证。模型名、actor.role 与报告内容都要与实际分派和 reviewer 记录交叉核对；同权限工作区的恶意篡改仍超出此工具的保障。

## 3. 纯状态接口与转换

~~~typescript
validateTask(contract: unknown): Validation;
validateEvents(contract: TaskContract, events: unknown[]): Validation;

type Validation = {
  ok: boolean; state: Status | null;
  lunaFailures: number; nextAttempt: number;
  binding: CandidateBinding | null;
  error: string | null;
};
type CandidateBinding = {
  attempt: number; candidateCommit: string;
  author: Actor; reviewer: Actor | null;
  reviewPath: string | null;
};
~~~

CandidateBinding 只能由事件重放产生；不能采信调用方自造的绑定对象。新候选清除旧 reviewer/reviewPath。

函数同步、无 IO、无当前时间读取，不修改输入。失败返回 ok=false 和稳定具名 error，不抛出未处理异常。JSON 语法解析错误由调用边界转为 MALFORMED_INPUT；对象字段检查统一复用 state.mjs 中的一处 helper。

有效初始事件必须 claim，seq 从 1 连续增长；claim 后状态 Ready、attempt=0。start 创建 Running，attempt 首次为 1。Ready 的依赖必须由 controller 核实；pure validator 不伪装已经查询别的任务。

转换约束：

| event | 允许来源 → 结果 | 特殊检查 |
| --- | --- | --- |
| claim | 空序列 → Ready | actor=controller；契约有效；仅出现一次。 |
| start | Ready/Retryable → Running | 新 attempt 严格递增；Luna 已失败两次则拒绝。 |
| candidate | Running → Candidate | 相同 attempt；worker 报告、candidate/checkRun 非空。 |
| review_start | Candidate → InReview | reviewer 独立 thread，不能等于当前补丁作者；candidate 保持。 |
| review_pass | InReview → InReview | reviewer 的正式结论引用；尚不表示最终 Accepted。 |
| attempt_fail | Running/Candidate/InReview → Retryable 或 NeedsArchitect | 本 attempt 只终结一次；Luna 失败达 2 即 NeedsArchitect；reason 必填。 |
| block | Ready/Running/Candidate/InReview/NeedsArchitect → Blocked | resumeFrom 必须等于来源；reason/解除条件在报告；spentMs 保留。 |
| resume | Blocked → 原状态 | actor=controller；无重置失败或 spentMs；Candidate 恢复后仍须 review_start；InReview 保持已登记 reviewer/candidate 绑定；外部检查见 lock/recovery。 |
| fallback_start | NeedsArchitect → Running | actor=fallback；新 attempt；不受 Luna 次数许可；不得换成 Luna。 |
| fallback_candidate | fallback 的 Running → Candidate | 新候选绑定；必须独立 reviewer。 |
| accept | InReview → Accepted | 已有匹配 review_pass；actor=controller；Git/check/review 外部核验全部成立。 |
| reslice | NeedsArchitect → Superseded | actor=architect；children 非空且新 ID；必须引用实质拆分 review。 |

architect/fallback 的失败直接进入 NeedsArchitect，不增加 Luna 失败计数；强模型需要重新判断或拆卡，不能利用 Retryable 无限重复。

同一 taskId 的 revision 变化不重置 attempt 或失败计数。候选/基线变化不能复用先前 review_pass。Fallback 失败进入 NeedsArchitect 并具名报告；禁止自动再开一轮 fallback，除非强模型先冻结新的实质拆分任务。这里的派发上限由 controller 按 protocol 执行，校验器也拒绝同一 task revision 的第二个 fallback_start。

未终结 attempt 的环境中断沿用编号与累计预算；spentMs 单调不减。已经终结的 attempt 不能 resume。block/resume 不计作实现失败；不能用它们绕过已发生的 attempt_fail。

稳定错误族至少包含：INVALID_TASK、UNKNOWN_FIELD、INVALID_SEQUENCE、INVALID_TRANSITION、ATTEMPT_REUSED、FAILURE_LIMIT、FALLBACK_LIMIT、ACTOR_MISMATCH、EVIDENCE_MISSING、CANDIDATE_CHANGED、BUDGET_RESET。错误内容不可包含环境秘密。

## 4. Git 和检查执行接口

~~~typescript
inspectCandidate(contract: TaskContract, repoDir: string,
  candidateCommit: string): Promise<GitEvidence>;

type GitEvidence = {
  ok: boolean; error: string | null;
  baseCommit: string; candidateCommit: string;
  changedPaths: string[]; untrackedPaths: string[];
  diffStat: string;
};

runChecks(contract: TaskContract, repoDir: string,
  candidateCommit: string, outputDir: string): Promise<CheckRun>;

verifyEvidence(contract: TaskContract, events: Event[], git: GitEvidence,
  checks: CheckRun, review: Review): {ok: boolean; error: string | null};

getCommonDir(repoDir: string): Promise<string>;

// 仅 test-support.mjs：建立临时 Git 仓库，在 finally 清理自己创建的目录。
withTempRepo(run: (repoDir: string) => Promise<void>): Promise<void>;
~~~

Git 调用统一采用 execFile/spawn 与 argv，禁止把路径/报告/模型文本拼进 shell。核对仓库身份、base 为 candidate 的祖先、sourceCardBlob、实际 changed paths（含 delete/rename 两端）、未跟踪文件和脏工作树。sourceCardPath、报告和原始日志必须位于批准路径；拒绝 symlink 逃逸。

inspectCandidate 不 checkout、不 reset、不合并、不写工作树。要求当前实现 checkout 的 HEAD 等于 candidate，工作树中不得有未提交的产品/验收变更。单独记录提交的审阅在 controller 的 record-only 阶段处理，不能随便忽略所有 docs 路径。

runChecks 只运行 TaskContract 中冻结的 argv，在所核验 repoDir 中运行；不执行工具输出建议的命令。每项有 timeout，超时/无法启动/缺完成标记/必需测试数不可确认都不能通过。验证会运行构建过程中的代码，因此不是“read-only 安全沙箱”的同义词；使用任务 worktree 与隔离测试数据。

CheckSpec.kind=test 时 minimumExecuted 必须 >=1；command 时为 null。Node tests 使用稳定 TAP 结果，Cargo 测试以测试枚举和实际 summary 交叉确认；不认识的输出格式直接 CHECK_UNVERIFIED，不能“猜通过”。首个版本仅支持冻结卡用到的 Node/Cargo 形式，不造通用日志解析平台。未来增加适配有专卡和拒绝路径测试。

verifyEvidence 先调用 validateEvents，使用其 CandidateBinding 核对实际作者与已登记 reviewer，要求 Review.actor 的 role/thread/model/effort 与该绑定完全一致，且 reviewer thread 不等于作者。再核对 task/revision/candidate、每个必需 check 恰有一份结果、argv 一致、exitCode=0、completed=true、skipped=false、测试数满足要求与 matching accepted 结论。它不会把未实际执行的 JSON 声明变成可信证明；reviewer 检查 runner 日志并运行适用集成门。

## 5. 锁与恢复接口

~~~typescript
acquireLock(repoDir: string, owner: LockOwner): Promise<LockResult>;
releaseLock(repoDir: string, owner: LockOwner): Promise<LockResult>;
checkResume(contract: TaskContract, events: Event[],
  observation: ResumeObservation): Validation;

type LockOwner = {controllerId: string; taskId: string; revision: number};
type LockResult = {ok: boolean; error: string | null};
type ResumeObservation = {
  priorWorkerStopped: boolean;
  blockerResolved: boolean;
  baseCommit: string;
  candidateCommit: string | null;
};
~~~

锁放到 Git common dir 下的独立 codex-lane-lock 目录，保证不同 worktree 共享同一控制器锁；通过单次 mkdir 的排他语义获取。owner 元数据未写完整按占用/损坏处理，不能当作空锁。不是远程租约，不靠 TTL 抢占。

releaseLock 只释放与调用 owner 三字段完全匹配的锁。checkResume 是纯验证，priorWorkerStopped/blockerResolved 必须来自实际工具观察与 controller 记录，不能由 worker 自证。锁异常时保留目录与证据，交 controller 诊断，不能递归删除未知路径。

## 6. 交接与自举

S0 工具在尚未实现前，由强模型主线程按相同规则手动检查。不得要求 S0-02 的首个提交先通过尚不存在的 S0-03 工具。

实现当前卡的测试、运行 ./scripts/test_changed.sh；提交 worker candidate；写独立报告；reviewer 读 diff 并运行适用门；controller 维护队列和最终记录。脚手架实现卡允许修改其负责的工具及测试，**不允许顺手修改其他已验收工具、协议或自己的验收条件**。

以后规范和实现一起演进时，应有专门变更卡：先由 architect 冻结新版本，再让 worker 实现。运行中的旧 revision 不因文件更新而悄悄改变。

## 7. Controller 的单一 CLI 入口（S0-05A）

~~~text
node scripts/codex-control/cli.mjs <operation> <request.json>
~~~

只接受这两个位置参数，拒绝未知 operation、字段或额外参数。request.json 按下表读取；路径由 controller 在冻结卡/隔离 worktree 中提供，不能从模型输出取 shell 命令。

| operation | 精确 request 字段 | 处理 |
| --- | --- | --- |
| validate | contractPath, eventsPath | 解析 JSON 后 validateTask/validateEvents。 |
| inspect | contractPath, repoDir, candidateCommit | inspectCandidate。 |
| checks | contractPath, repoDir, candidateCommit, outputDir | 先 inspectCandidate，再 runChecks。 |
| verify | contractPath, eventsPath, repoDir, candidateCommit, checkRunPath, reviewPath | 重新 inspectCandidate，再 verifyEvidence；不采信旧 GitEvidence 文件。 |
| acquire / release | repoDir, owner | 依对应锁接口处理，owner 使用 LockOwner 形状。 |
| resume | contractPath, eventsPath, observation | checkResume；observation 使用 ResumeObservation 形状。 |

CLI 只包装上述接口，不派发模型、不 commit、不改变任务状态文件。stdout 只输出一个 JSON 结果，stderr 用于工具诊断；exit 0=检查通过，1=规则拒绝，2=输入/运行时无法完成。error 不得回显秘密。实际工具未完成时，不能因打印 JSON 或捕获异常后默认 return 而变成 exit 0。

checks 的日志进入 outputDir；结果由 CLI JSON 返回，controller 将确切结果保存到冻结的 checkRunPath。接受任务前的 verify 必须在干净候选 worktree 上重核 actual HEAD；最终事件/backlog 在 controller 记录阶段更新。reviewer-only 里程碑卡用此入口验证前置实现卡，再按 protocol 写里程碑报告。
