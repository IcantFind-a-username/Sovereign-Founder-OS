# Codex lane：施工、验收与恢复协议

**Target / 规范冻结，运行器尚未实现。**本文件是开发流程的唯一详细规范；状态 schema 与工具接口见 [contracts.md](contracts.md)，模型配置见 [models-and-goals.md](models-and-goals.md)，领取任务先读 [README](README.md)。安全设计仍服从既有 RFC。

## 9. 自动施工协议：可恢复、可验收、有限重试

本节是要实现并试演的流程规范。**在 S0 验收之前，依靠主线程明确执行这些步骤，不宣称已有机器强制保障。**

### 9.1 一个控制器、一个写入任务

初期默认串行施工。一个 controller 持有一个执行目标；同一时刻只有一个产品 worker 写入。Reviewer 在 worker 停止之后读取固定 candidate。

如果以后并行：每个 worker 使用独立 worktree；独占写入文件集必须无交集；共享 Cargo.toml、Cargo.lock、mod.rs、lib.rs、全局测试 fixture、backlog 都算写入文件。只是“不同功能”不代表可以并行。

Controller 先完成：

1. 核对 repo、基线、工作树与正在运行的旧流程，保留用户已有改动。
2. 从 backlog 选择符合依赖的卡，刷新工具清单和文件所有权表。
3. 冻结 task revision、接口、验收、预算、worker/reviewer 模型。
4. 通过项目门后提交 claim，再分派。每张卡必须有且只有一个 claim owner。
5. 运行中只让 worker 修改白名单源文件和自己的报告；backlog、任务卡、验收规则和最终状态由 controller 维护。

这与[旧人工 handoff 协议](../README.md)的 worker 自行改 backlog 不同。新 Codex lane 必须在 S0-00 明确登记这一差异；旧流程继续按旧规则运行，不能混用。同一项工作不得同时被两条 lane 认领。

### 9.2 状态与恢复

事件名、状态与合法转换的唯一规范为 [contracts §3](contracts.md)。本文件不维护第二份状态表或字段定义。没有从候选交付直接跳到下一张卡的路径；worker 的“完成了”表示请求验收。

阻塞记录保存来源、原因、解除条件、当前 attempt 与已消耗预算。恢复前核对冻结输入、基线、候选、旧 worker 已停止和实际解除证据。未终结 attempt 沿用编号与预算；已终结编号不可复用；原候选恢复后送审。受约束内容变化使旧审阅失效，累计失败不会因暂停、重开或换会话清零。

任务级 Blocked 与平台 Goal 生命周期分开处理，不能把阻塞描述成目标已经完成。

### 9.3 “失败两次”的精确定义

一次 attempt 从 controller 分派开始，到一个终结结果结束。以下任一情况消耗一次失败额度：

- worker 宣布候选交付后，必需的测试或校验未通过；
- worker 在本卡约定的工作预算内无法达到交付条件，且诊断是实现/理解问题；
- 强模型因行为、边界、兼容性、测试有效性或越界修改拒收该次候选。

同一 attempt 中的这些问题只计一次。正常 TDD 的预期 RED 不计失败。错误模型、依赖服务不可用、配额耗尽、网络中断和构建环境缺失归为环境阻塞，单独计数，不能把 worker 的实际代码失败伪装成环境问题。

默认约束：

- Luna 最多 **2 个失败 attempt**，之后必须停止施工并交强模型；不得开新会话清零。
- attempt 内测试/修复循环有明确上限和时间预算；建议首轮试演每卡最多 30 分钟、候选交付前最多 3 轮针对性修复。具体值在卡中冻结，不作为全项目永久 SLA。
- 对同一外部故障最多做一次有意义的恢复检查；仍无进展则报告具名阻塞。不得无限启动空任务。
- 第二次失败由 controller 保留两次诊断并标 NeedsArchitect；旧兼容标签 needs:fable 只有架构角色可解除。
- Fallback 一轮后必须产出通过验收的补丁、可施工的新卡，或具名阻塞。重复失败不能变成无限强模型循环。
- 重新拆卡保留原任务 lineage 和所有失败记录。只有问题、范围或接口实质改变才产生新 revision；重命名不重置计数。

### 9.4 强模型怎样验收并规划下一步

Reviewer 保持产品源码只读。需要写 target、日志或临时 fixture 的检查由 controller/gate runner 在隔离的干净 candidate worktree 运行；reviewer 核对冻结 argv、确切 HEAD、实际日志与代码，必要时要求重跑。不得因为 reviewer 的只读 sandbox 拒绝构建写入就跳过 gate，也不因此给 reviewer 任意改源权限。

每次验收读取：

- 任务卡及其 revision、冻结的接口和适用 RFC；
- 基线与候选提交之间的完整 diff，包括新文件；
- 测试的行为、实际执行结果、退出码、是否零测试/跳过；
- worker 报告、偏差、两次以内的失败记录；
- 与现有用户路径、边界和文件体积的关系。

验收只接受三个结果：

| 结果 | 必须附带 | 后续 |
| --- | --- | --- |
| accepted | 对应 candidate、验收证据、集成条件、下一卡 ID 或里程碑终点 | controller 完成集成与最终状态记录，再选卡。 |
| changes_requested | 可复现问题、文件位置、对应契约、最小修复范围 | 累计失败；尚未满两次才交回 Luna。 |
| blocked | 具体缺失条件、已做检查、解除条件 | 不消费无关产品卡来伪装进度。 |

强模型应读取实际 diff，不仅阅读 Luna 的总结。如果它修改了代码，该次角色变为作者；由另一个独立 reviewer 上下文完成验收。独立上下文提高交叉检查能力，不意味着统计上完全独立，更不提供绝对正确保证。

每个里程碑完成后，强模型再审视业务价值、未解决风险和下一阶段接口。目标未变时继续计划内的 Ready 卡；不得因为“还有额度”擅自扩成整个 OS 的无限开发。

### 9.4.1 Reviewer-only 里程碑卡

S0-06、S1-11 是对前置候选的独立汇总验收，不产生新的实现 candidate，不分派 worker、不创建实现型 TaskContract/attempt，也不消耗 Luna 失败额度。它们的精确输入由对应卡的依赖和实际 candidate 列表构成。

Reviewer 对每个前置实现卡调用已有证据验证，并执行该里程碑门，随后提交唯一报告：gate ID/revision、各 task/candidate/review 引用、实际命令与结果、未完成项、accepted/changes_requested/blocked、下一张卡。Controller 核对这些引用与队列一致后才勾选里程碑卡。发现实现问题生成或退回具名修复卡；reviewer 不在验收报告任务里修改代码。

该报告只是汇总已有执行证据，不能代替任何缺失的 worker 检查或独立 review。没有匹配的前置验收记录，就不能完成里程碑卡。

### 9.5 持久记录与防旧结果误用

建议持久目录（**计划路径，S0 中创建**）：

~~~text
docs/handoff/codex/
  cards/        # 冻结卡：controller/architect 维护
  events/       # 每次状态变化一个新 JSON 记录；Git 追踪
  reports/      # 每次 attempt 的 worker 报告
  reviews/      # 独立 reviewer 结论
  contracts.md  # 字段/状态/工具接口唯一规范
.harness/codex/ # 原始日志、临时结果；不作为唯一完成证明
~~~

精确字段与事件格式只在 [contracts §2](contracts.md) 维护；运行记录绑定任务、冻结规范输入、基线、candidate、实际模型、attempt、检查、review 与后继任务。锁位于 Git common dir，路径与恢复规则见 contracts §5。

约束：

- 不记录 API key、客户数据或完整敏感 prompt。
- candidate_commit 指向已完成产品改动的提交；报告/验收记录可在后续仅记录提交中保存，避免“记录自身 HEAD”的循环。
- 若后续提交改变产品、测试、验收配置或其他受约束文件，原验收失效。仅记录追加也须通过独立的路径与事件校验，不能依文件后缀放行。
- 基线或集成目标变化后，重新核对 diff 和必需集成门。工作树脏状态不能被已有绿色报告覆盖。
- 以 Git 历史和可重建状态为依据；发现半写入、非法转换、重复 attempt、失败历史被删除或 head 不匹配时拒绝继续。
- 锁不能只用固定睡眠或过期时间自动抢占。恢复前确认原 worker 已停止且基线一致；不能在旧进程仍运行时重新派写入任务。

轻量实现优先：原生 Goal/子 Agent 负责启动与等待；一个独立的校验工具负责状态、证据和路径检查。先不写长期驻留 daemon，不引入 Temporal 或新数据库。JSON 使用标准序列化；本记录不发明第二套加密签名或规范化协议。

**边界提醒：**在所有开发 Agent 都有相同文件系统权限的环境里，Prompt、角色 TOML 和仓库脚本不能防止恶意 Agent 篡改一切。上述机制先提供可发现、可复核的开发纪律；更强隔离需要独立执行身份、受保护 reviewer/CI 与权限配置。不得把这套施工协议包装成产品安全内核的证明。

### 9.6 Git、旧自动化与对外动作

- 允许前缀沿用 CLAUDE.md：feature/、hotfix/、fix/、chore/、docs/、refactor/。旧 handoff 文档中的 test/ 不覆盖现行规则。
- 一张卡一个 worker 任务上下文和分支；所有提交沿用仓库 owner 身份与 conventional commits，不加 AI 署名。
- Worker 不合并、不 push、不创建 PR。
- 新 lane 默认集成到本地专用 integration 分支；写入 main、push、发布、部署和对外发送不由本蓝图自动开启。
- 本地整合只接受经过审阅的确切提交；冲突回到修复卡，不能在合并时顺手改逻辑。
- 不使用 git reset --hard 清除用户工作，不自动删除失败分支或证据。
- 旧 nightly runbook 与人工 handoff 保持原样。S0 检查是否存在仍在运行的流程；必须解决重复 claim 和文件冲突后才能启用新 lane。暂停或修改既有自动化要根据当时明确授权执行。

## 10. 适用门槛与工具复用

### 10.1 验证矩阵

| 变化范围 | worker 每卡必需门 | reviewer / 集成门 |
| --- | --- | --- |
| 文档 | 链接/任务引用检查、git diff --check、test_changed | 检查与 RFC/源码一致，不能编造运行证据。 |
| Rust crate | 对应 crate 和有必要的 adversarial tests、fmt/clippy、文件体积、test_changed | 每波一次全 workspace test/clippy/fmt/file-size；scope 变化重新验。 |
| 前端 | 新路径对应 checkJs/tsc、浏览器路径与安全输出验证、test_changed | 现有 UI 兼容性；可用性结论需真实观察。 |
| scripts/config | 工具本身的有效失败/零输入/缺运行时测试、test_changed | 调度状态与错误处理试演；脚本变化按当前 gate 可能触发全量。 |
| 未来 Python worker | 项目定义的 pytest、ruff、mypy；根与 tests 的 conftest、共享 fixtures | 引入 Python 后把这些门纳入 CI；不能用 Rust 测试替代 Python 验收。 |
| 新运行依赖/发布 | 许可证与锁文件检查、最小运行实验 | 依赖审计、release build 与相应 CI；必须记录网络/模型/资源边界。 |

仓库常用命令：

~~~bash
./scripts/test_changed.sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
./scripts/check-file-size.sh
npx -y -p typescript@5.5.4 tsc -p apps/cli/assets/tsconfig.json
git diff --check
~~~

S1 最终还需遵守 v2 计划中更严格的 all-features clippy、release build、Playground 自身 tsconfig、真实进程隔离和既有 UI transcript 比较。

当前 test_changed 在缺 npx 时可能报告 SKIPPED frontend tsc；**ALL GREEN 字样不等于本卡所有必需门都执行过**。新 lane 必须拒绝必需检查的 skipped、零测试、无完成标记或运行器错误。未来测试名先用 --list 等方式确认存在，再运行筛选，禁止“筛到零个也过关”。

Python 并行协议中的 pytest/ruff/mypy 只对实际 Python 工作适用，本 Rust 文档任务不虚构它们的运行结果。同输入表驱动、fixture 复用和序列化纪律仍应在对应语言落地。Rust 已签名类型的 serde 声明顺序不能为了通用规范化而改变。

### 10.2 分派前必须附上的真实工具清单

| 已有实现 | 用途与限制 |
| --- | --- |
| crates/workflow/src/lib.rs：WorkflowStep / WorkflowRunner / 私有 persist | 当前步骤/收据/checkpoint；persist 不是可随意跨 crate 调用的公共 helper。 |
| crates/authority/src/lib.rs：AuthorityStore | 现有授权消费、bundle 与撤销；先确认调用路径。 |
| crates/audit-ledger/src/lib.rs：AuditLedger / verify_chain | 已有链验证；不能由“链合法”推出新鲜度与每个业务字段真实性。 |
| crates/model/src/lib.rs：ModelProvider | 现有网关接口；真实 egress 必须补 RFC 0004 边界。 |
| crates/capability/src/v2.rs：policy_decision_digest / 私有 canonical_claims | 已有摘要/规范化路径；不可重写相同签名编码。 |
| crates/policy/src/lib.rs：evaluate_prepared | 现有 prepared invocation 策略检查。 |
| apps/cli/src/workspace/mod.rs：Store；compose.rs：私有 compose_email；verify.rs：verify_export | 现有产品路径可参考/复用；S1 leaf 禁止依赖它们。 |
| crates/contracts/tests/signed_shape.rs | 锁定已签名对象字段与顺序。 |
| crates/consultant-playground/tests/support/ | 已有边界扫描和 fixture 支持；禁止再写一套同用途源码扫描器。 |

用户并行协议列出的 benchmark/artifacts.py、report.py、metrics.py、matcher.py、review/executor.py 在本仓库不存在；不能声称已经复用了它们。新的运行时若需要一个共享工具，先在卡中申报用途和不能复用现有实现的原因。

每份任务简报末尾附：

> 复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。

每波验收扫描重复实现、过大文件与无消费者 re-export。按任务源码范围查找原子写、摘要、JSON 编码、fixture 等同用途代码；发现重复就转为当前修复或专门先行卡，不放任形成第二套基础库。

## 11. 任务卡与报告模板

模板是给 controller 使用的，不是给 worker 自行改变验收条件的入口。

详细任务的唯一正文在 cards/。Controller 按 [contracts §2](contracts.md) 生成并冻结 TaskContract，而不从本文件复制另一份 YAML schema。

分派简报按此顺序展开：任务 ID/revision → 当前基线 → 用户目标 → 精确写入/禁止集合 → 已冻结接口与不变量 → 正向/拒绝路径测试 → 必需门 → 角色/预算 → 唯一报告路径 → 实际工具清单。不附整个会话历史。

每次 worker 报告必须包括：

~~~text
任务 / revision / attempt / 基线 / candidate commit
实现的用户行为
git diff --stat（相对该卡基线）
所有新增文件（包含未跟踪检查）
测试与实际结果；未运行、跳过、失败分别写明
复用了哪些已有工具；新增了什么工具
偏离与发现的问题
remaining work
请求验收；不自行开始第二张卡
~~~

Reviewer 报告包含：reviewed commit、accepted/changes_requested/blocked、问题与证据、必需检查、实际集成条件、下一张卡与理由。报告模板里不能预填“全部通过”。
