# 开发角色、模型配置与有限 Goal

**Target / 配置示例尚未安装。**本文件是开发角色与模型选择的唯一规范。任务卡用角色名引用这里，执行报告记录实际调用证据。见 [总入口](README.md)、[协议](protocol.md)。

## 8. 开发用 Agent 与产品中的 AI 员工是两套系统

产品 Agent 帮创始人经营公司；开发 Agent 帮我们写这个仓库。它们可借鉴相同的“建议与验收分离”原则，但拥有完全不同的工具、凭据和安全边界。Codex 订阅也不自动成为产品的模型 API 服务。

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

### 13.1 第一段 Goal：把施工机制跑通

以下提示词面向强模型 bootstrap；目标有明确终点：

~~~text
目标：在 Sovereign Founder OS 中在已冻结的 S0-00 文档基础上完成 S0-01 至 S0-06，
交付并验证 Luna medium worker、强模型逐卡 reviewer、两次失败升级和恢复机制。

先读 CLAUDE.md、docs/backlog.md、
docs/handoff/codex/README.md 与适用设计。
一次只派一张冻结卡；先 claim；精确写入白名单；复用工具清单。
worker 使用 gpt-5.6-luna / medium；
reviewer 与 fallback 使用 gpt-6-astra / high。
每卡 worker 回报后必须等 reviewer 与适用集成门通过再推进。
第二次实质失败交 fallback，不清零历史；fallback 的补丁独立复核。
模型、权限或环境不满足时明确记录阻塞，不假装通过。
保留用户已有工作。仅本地分支与经过审阅的本地集成；
不 push、不建 PR、不发布、不部署、不访问真实客户数据或发送消息。
S0 验收完成，交付第一张 S1 Ready 卡和运行报告，结束本目标。
~~~

不应一开始下达“把整个 OS 做到完美”。长目标应该连续，但完成条件必须有限、可测。

### 13.2 第二段 Goal：由 Luna 主线程推进 S1

S0 已验收后，主线程可选择 Luna medium，再启动：

~~~text
目标：完成蓝图 S1 的固定咨询 Playground，并交付可运行、经强模型验收的 Demo。

严格使用已验收的 Codex lane、状态校验器和冻结任务卡。
你担任 controller，按依赖派 Luna medium worker，每次只允许一个写入任务。
每卡交给 gpt-6-astra / high reviewer 验收并规划下一卡；
两个失败 attempt 后停止该 worker，交强模型修复或重拆。
保留现有 sovereign ui、Workspace、资产、导出、校验与全部既有行为。
Playground 仅固定合成数据、四动作、双语、八条路由，无模型/持久化/真实输入。
所有必需门实际执行；不得以零测试或 SKIPPED 通过。
本地分支工作；不 push、PR、发布、真实发送或修改真实业务数据。
完成 S1-11、可运行入口和验收报告后结束，不自动启动 S2。
~~~

启动后保持应用和机器可用；需要时启用 Prevent sleep while running。Goal 仍受权限、额度、网络与人工决策约束；中断后从持久记录恢复，而不是假设任务从未停止。[运行条件](https://learn.chatgpt.com/docs/long-running-work)

### 13.3 若以后需要每天自动接着做

Goal 解决一次持续目标；scheduled automation 解决以后再运行。初期先把一段 Goal 跑通，再考虑当前线程的 heartbeat。若明确需要每次新任务或独立项目工作，再采用相应 standalone 方式。

定时任务应只继续已选里程碑的 Ready 卡，遵守相同失败记录和停止条件。不能每次调度都生成全新“零失败”的任务。也不能用定时任务绕过强模型不可用、用户决策或并发锁。

本地定时运行依赖机器与应用可用，不能保证关机后继续开发。[定时运行依据](https://learn.chatgpt.com/docs/automations?surface=app)
