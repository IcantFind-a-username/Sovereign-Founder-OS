# S0 开发校验器契约 v2

**规范 v2 冻结；代码尚未实现。**本文件只定义开发工具，不提供产品授权、加密存储或恶意 Agent 隔离。状态语义的唯一上位规范是 [protocol.md](protocol.md)。v2 在首次实现前补齐历史重放、共享接口与提交路径；不得把旧文档验收当作 v2 运行验收。修改此契约必须增加版本，并重新审阅受影响卡。

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

下述类型是语义契约，采用普通 JSON 可表示的数据。对象拒绝未知字段；整数字段必须为有限整数；记录中的仓库路径不得为空、绝对、含 .. 或 NUL。可空字段必须显式为 null，不使用缺失值猜测状态。函数的 repoDir/outputDir 与 CLI 输入文件位置属于 controller 提供的宿主路径，读取规则另见 §7，不混作记录中的仓库路径。

~~~typescript
type Role = "controller" | "worker" | "reviewer" | "architect" | "fallback";
type Status = "Ready" | "Running" | "Candidate" | "InReview"
  | "Retryable" | "NeedsArchitect" | "Blocked" | "Accepted" | "Superseded";
type EventKind = "claim" | "start" | "candidate" | "review_start"
  | "review_pass" | "attempt_fail" | "block" | "resume"
  | "fallback_start" | "fallback_candidate" | "accept" | "revise" | "reslice";

type Actor = {role: Role; thread: string; model: string; effort: string};
type CheckSpec = {
  id: string; argv: string[]; kind: "test" | "command";
  completion: "exit" | "node-tap" | "cargo-test" | "scoped-gate";
  minimumExecuted: number | null; timeoutMs: number;
};
type TaskContract = {
  schemaVersion: 2; taskId: string; revision: number;
  parentTaskId: string | null;
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
  schemaVersion: 2; seq: number; taskId: string; revision: number;
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
  schemaVersion: 2; taskId: string; revision: number;
  candidateCommit: string; results: CheckResult[];
};
type Review = {
  schemaVersion: 2; taskId: string; revision: number;
  candidateCommit: string; actor: Actor;
  outcome: "accepted" | "changes_requested" | "blocked";
  checkRunPath: string; findings: string[];
  nextTaskId: string | null;
};
~~~

taskId 使用稳定的 S0-01/S1-G01 等 ID；commit/blob 使用 Git 完整 object ID，支持仓库实际 object format，不只检查字符串长度后声称对象存在。allowedWritePaths 是精确仓库相对文件集合，不支持 glob。Worker 的 reportPath 是唯一额外可写文件；controller 的 backlog/event/review 提交与 worker 写集分开核验。

revision、seq、attemptBudgetMs、timeoutMs 为正整数，attempt/spentMs 为非负整数。标识、model/effort/thread 与 argv 元素均为非空字符串；argv 至少有一个元素。checks 至少一项且 id 唯一；dependencies、allowedWritePaths 和 inputBlobs.path 各自不得重复。expectedActors 恰好包含五种 Role，每个值恰有 model/effort。reportPath 不得与源码写集重合。

TaskContract 是 controller 对已提交任务卡生成的分派快照，sourceCardBlob 必须对应 baseCommit 中那份卡；它不成为第二份可自由编辑的任务规范。TaskContract 在 controller 分支的 claim commit 中冻结，worker 从认领前的 baseCommit 分叉；具体提交顺序只定义在 [protocol §9.6.1](protocol.md)。修改卡/参数需新 revision，但不能抹掉同任务失败历史。

inputBlobs 固定本卡读取的 protocol/contracts/models、CLAUDE.md 与适用规范的 Git blob；expectedActors 是从角色规范及实际可用分派路径生成的快照。它们不能在执行途中随文件更新而改变。事件 actor 的 model/effort 必须匹配该快照；异步恢复需重核输入 blob。

每个契约存为 `docs/handoff/codex/tasks/<taskId>/r<revision>/contract.json`，事件保存为同任务的一份完整 JSON 数组 `docs/handoff/codex/events/<taskId>.json`，只追加、不截断。contracts 参数是该 taskId 所有已认领 revision 的契约数组，按 revision 严格递增；最初版本不要求从 1 开始，因为卡可能在首次认领前修订。新契约的 inputBlobs 必须包含全部先前契约文件在新 baseCommit 中的 blob，Git 检查比对这些归档 JSON 与传入历史的结构相等；不能只传最新契约或重建一个零失败事件流。

重放结束时必须已经消费历史中的每份契约，拒绝没有对应 revise 的额外契约。首次未认领的文档修订不构成历史 attempt；一旦出现 claim，之后的规范变化必须走上述 revision 历史。

parentTaskId 仅用于实质拆卡，普通任务为 null。父任务 reslice 事件必须列出子 ID，子卡冻结时引用父契约、事件和拆分 review 的已提交 blob。跨 task 的准入由 controller 与独立 reviewer 核验；纯状态机只检查本任务的 parent 字段、reslice 事件与历史保留，不声称已经查过另一任务。新 revision 不改变 parentTaskId。

JSON 记录不作为强身份认证。模型名、actor.role 与报告内容都要与实际分派和 reviewer 记录交叉核对；同权限工作区的恶意篡改仍超出此工具的保障。

## 3. 纯状态接口与转换

~~~typescript
checkRecordKeys(value: unknown, keys: readonly string[]):
  "EXPECTED_OBJECT" | "MISSING_FIELD" | "UNKNOWN_FIELD" | null;
isNonNegativeInteger(value: unknown): boolean;
isRepoRelativePath(value: unknown): boolean;
validateTask(contract: unknown): Validation;
validateEvents(contracts: TaskContract[], events: unknown[]): Validation;

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

上述三个共享检查函数必须由 state.mjs 导出并测试，后卡直接复用。checkRecordKeys 只接受普通对象或 null 原型对象，拒绝数组/非对象，字段恰好等于 keys；先检查对象，再未知字段，再缺失字段。整数检查拒绝 NaN、无穷、小数和负数。路径检查只接受非空 POSIX 仓库相对路径，拒绝绝对路径、盘符、反斜线、NUL、空段、`.`/`..` 段；symlink 的实际逃逸由 IO 层检查。这些是有限字段检查，不建设通用 schema 引擎。

函数同步、无 IO、无当前时间读取，不修改输入。失败返回 ok=false 和稳定具名 error，不抛出未处理异常。JSON 语法解析错误由调用边界转为 MALFORMED_INPUT；对象字段检查统一复用 state.mjs 中的一处 helper。

validateTask 只检查单份契约形状，返回 state=null、lunaFailures=0、nextAttempt=1、binding=null；通过时 error=null。validateEvents 要求非空契约历史和非空事件；失败时保留最后一个合法事件的重放状态，首事件前失败使用上述初始值。不得部分采纳非法事件。

有效初始事件必须 claim，seq 从 1 连续增长；claim 使用历史中首份契约，之后仅 revise 切到下一份。claim 后状态 Ready、attempt=0、spentMs=0。start 创建 Running，attempt 首次为 1。依赖及父任务准入由 controller 核实；pure validator 不伪装已经查询别的任务。

转换约束：

| event | 允许来源 → 结果 | 特殊检查 |
| --- | --- | --- |
| claim | 空序列 → Ready | actor=controller；契约有效；仅出现一次。 |
| start | Ready/Retryable → Running | 新 attempt 严格递增；Luna 已失败两次则拒绝。 |
| candidate | Running → Candidate | 相同 attempt；worker 报告、candidate/checkRun 非空。 |
| review_start | Candidate → InReview | reviewer 独立 thread，不能等于当前补丁作者；candidate 保持。 |
| review_pass | InReview → InReview | reviewer 的正式结论引用；尚不表示最终 Accepted。 |
| attempt_fail | Running/Candidate/InReview → Retryable 或 NeedsArchitect | 本 attempt 只终结一次；Luna 失败达 2 即 NeedsArchitect；reason 必填。 |
| block | Ready/Retryable/Running/Candidate/InReview/NeedsArchitect → Blocked | resumeFrom 必须等于来源；reason/解除条件在报告；spentMs 保留。 |
| resume | Blocked → 原状态 | actor=controller；无重置失败或 spentMs；Candidate 恢复后仍须 review_start；InReview 保持已登记 reviewer/candidate 绑定；Retryable/NeedsArchitect 只恢复任务状态，不复活终结 attempt。外部检查见 lock/recovery。 |
| fallback_start | NeedsArchitect → Running | actor=fallback；新 attempt；不受 Luna 次数许可；不得换成 Luna。 |
| fallback_candidate | fallback 的 Running → Candidate | 新候选绑定；必须独立 reviewer。 |
| accept | InReview → Accepted | 已有匹配 review_pass；actor=controller；Git/check/review 外部核验全部成立。 |
| revise | Ready/Retryable/NeedsArchitect → 相同状态 | actor=architect；切换到历史数组下一份更高 revision 契约；本次事件按新契约核对 actor/base；保留 attempt、失败和 fallback 总数，不得留下在途候选。 |
| reslice | NeedsArchitect/Blocked → Superseded | actor=architect；children 非空、唯一且不含自身；reason 与拆分 review 必填；旧 worker 已停止由 controller 核验。保留父任务完整历史。 |

architect/fallback 的失败直接进入 NeedsArchitect，不增加 Luna 失败计数；强模型需要重新判断或拆卡，不能利用 Retryable 无限重复。

事件角色固定：start 使用当前契约的 workerRole；candidate 必须保持该 attempt 的实际作者；fallback_start/fallback_candidate 使用 fallback；claim/block/resume/attempt_fail/accept 使用 controller；review_start/review_pass 使用同一独立 reviewer；revise/reslice 使用 architect。失败次数按被终结 attempt 的作者计，不按记录 attempt_fail 的 controller 计。除 block 外 resumeFrom=null；不适用的 candidate/报告/审阅/检查路径为 null，children 仅 reslice 非空，不携带旧候选的审阅绑定。

同一 taskId 的 revision 变化不重置 attempt、失败或 fallback 次数。重放每个事件时，按当时有效的契约核对角色、base 和字段，不拿新契约解释旧事件。revise 清除旧候选与审阅绑定；Running/Candidate/InReview 不能就地改规范。阻塞中的在途工作若必须改变范围，先按 reslice 归档并经审阅产生子卡，不能伪造实现失败来解除阻塞。Fallback 失败进入 NeedsArchitect 并具名报告；校验器拒绝同一 taskId 跨 revision 的第二个 fallback_start，后续只能实质拆卡或保持具名阻塞。

spentMs 是当前 attempt 的累计 worker 工作时间，同一 attempt 单调不减；门、review 和等待时间不充作 worker 工作时间。仅 start/fallback_start 创建新编号并从 0 计时，revise 和任务状态恢复不清零。超过 attemptBudgetMs 不得提交 candidate 或获准 accept，只能记录失败或有证据的环境阻塞。已终结 attempt 不能恢复 Running；恢复 Retryable/NeedsArchitect 后须启动新编号。block/resume 不计作实现失败，不能绕过已发生的 attempt_fail。

稳定错误族至少包含：INVALID_TASK、UNKNOWN_FIELD、INVALID_SEQUENCE、INVALID_TRANSITION、ATTEMPT_REUSED、FAILURE_LIMIT、FALLBACK_LIMIT、ACTOR_MISMATCH、EVIDENCE_MISSING、CANDIDATE_CHANGED、BUDGET_RESET。错误内容不可包含环境秘密。

## 4. Git 和检查执行接口

~~~typescript
inspectCandidate(contracts: TaskContract[], repoDir: string,
  candidateCommit: string): Promise<GitEvidence>;

type GitEvidence = {
  ok: boolean; error: string | null;
  baseCommit: string; candidateCommit: string;
  changedPaths: string[]; untrackedPaths: string[];
  diffStat: string;
};

runChecks(contract: TaskContract, repoDir: string,
  candidateCommit: string, outputDir: string): Promise<CheckRun>;

verifyEvidence(contracts: TaskContract[], events: Event[], git: GitEvidence,
  checks: CheckRun, review: Review): {ok: boolean; error: string | null};

getCommonDir(repoDir: string): Promise<string>;

// 仅 test-support.mjs：建立临时 Git 仓库，在 finally 清理自己创建的目录。
withTempRepo(run: (repoDir: string) => Promise<void>): Promise<void>;
~~~

Git 调用统一采用 execFile/spawn 与 argv，禁止把路径/报告/模型文本拼进 shell。使用历史最后一份契约检查当前候选；核对仓库身份、base 为 candidate 的祖先、sourceCardBlob 与 inputBlobs 均确实绑定 base 中对应路径、历史契约归档内容、实际 changed paths（含 delete/rename 两端）、未跟踪文件和脏工作树。sourceCardPath、报告和原始日志必须位于批准路径；拒绝 symlink 逃逸。

inspectCandidate 不 checkout、不 reset、不合并、不写工作树。要求当前实现 checkout 的 HEAD 等于 candidate，工作树中不得有未提交的产品/验收变更。单独记录提交的审阅在 controller 的 record-only 阶段处理，不能随便忽略所有 docs 路径。

runChecks 接收当前单份 TaskContract，只运行其中冻结的 argv，在已由 inspectCandidate 核验的 repoDir 中运行；不执行工具输出建议的命令。每项有 timeout，超时/无法启动/对应 completion 判据缺失/必需测试数不可确认都不能通过。验证会运行构建过程中的代码，因此不是“read-only 安全沙箱”的同义词；使用任务 worktree 与隔离测试数据。

CheckSpec.kind=test 时 minimumExecuted 必须 >=1，completion 只能为 node-tap/cargo-test；command 时 minimumExecuted=null，completion 只能为 exit/scoped-gate。只支持下表，不识别的输出返回 CHECK_UNVERIFIED。未来适配另立卡，不造通用日志解析平台。

| completion | 正常完成与 skipped 的精确定义 |
| --- | --- |
| exit | 普通命令进程正常退出 0 且无 timeout/signal；允许空 stdout。用于 git diff、fmt/clippy、tsc 等显式命令，不接受用它包装复合测试脚本来跳过计数。 |
| node-tap | argv 显式使用 `node --test --test-reporter=tap`；收到完整 TAP 尾部统计、fail/cancelled 为 0，实际非 skipped/todo 测试数达到 minimumExecuted；任何 skip/todo 使 skipped=true。 |
| cargo-test | 完整 Cargo 测试 summary，全部失败数为 0，汇总实际 passed 数达到 minimumExecuted；ignored 或 filtered-out 非零则 skipped=true。可以有正常的 0-test 二进制，但不能整体零测试。测试名称是否包含卡的必需项由 controller 在 claim 前枚举、reviewer 核对；枚举命令单独冻结为 command，runner 不隐式生成额外命令。 |
| scoped-gate | argv 必须为仓库 test_changed.sh；退出 0 且 stdout 有其 ALL GREEN/steps 完成行。runner 将 TEST_CHANGED_LOG 固定到 outputDir 下该 check 的日志，核对文件存在、日志步骤记录与 stdout 完成行一致；stdout/stderr/详细日志任一含 SKIPPED 则 skipped=true。缺日志/完成行、提前退出或空步骤均不能通过。 |

所有 result 的 completed 只表示按该判据观察到了完成，不能吞掉 timeout/spawn failure。必需项出现 skipped 一律拒收。stdoutPath/stderrPath 和 scoped gate 详细日志都在批准 outputDir，日志不得含秘密；不打印整个进程环境。

verifyEvidence 先用完整 contracts 历史调用 validateEvents，使用其 CandidateBinding 核对实际作者与已登记 reviewer，要求 Review.actor 的 role/thread/model/effort 与该绑定完全一致，且 reviewer thread 不等于作者。再按当前契约核对 task/revision/candidate、每个必需 check 恰有一份结果、argv 一致、exitCode=0、completed=true、skipped=false、测试数满足要求与 matching accepted 结论。它不会把未实际执行的 JSON 声明变成可信证明；reviewer 检查 runner 日志并运行适用集成门。

## 5. 锁与恢复接口

~~~typescript
acquireLock(repoDir: string, owner: LockOwner): Promise<LockResult>;
releaseLock(repoDir: string, owner: LockOwner): Promise<LockResult>;
checkResume(contracts: TaskContract[], events: Event[],
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

S0 工具在尚未实现前，由 [bootstrap controller](models-and-goals.md#bootstrap-controller) 按相同规则手动检查；它可以是强模型主线程，也可以是 Luna 主线程显式调用的强模型子任务。不得要求 S0-02 的首个提交先通过尚不存在的 S0-03 工具。

实现当前卡的测试、运行 ./scripts/test_changed.sh；提交 worker candidate；写独立报告；reviewer 读 diff 并运行适用门；controller 维护队列和最终记录。脚手架实现卡允许修改其负责的工具及测试，**不允许顺手修改其他已验收工具、协议或自己的验收条件**。

以后规范和实现一起演进时，应有专门变更卡：先由 architect 冻结新版本，再让 worker 实现。运行中的旧 revision 不因文件更新而悄悄改变。

## 7. Controller 的单一 CLI 入口（S0-05A）

~~~text
node scripts/codex-control/cli.mjs <operation> <request.json>
~~~

只接受这两个位置参数，拒绝未知 operation、字段或额外参数。request.json 按下表读取；路径由 controller 在冻结卡/隔离 worktree 中提供，不能从模型输出取 shell 命令。

request.json 与其中的契约/事件/checkRun/review 输入文件位置可以是 controller 指定的绝对宿主路径；相对输入位置统一相对调用 CLI 时的 cwd 解析，不相对 repoDir 猜测。这样可以从记录分支 C 读取契约，同时在候选 worktree W 核验源码。记录内容中的 sourceCardPath/inputBlobs/reportPath 等始终是仓库相对路径。repoDir 指向候选 checkout；outputDir 必须落在该 checkout 的 `.harness/codex/` 内，实际解析后拒绝 symlink 逃逸。输入位置必须由 controller 从已核对的记录 worktree 选择，CLI 不凭此声称具备跨工作区访问隔离。

| operation | 精确 request 字段 | 处理 |
| --- | --- | --- |
| validate | contractPaths, eventsPath | contractPaths 是按 revision 递增的非空路径数组；逐份解析/validateTask，再 validateEvents。 |
| inspect | contractPaths, repoDir, candidateCommit | 加载完整契约历史后 inspectCandidate。 |
| checks | contractPaths, repoDir, candidateCommit, outputDir | 先对完整历史 inspectCandidate，再用当前契约 runChecks。 |
| verify | contractPaths, eventsPath, repoDir, candidateCommit, checkRunPath, reviewPath | 重新 inspectCandidate，再 verifyEvidence；不采信旧 GitEvidence 文件。 |
| acquire / release | repoDir, owner | 依对应锁接口处理，owner 使用 LockOwner 形状。 |
| resume | contractPaths, eventsPath, observation | checkResume；observation 使用 ResumeObservation 形状。 |

CLI 只包装上述接口，不派发模型、不 commit、不改变任务状态文件。stdout 只输出一个 JSON 结果，stderr 用于工具诊断；exit 0=检查通过，1=规则拒绝，2=输入/运行时无法完成。error 不得回显秘密。实际工具未完成时，不能因打印 JSON 或捕获异常后默认 return 而变成 exit 0。

checks 的日志进入 outputDir；结果由 CLI JSON 返回，controller 将确切结果保存到冻结的 checkRunPath。接受任务前的 verify 必须在干净候选 worktree 上重核 actual HEAD；最终事件/backlog 在 controller 记录阶段更新。reviewer-only 里程碑卡用此入口验证前置实现卡，再按 protocol 写里程碑报告。
