# Founder OS：Luna 施工总入口

**从这里开始。版本 v1，2026-09-09。**本目录是 Codex 自动施工的唯一入口。文档脚手架已建立；模型角色配置、校验器和自动升级仍须按 S0 实现、试演后才能声明可用。

当前推荐目标为 [全流程可演示 MVP](models-and-goals.md)：覆盖 S0–S3 和最终业务/AI 界面集成，阶段验收后继续推进。完整目标正文只维护在该文件 §13.1。

## 唯一事实来源

“全局唯一 truth”指每类事实只有一个规范位置，由本表统一寻址；不会让新文档覆盖既有安全 RFC，也不再维护一份重复的大计划。

| 要找的事实 | 唯一规范位置 | 其他文档的职责 |
| --- | --- | --- |
| 项目硬规则、Git/迭代纪律 | [CLAUDE.md](../../../CLAUDE.md)（AGENTS.md 指向它） | 本目录只说明获准的新 Codex lane 差异。 |
| 产品愿景、当前代码判断、组件研究 | [产品与选型](../../product/founder-os-execution-blueprint.zh-CN.md) | 任务卡引用对应设计，不重复选型结论。 |
| 安全协议、数据/授权边界 | [现有 RFC](../../../rfcs/)、[MANIFESTO](../../../MANIFESTO.md)、[THREAT_MODEL](../../../THREAT_MODEL.md) | 本目录不得降级安全要求；源码与规范不一致是缺口，不是豁免。 |
| 任务是否进入队列、认领与完成 | [backlog](../../backlog.md) | 这里不维护第二份勾选清单。 |
| 一张任务的范围、接口、验收、依赖 | [任务卡](#任务卡)；实现卡引用 contracts | backlog 仅保存 ID、优先级、链接、claim/诊断/完成；详细正文只在卡中修改。 |
| 执行步骤、失败/阻塞/验收/恢复 | [protocol.md](protocol.md) | Prompt 和简报引用它；不得另写一套重试规则。 |
| S0 校验工具的数据形状与函数接口 | [contracts.md](contracts.md) | 任务卡只指向负责实现的接口与命名测试。 |
| 模型、角色与 Goal 入口 | [models-and-goals.md](models-and-goals.md) | 用 worker/reviewer/fallback 角色名引用；运行报告记录实际模型证据。 |
| 阶段方向与远期未冻结工作 | [milestones.md](milestones.md) | 索引性质；对应任务卡出现后，索引不再复制详细契约。 |
| 一次 attempt 的实际经过 | 分派后追加的 events/reports/reviews 与对应 Git 提交 | 它们是执行证据；状态必须与 backlog 的认领/完成一致，冲突即停止检查。 |

执行范围冲突时：先满足上位的 owner 指令和仓库硬规则；安全内容服从适用 RFC；同一任务以冻结的卡及其引用契约为准。发现旧卡、源码或索引不一致时，由 controller/architect 修正唯一源并发新 revision，worker 不自行挑一个较宽松版本。

## Luna 第一次进入仓库的读法

1. 读 CLAUDE.md、本文件和 backlog，只选择本 lane 的一张未完成卡。
2. 读 protocol、该卡、它引用的 contracts 段落；涉及产品时读对应 RFC/Playground v2 计划。
3. 查看所有依赖是否有已提交的验收证据；Spec frozen 不代表已实现，Blocked design 卡不能派给 worker。
4. Controller 写入本次 base commit、卡的 Git blob、精确写入集合、角色、预算和报告路径；运行门后提交 claim。
5. 用新任务上下文分派 worker，等待 candidate；强模型审阅实际 diff 与检查结果，再作最终验收。
6. 一张卡完成后由 reviewer 指定下一卡，controller 才继续。两次失败或边界变更按协议升级。

当前最先可实施的是 **S0-01 配置**与 **S0-02 纯状态校验核心**；默认串行，S0-01 先执行。这些卡的设计已冻结，但执行仍需正常 claim、干净基线和真实模型/环境预检。S0-00 的设计产物由本次文档任务提供，不再安排一轮同内容规划。

## 任务卡

| 卡 | 角色 / 规格状态 | 详细契约 |
| --- | --- | --- |
| S0-00 | 文档契约入口；实现未开始 | 本文件、protocol、contracts 和首批卡共同构成，不另建副本。 |
| S0-01 | worker；Frozen | [项目模型配置与真实分派](cards/S0-01.md) |
| S0-02 | worker；Frozen | [事件状态校验](cards/S0-02.md) |
| S0-03 | worker；Frozen，等待 S0-02 | [Git 与检查证据](cards/S0-03.md) |
| S0-04 | worker；Frozen，等待 S0-03 | [独占与恢复](cards/S0-04.md) |
| S0-05A | worker；Frozen，等待 S0-02–04 | [单一 CLI 入口](cards/S0-05A.md) |
| S0-05 | worker；Frozen，等待 S0-01–04、S0-05A | [试演与报告](cards/S0-05.md) |
| S0-06 | reviewer；Frozen，等待 S0-05 | [脚手架验收](cards/S0-06.md) |
| S1-00 | architect；设计任务已冻结，产出的产品接口尚待裁决 | [Playground 接口与门迁移](cards/S1-00.md) |
| S1-G01 / G02、S1-01–11 | Blocked design/dependencies，禁止提前施工 | [后续 Playground 卡](cards/S1-after-design.md) |
| S2–S6 | Target / Research | [里程碑索引](milestones.md)，按证据逐阶段生成卡。 |

Frozen 表示这张卡自身的任务契约可用；依赖、claim、环境与实际运行状态仍从 backlog/执行证据读取。S1-00 是可以交给架构角色的设计工作，不是可以让 Luna 自行确定安全边界的功能卡。

## 变更规则

- 一项规则只改其规范文件；所有引用仍指向该处。不得添加第二个“最新版完整计划”。
- 卡字段变化就增加 revision；已分派卡的旧 blob 与执行记录必须保留。
- Worker 不编辑自己的卡、上位规范、模型规则、队列或验收器；除非该卡本来就是经架构批准的验收器实现卡，且由独立 reviewer 验收。
- 这里不会把永久安全不变量与 Task 1 源码快照混为一谈。边界演进走 S1-00 与专门门迁移卡。
- 旧人工 handoff 和 nightly 模式不自动采用本 lane。启动前确认没有重复 claim；不得让旧 worker 因遇到新 P1 条目就提前施工。
- 原始日志可放 .harness，最终报告与状态证据须入 Git；不可依赖某次聊天保存唯一记录。

本次只建立文档集，不安装依赖、不改全局模型、不启动 Goal 或 recurring automation。后续以 S0-06 的实际验收结论决定能否启用日常 Luna controller。
