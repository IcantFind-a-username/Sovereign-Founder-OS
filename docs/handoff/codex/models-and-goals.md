# 开发角色、模型配置与有限 Goal

**Target / 配置示例尚未安装。**本文件是开发角色与模型选择的唯一规范。任务卡用角色名引用这里，执行报告记录实际调用证据。见 [总入口](README.md)、[协议](protocol.md)。

## 8. 开发用 Agent 与产品中的 AI 员工是两套系统

产品 Agent 帮创始人经营公司；开发 Agent 帮我们写这个仓库。它们可借鉴相同的“建议与验收分离”原则，但拥有完全不同的工具、凭据和安全边界。Codex 订阅也不自动成为产品的模型 API 服务。

产品首次使用的提供者、凭据边界以[蓝图 §7.1](../../product/founder-os-execution-blueprint.zh-CN.md#provider-boundary)为准；录入步骤只维护在[provider-setup](provider-setup.md)。它们不改变本文件的开发模型配置。

用户要求可以落实为：

| 开发角色 | 建议模型 | 负责 | 交接点 |
| --- | --- | --- | --- |
| Bootstrap architect | 当前强模型；固定配置候选 gpt-6-astra / high | 冻结协议、第一批卡、接口与验收 | 产出可施工的卡，完成 S0 试演。 |
| Controller | 默认 gpt-5.6-luna / medium | 按规则选卡、启动/等待子任务、记录状态、提交给 reviewer | 不自行降低验收、解除设计阻塞或宣布自己的补丁获批。 |
| Worker | gpt-5.6-luna / medium | 一卡一个任务上下文，测试先行，限定文件施工 | 交付 candidate 与报告后停止。 |
| Reviewer / planner | gpt-6-astra / high | 核对实际 diff、门槛和报告；验收；选择或细化下一张卡 | 每卡必须返回结构化 accepted / changes_requested / blocked。 |
| Fallback | gpt-6-astra / high，单独任务上下文 | 第二次实质失败后定位、修复或重新拆卡 | 自己改的代码再交独立 reviewer 上下文验收。 |

模型名称以当前账号真实可调用结果为准。如果指定的强模型不可用，应明确记录阻塞；不能继承 Luna 默认值后假装完成了强模型验收。

第一阶段建议由当前强模型主线程完成 bootstrap，并亲自验收首张产品卡；流程通过后，日常主线程可以改为 Luna medium，强模型只在规划、验收和兜底时运行。

“交给我兜底”的持久实现是**强模型角色 + 仓库内的设计和记录**。不能承诺每次都唤醒同一个已经结束的会话实例，或依靠某个会话永不丢失的记忆。

### 8.1 已有平台能力与本项目要补的能力

| 项目 | 当前判断 |
| --- | --- |
| 指定子 Agent 的模型和推理档 | 当前会话工具可用；官方也支持项目自定义 Agent 配置。 |
| 父 Agent 汇总子 Agent 结果 | 原生支持；可用于 worker 完成后回报。 |
| Goal 持续推进明确目标 | 原生支持；需要具体完成条件。 |
| 两次失败自动转强模型 | 本项目协议与调度逻辑；不是把默认模型改成 Luna 就会出现的能力。 |
| 强模型逐卡验收、验收后发下一卡 | 需要明确分派、等待、结果绑定与状态检查。 |
| 电脑睡眠/关闭应用后永不停止 | 不能承诺；本地运行依赖机器、应用、网络与额度。 |
| 24 小时定时唤醒 | 属于 scheduled automation，与当前 Goal 分开配置。 |
| 强模型审查安全权限请求 | 平台自动权限审核不能替代本项目代码验收。 |

官方资料：[子 Agent 与配置](https://learn.chatgpt.com/docs/agent-configuration/subagents)、[Goal / 长时间工作](https://learn.chatgpt.com/docs/long-running-work)、[定时任务](https://learn.chatgpt.com/docs/automations?surface=app)、[Luna 模型](https://developers.openai.com/api/docs/models/gpt-5.6-luna)。


## 13. 默认 Luna medium 的配置与启动方式

以下是**拟采用的配置示例**，本文件没有把它们写入项目或用户全局配置。S0-01 必须验证当前客户端是否加载这些文件、运行时是否覆盖权限配置，以及实际调用的模型。

~~~toml
# .codex/config.toml
model = "gpt-5.6-luna"
model_reasoning_effort = "medium"

[agents]
enabled = true
max_concurrent_threads_per_session = 2
default_subagent_model = "gpt-5.6-luna"
default_subagent_reasoning_effort = "medium"
~~~

~~~toml
# .codex/agents/founder-reviewer.toml
name = "founder_reviewer"
description = "Review one frozen task candidate and nominate the next bounded task."
model = "gpt-6-astra"
model_reasoning_effort = "high"
sandbox_mode = "read-only"
developer_instructions = """
Read the frozen task, candidate diff, actual checks, and worker report.
Return accepted, changes_requested, or blocked, bound to the reviewed commit.
Do not edit product code, relax acceptance, erase attempts, or authorize effects.
"""
~~~

Reviewer 的只读配置用于评审源码。写构建产物的 gate 由 controller 在隔离 candidate worktree 运行，reviewer 核对确切 HEAD 与日志；分工唯一见 [protocol §9.4](protocol.md)。

Worker 使用同样的必填 name/description/developer_instructions 结构，固定 Luna medium，限制为一卡。Fallback 固定强模型，单独上下文，只处理升级卡；它修改后的 candidate 必须再送独立 reviewer。

官方配置支持单独指定 Agent 的 model 与 model_reasoning_effort；未指定时可能继承。当前运行时权限覆盖可能优先于 Agent 文件，因此 read-only 字样必须以真实工具行为验证。[配置依据](https://learn.chatgpt.com/docs/agent-configuration/subagents)

若当前分派工具只接受 model/effort 而不接受自定义 Agent 名称，就显式传入模型和推理档，并携带相同的冻结简报；不能声称只创建 TOML 就已经保证分派正确。模型身份能核实到什么程度，应在 smoke report 写明，不接受模型在文本中自称某个型号作为唯一证据。

### 13.1 当前推荐 Goal：交付全流程可演示 MVP

**目标规格：Target；尚未启动。**本节是当前完整 MVP Goal 的唯一正文，替换原先分别止于 S0/S1 的启动提示词。验收定位为合成业务数据、真实模型参与、真实可操作界面的集成 MVP；首次模型出网须先完成 S3-M，真实客户数据与发信等业务外部执行仍按 S4/S5 的门槛推进。

将以下内容作为一个持续目标：

~~~text
目标：交付并实际演示 Sovereign Founder OS 的全流程咨询业务 MVP，
让一个人在同一个本地界面中完成：
公司与服务设定 → 客户线索 → 需求分析 → 方案/报价 →
AI 法务审阅 → 创始人审阅与返工 → 交付计划与任务 → 账单/应收草稿 →
跟进与今日经营概览。

首个试点方向固定为新加坡的独立顾问/专业服务一人公司，依据产品蓝图 §1.3。
本目标交付本地浏览器 MVP；桌面安装包后置，遵循蓝图 §5.4。
产品模型首选 Claude，按蓝图 §7.1 和 provider-setup 配置；员工共用受控调用路径，
用版本化岗位 skills 区分职责，不为 MVP 开展后训练。

从包含脚手架文档集的当前工作树或后续分支开始。
唯一施工入口：docs/handoff/codex/README.md。
按该入口读取 CLAUDE.md、backlog、protocol、contracts、任务卡和适用 RFC。
任务、规则和执行记录各自只有一处规范来源，不新建相互竞争的总计划。

开发范围覆盖 S0、S1、S2、S3，以及把 S2 业务界面与 S3 AI 员工接通的最终集成。
S3 必须包含 S3-M 真实模型连接前置与 S3-L 法务/RAG 扩展。
真实网络调用所需的既有核心安全任务按 S3-M00 审核结果前置；
阶段名或合成数据不豁免 RFC，未实现的真实 owner/授权机制不能用 fixture 替代。
阶段验收是检查点：通过后继续规划和执行下一阶段，直到完整 MVP 验收。
S0 脚手架、S1 固定 Playground、独立 Agent 实验都只是中间产物。
进入未冻结阶段时，自动交强模型冻结设计、接口、验收和小任务卡，写入 backlog 后施工。
最终集成缺少任务卡时，同样由强模型先补齐；不得把分别完成的模块当作集成完成。

Controller 和普通 worker 默认使用 gpt-5.6-luna / medium。
架构、逐卡 reviewer 和 fallback 使用 gpt-6-astra / high。
一个 worker 一次只做一张冻结卡；默认只允许一个写入任务。
每卡回报后，强模型检查实际 diff、测试证据和用户行为，再决定下一卡。
同一卡两次实质失败后停止 Luna 重试，保留诊断，交强模型修复或重新拆分；
强模型修复的代码再由独立 reviewer 验收。按 protocol 处理预算、锁与中断恢复。
S0 工具尚未实现时，由强模型承担明确的人工式控制步骤，不假装自动机制已存在。

MVP 必须满足：
1. 有一个文档化启动命令和统一浏览器入口；创始人完成核心路径不需要手动改 JSON。
2. 至少一个完整合成顾问案例从线索贯穿到报价、审阅、交付、账单和跟进。
   各步骤关联同一份业务状态，页面之间无需手工复制内容。
3. 需求分析员、报价助理、交付规划员、质量检查员和 AI 法务助手通过受控模型通道实际运行，
   结果进入上述业务流程；同一模型可以承担多个岗位。
   界面能看到任务状态、草稿、依据和失败；预置文本不得冒充真实模型结果。
4. 至少走通成功路径，以及“退回报价 → 修改 → 重新审阅”的返工路径；
   另有模型超时或不可用的明确失败呈现，不能显示虚假成功。
5. 金额、币种、报价版本、审阅对象和交付引用经过确定性校验；
   修改报价产生新版本，旧审阅结果不能自动批准新内容；
   账单草稿、预计收入、应收和模拟回款分别标明。
6. 界面明确展示合成数据和实验能力边界，支持重置并重复演示。
   沿用 S1 已批准边界；完整业务和模型实验使用强模型批准的扩展面，
   不把 S1 固定 leaf 偷偷改成任意输入、持久化或模型通道。
7. 相关单元、集成、浏览器、边界和兼容性检查实际运行并通过；
   强模型对最终集成候选验收，证据绑定确切提交。
8. 交付时提供可运行版本、启动命令、演示步骤、实际操作截图或录像、
   验收报告和已知限制，并打开或提供可访问的本地预览让我实际查看。
9. 按产品蓝图 §4.4–4.5 和 S3-L 系列卡完成法律 RAG、法务审阅与地区规则的合成演示。
   新加坡、欧盟、美国的地区资料、覆盖状态与未知成员国/州可见；
   首批受控新加坡资料经过真实检索，引用可回到原文、条文与版本；
   在统一报价流程中展示有来源的 AI 建议、证据不足、后端规则拒绝和待复核状态。
   合成限制必须明确标为 demo/系统规则；模型不能自行放行或发布法律规则。
   正式法律内容的专业复核和真实地区准入属于后续 S4，不能伪造为已完成，
   也不把三地完整法律覆盖变成此合成 MVP 的完成前提。

优先形成创始人可见的业务进展。S1 首次可运行时展示预览，但保持目标继续执行。
基础设施只做到支持本目标和既有验收要求的必要程度；
不无故扩展框架、重复造工具或连续用新规划文档代替产品交付。

本轮使用合成业务数据。真实模型必须通过符合已批准实验边界的通道运行；
不能为完成演示开放未知出网、旁路凭据或真实数据访问。
缺模型资源、强模型、权限或环境时，如实记录具名阻塞，
继续可独立完成的工作；不得用模拟结果替代缺失项后宣布整体完成。
在本目标范围内自动完成可逆实现与受审本地集成；
保留已有用户改动，不自动 push、创建 PR、发布、部署、真实发信或操作资金。

只有上述全流程在统一界面实际走通、最终验收通过并可供我查看，
才将目标标为完成。未满足时保持真实进度，不因完成一个阶段而结束整个目标。
~~~

### 13.2 阶段推进与完成判定

S0-06、S1-11、S2-07、S3-06 各自验收其阶段，之后 controller 根据强模型的下一卡继续本 Goal。它们不单独终结本节目标。最终集成与浏览器验收必须有受审任务卡和记录，具体实现契约仍只维护在该卡及适用规范中。

用户若另行明确要求只完成某阶段，才缩小目标。此 Goal 不自动放行尚未冻结的设计、不降低原有测试或安全门槛，也不把合成 MVP 的完成等同于 S4 真实经营 Alpha。

运行期间保持应用、机器和工作树可用；权限、额度、网络和必要人工决策仍然适用。中断后核对持久记录恢复。[长时间运行条件](https://learn.chatgpt.com/docs/long-running-work)

### 13.3 若以后需要每天自动接着做

Goal 解决一次持续目标；scheduled automation 解决以后再运行。初期先把一段 Goal 跑通，再考虑当前线程的 heartbeat。若明确需要每次新任务或独立项目工作，再采用相应 standalone 方式。

定时任务应只继续已选里程碑的 Ready 卡，遵守相同失败记录和停止条件。不能每次调度都生成全新“零失败”的任务。也不能用定时任务绕过强模型不可用、用户决策或并发锁。

本地定时运行依赖机器与应用可用，不能保证关机后继续开发。[定时运行依据](https://learn.chatgpt.com/docs/automations?surface=app)
