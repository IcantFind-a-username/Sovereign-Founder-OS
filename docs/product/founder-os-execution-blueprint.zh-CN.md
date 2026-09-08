# Sovereign Founder OS：产品愿景、组件选型与开发方向

**中文执行文件 · 2026-09-09 · v1**

**用途：**给创始人决定方向，给强模型制定开发方向。Luna 的可执行任务与规范统一从 [施工文档集](../handoff/codex/README.md) 进入。

**状态：Target / 执行提案。**本文负责产品与研究判断；施工文档集负责协议、契约和任务定义，不代表其中的产品、Agent 员工或自动调度器已经实现。源码核对基线为 main 的 **933af77**。开发时必须重新核对实际 HEAD；不得把这个基线永久写死到任务执行器里。

English summary: This Chinese execution blueprint connects the founder-facing vision, the verified Rust baseline, component research, and a bounded delivery process using Luna at medium reasoning with independent strong-model review and escalation after two failed attempts. It does not supersede accepted RFCs or activate unattended development.

## 1. 先确定我们究竟在造什么

最终愿景是：**让一个人能够建立、经营和持续改善一家公司，同时始终掌握自己的数据、业务判断和执行权。**

产品应当帮助创始人完成这样一条链路：

> 了解自身能力与资源 → 找到值得验证的机会 → 与客户验证需求 → 形成可销售的服务或产品 → 获客与报价 → 履约与交付 → 记录应收和回款 → 复盘并调整经营。

其中，AI 员工承担调研、整理、起草、检查和在限定权限内执行的工作；创始人保留目标、承诺、授权与重要取舍。模型可以换，服务商可以换，公司记录和业务连续性仍然由用户掌握。

[ROADMAP](../../ROADMAP.md)已经把产品划分为 Founder OS 与 Sovereign Trust Layer 两个相互支撑的层次。之后每个阶段都应同时回答：

1. 创始人现在能完成什么具体工作？
2. 支撑这项工作的最小可信边界是什么？
3. 有什么证据表明两者确实成立？

安全内核是很有价值的起点。下一步应把它转化成能让人看见业务价值的路径。孤立增加安全原语、组织架构图或聊天角色，均不能单独证明 Founder OS 已经向前推进。

### 1.1 不可交易的设计原则

以[宣言](../../MANIFESTO.md)、[威胁模型](../../THREAT_MODEL.md)、[架构](../../ARCHITECTURE.md)和适用 [RFC](../../rfcs/) 为准：

| 原则 | 对开发的直接要求 |
| --- | --- |
| AI 不能授权自己 | 建议、业务校验、权限裁决、执行分别承担责任；另一个 AI 点头也不能代替确定性策略或真实 owner 授权。 |
| 策略是代码 | Prompt、Agent role、SDK guardrail 都不能充当权限边界。 |
| 权限窄、短、可撤销 | 工具调用按主体、目的、资源、具体操作和有效期授权；员工身份不附带永久万能凭证。 |
| 重要动作有可持久验证的证据 | 保存被审阅版本、执行意图、实际结果与失败状态；不把聊天记录当完整审计。 |
| 数据与运行由用户掌握 | 本地状态有明确所有者、可恢复路径和迁移规则；云服务不能成为唯一真相。 |
| 外部内容默认不可信 | 模型输出、网页、附件、插件和 MCP 服务均先作为输入处理，不能自行变成指令或授权。 |
| 先证明恢复，再扩大自治 | 不把导出、重启演示、数据库副本或工作流 checkpoint 称为完整恢复。 |
| 成熟度诚实 | Current、Experimental、Target、Research 各用其名，真实业务、合成样例、模拟模型明确标注。 |

### 1.2 第一类用户与商业假设

首个用户选择**独立顾问或提供专业服务的一人公司**，承接现有咨询业务场景。这一选择降低业务模型复杂度：先处理客户、需求、服务、报价、交付、应收，不急于进入库存、制造、薪资和跨境税务。

第一轮需要验证的假设：

- 创始人需要一个贯穿客户与交付的工作台，而不只是另一个聊天窗口。
- 从需求到可审阅的报价与交付计划，能形成可感知的时间收益。
- 能看到 AI 建议依据、修改结果和授权边界，会提高实际采用意愿。
- 本地数据掌控与可替换模型，对目标用户有价值；价值强弱仍需访谈验证。

这些是待验证假设，不是已经证明的市场结论。Demo 阶段不承诺定价、收入或产品市场匹配。付费方向可以研究安装与支持、托管便利、专业工作流包、团队协作服务；不能用付费功能夺走用户对自身数据和恢复的基本控制权。收入模式、地域、税务与合同责任需要另行验证。

## 2. 当前开发判断：哪些已有，哪些尚未接通

本节来自源码和仓库记录核对，**没有为本文件重新运行全量产品测试或性能测量**。旧文档中的测试数量、模块成熟度可能滞后；执行卡应引用具体代码和最新验收记录。

| 层面 | 基线中的事实 | 对下一步的含义 |
| --- | --- | --- |
| Rust 基础 | contracts、policy、capability、authority、identity、vault、audit-ledger、sandbox、model、effects、execution、artifact、workflow 等已有实现 | 复用已验证机制；不重造第二套签名、策略或执行链。 |
| authority | bundle transaction 和 durable revocation 已在该 crate 落地；capability 的接线仍有未完成 backlog | “底层有方法”和“真实产品路径已使用它”必须分别验收。 |
| 现有 Web UI | Command Center、Workspace、Security Center；零依赖前端 JS + checkJs | 第一阶段保留现有入口，不开展整体 React/Tauri 重写。 |
| Workspace | 公司、客户、固定 Offer/Invoice、审批、局部本地 .eml 与证据链 | 是 Experimental 产品骨架；没有完整 CRM、报价编辑、项目管理、真实付款核对。 |
| owner 与真实副作用 | 现有本地 API 不是独立认证的 owner ceremony；最终收件人和确切输出字节的授权还需完善 | 不能把“后台代签”当作真实人的授权；不可接上真实发送便宣称安全闭环。 |
| Model Gateway | 有路由模拟与 ModelProvider 接口，尚未形成真实模型的完整可信边界 | ProviderTrust::Local、自报标签、localhost 地址都不是本地隔离证明。 |
| Workflow | 有顺序步骤、收据和同目录 checkpoint resume | 可复用已完成步骤记录；尚不能据此声称跨机容错或外部副作用 exactly-once。 |
| Vault v2 | 有独立的非产品 engine crate 与部分构建/边界工作 | 不能据此声称真实业务已经 ActiveV2、可以完整恢复或自动迁移。 |
| Consultant Playground | 只有私有固定 graph、session 初始化与物理边界测试；lib.rs 仍指向 Task 2 | 正好是下一段可交给 Luna 的受控施工面。 |
| Codex 自动施工 | 当前可调用指定模型/推理档的子智能体；未安装本计划的角色配置、状态校验器与自动验收流程 | 原生能力可承载工作流，尚无本项目自动升级和验收的运行证明。 |

源码入口：[Playground](../../crates/consultant-playground/src/lib.rs)、[Workspace](../../apps/cli/src/workspace/mod.rs)、[AuthorityStore](../../crates/authority/src/lib.rs)、[ModelProvider](../../crates/model/src/lib.rs)、[WorkflowRunner](../../crates/workflow/src/lib.rs)。

当前最值得做的是：**完成一个可运行、边界清楚的咨询业务教学 Demo，再用它驱动真实业务模型和 AI 员工接入。**恢复、owner 授权、模型隔离等底层工作按真实功能依赖推进，不必等整个 OS 完成才向用户展示产品价值。

## 3. 从 Odoo、SAP 借鉴什么

Odoo 值得借鉴的是业务对象之间的连续关系：机会关联客户，机会可以形成报价，报价确认后进入订单；里程碑的完成影响可开票数量。报价、订单、账单与实际收款必须是不同事实。[Odoo 机会转报价](https://www.odoo.com/documentation/19.0/applications/sales/crm/acquire_leads/send_quotes.html)、[里程碑开票](https://www.odoo.com/documentation/19.0/applications/sales/sales/invoicing/milestone.html)。

SAP Fiori 的角色导向、简洁、一致与自适应原则适合参考。一人公司同一个人会切换销售、交付和经营者视角，界面应围绕当前任务组织。[SAP Fiori 设计原则](https://www.sap.com/design-system/fiori-design-web/v1-151/discover/sap-design-system/vision-and-mission/design-principles)。

我们的具体取舍：

| 借鉴点 | 在 Founder OS 中的落点 |
| --- | --- |
| 相互关联的业务对象 | 客户 → 机会 → 需求记录 → 报价版本 → 交付项目 → 账单 → 收款记录。 |
| 状态和责任清楚 | 每个对象显示当前阶段、下一动作、负责人、依据与待审事项。 |
| 角色相关的信息 | 首页突出“今天值得做什么”；客户页集中上下文；审阅页集中变化与风险。 |
| 可配置但有默认路径 | 首发提供顾问工作流；先把默认路径做顺，再验证配置需求。 |
| 专业系统中的可信记录 | 金额、币种、阶段、版本、证据引用是结构化数据；历史批准版本不可被新生成内容静默覆盖。 |

先做四个核心工作面：**今天、机会、客户详情、审阅**。交付与账单先作为客户/机会路径中的明确步骤，确认使用频率后再扩成独立导航。

Odoo 与 ERPNext 先作为业务设计和未来适配对象。当前不把整个 ERP 引擎嵌入 OS，不引入大企业的组织、报表和权限复杂度。SAP 是设计参考，不是本次可直接复用的开源依赖。

## 4. “AI 员工”怎样实现

### 4.1 员工定义与一次工作分开

一个员工角色可以长期存在，但每次工作应是有边界、可终止的任务实例。同一个模型可以承担不同技能的角色，也可以在不改变业务对象的情况下被替换。

**员工角色 = 职责 + 方法/技能 + 输入范围 + 输出契约 + 可请求的工具 + 评价标准。**

**任务实例 = 具体目标 + 输入快照 + 预算/期限 + 运行记录 + 交付物 + 验收结论。**

公司长期记忆由结构化业务图、文档和经审阅的事实持有。模型会话、向量索引和框架 checkpoint 都是派生材料，不能成为不可替代的公司记忆。

### 4.2 首批岗位

| 岗位 | 输入 | 交付物 | 不能自行决定 |
| --- | --- | --- | --- |
| 需求分析员 | 经允许的客户简报/访谈记录 | 问题、约束、预算、待澄清点、证据引用 | 把推测写成客户确认事实。 |
| 报价助理 | 已确认需求、服务目录和报价规则 | 带版本的方案草稿、范围、金额、假设 | 向客户承诺价格或交付时间、发送报价。 |
| 交付规划员 | 已接受的范围和时间约束 | 里程碑、任务、依赖、验收草稿 | 静默改动合同范围或宣称任务已完成。 |
| 质量检查员 | 草稿、输入来源、业务规则 | 缺项、矛盾、计算差异和证据不足 | 给另一个 Agent 授权、替代 owner 批准。 |
| 经营助理（后续） | 可访问的结构化业务状态 | 今日优先事项、应收提醒、风险解释 | 把预测当收入、自动处分资金。 |

技能包采用版本化的输入/输出 schema、操作指南、模板、正反例和固定评测集。只有在稳定任务契约之后再做技能市场、热插拔组织图和自动招聘。

### 4.3 控制路径

~~~mermaid
flowchart TD
    F[创始人：目标与审阅] --> T[OS 任务与业务状态]
    T --> C[允许使用的输入快照]
    C --> A[受约束的员工运行器]
    A --> D[结构化建议与证据引用]
    D --> V[确定性校验与业务审阅]
    V --> F
    F --> P[适用的策略与 owner 授权]
    P --> E[OS 执行器]
    E --> R[结果、业务版本与证据]
    R --> T
~~~

具体规则：

- Agent 不拿数据库管理权限、owner 密钥或真实服务商凭据。
- 写操作先成为 ActionProposal；OS 决定是否允许、是否需要真实 owner 参与。
- 多 Agent 的一致意见只是一类建议证据。
- 工具结果和附件中的指令不得改写任务、权限或授权要求。
- “需要确认”“没有依据”“无法完成”是正常结果，不能用编造内容补齐 schema。
- 初期采用“分析 → 起草 → 检查”的有向流程，每个节点一次明确交付；不默认自由递归委派。
- Local Only 的真实实现必须满足 RFC 0004；本地 Python 进程、容器或推理服务器本身都不自动满足要求。

## 5. 组件调研与选型

**选型状态：研究结论与候选优先级；除仓库已有依赖外，不代表已经接入或跑过兼容性验证。**正式引入时锁定版本/commit、依赖图和许可证，再通过小实验决定是否晋级。

选型标准按顺序为：能否保持权限边界、能否替换和退出、能否本地运行且控制遥测、接口是否清楚、测试/维护能力、许可证和分发条件、实际接入成本。

### 5.1 Agent 框架

| 候选 | 已核对的能力/状态 | 本项目判断 |
| --- | --- | --- |
| **PydanticAI** | 类型化输出、工具和依赖；支持自定义 Model；核心 MIT。[文档](https://pydantic.dev/docs/ai/overview/)、[模型适配](https://pydantic.dev/docs/ai/models/overview/)、[许可证](https://github.com/pydantic/pydantic-ai/blob/main/LICENSE) | **首个合成员工实验的首选。**以 schema 和测试驱动短任务；Provider 与工具必须经 OS 适配。不要直接照示例把真实 API key 或网络权限交给 worker。 |
| **LangGraph** | 可单独使用的图执行、持久化与人工中断能力；核心 MIT。[文档](https://docs.langchain.com/oss/python/langgraph/overview)、[许可证](https://github.com/langchain-ai/langgraph/blob/main/LICENSE) | **复杂任务图的备选。**出现确切的分支/恢复需求后再验证；其 checkpoint 不能成为第二套业务真相或越过 OS 授权。 |
| **CrewAI** | 角色、任务、工具、委派等组织抽象；官方文档描述默认遥测及关闭方式。[Agents](https://docs.crewai.com/en/concepts/agents)、[遥测](https://docs.crewai.com/en/telemetry) | 借鉴岗位定义。暂不作为主运行器；先证明所有执行入口、记忆和遥测都能收口，再考虑试验。 |
| **Microsoft Agent Framework** | Python/.NET 多 Agent 与工作流方向。[官方仓库](https://github.com/microsoft/agent-framework) | 作为替代框架观察；若实际企业连接或调试需求明显优于首选，再做对照实验。 |
| **AutoGen** | 官方已进入维护模式，并推荐新用户评估 Agent Framework。[官方仓库](https://github.com/microsoft/autogen) | 不作为新架构的默认起点。历史设计仍可学习。 |
| **Google ADK** | 模型与部署可替换，生态对 Gemini 友好。[官方仓库](https://github.com/google/adk-python) | 备选；本项目不因 SDK 默认体验而绑定单一云模型。 |
| **Rig** | Rust Agent 组件，官方提示 API 仍可能有破坏性变化。[官方仓库](https://github.com/0xPlaygrounds/rig) | Rust 原生备选。与 PydanticAI 比较真实胶水成本后再选，不因同语言就跳过边界审查。 |

首个实验只选一个框架。没有必要同时安装 PydanticAI、LangGraph、CrewAI、ADK 来制造一个更大的组合层。

**PydanticAI 的退出标准：**如果必须绕过 OS 网关、必须开放任意网络/工具、引入第二份持久化事实，或类型适配代码明显压过任务实现，就停止试验。可以回到更薄的 Rust/JSON worker 或验证 Rig；业务 schema 和评测集应保持可复用。

### 5.2 整个 OS 可复用的组件

| 层 | 推荐次序 | 接入条件与来源 |
| --- | --- | --- |
| 持久工作流 | 先评估现有 sovereign-workflow；Temporal 暂后置 | 当前仅顺序 checkpoint。确有跨进程长期任务需求再研究 [Temporal](https://github.com/temporalio/temporal)；不可用框架重试掩盖不确定副作用。 |
| 本地推理 | llama.cpp 候选 | [llama.cpp](https://github.com/ggml-org/llama.cpp) 核心 MIT；运行器许可与模型权重许可分别审查。先固定合成输入，验证模型身份、IPC、资源限制和零出网。 |
| 文档解析 | Docling 候选 | [Docling](https://github.com/docling-project/docling) 代码 MIT，模型分别许可；本地处理也需沙箱、文件边界、资源限制和附件攻击测试。 |
| PDF 输出 | Typst 候选 | [Typst](https://github.com/typst/typst) 本地编译；[核心 Apache-2.0](https://github.com/typst/typst/blob/main/LICENSE)。固定模板、字体、包来源；禁止渲染时隐式联网下载。 |
| 工具互操作 | 官方 Rust MCP SDK / rmcp | [仓库](https://github.com/modelcontextprotocol/rust-sdk)；SDK 是协议适配，不是权限系统。当前[许可说明](https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE)处于 MIT→Apache-2.0 迁移，文档另有许可；必须按所锁版本核对，不能统称 MIT。 |
| 浏览器自动化/验收 | Playwright | [官方仓库](https://github.com/microsoft/playwright)。用于合成环境 E2E 或未来受控浏览器 worker；真实账号动作仍走审批和执行边界。 |
| UI | 先现有 JS；需要复杂表格再评估 TanStack Table/shadcn/ui | [TanStack Table](https://github.com/TanStack/table)、[shadcn/ui](https://github.com/shadcn-ui/ui)。先实现用户路径，增加构建链必须有具体收益。 |
| 桌面容器 | Tauri 后置 | [Tauri](https://github.com/tauri-apps/tauri)。本地 HTTP Demo 稳定后再判断安装、更新与 OS 凭据集成需求；桌面包装本身不证明隔离。 |
| 搜索 | 当前固定检索；真实数据先确定性查询，再按证据引入向量索引 | [Qdrant](https://github.com/qdrant/qdrant) 作为后续候选。向量、metadata、缓存也属于受保护存储清单；不能存成旁路明文。 |
| ERP 参考/适配 | Odoo / ERPNext | [Odoo LGPL-3.0 及附属代码说明](https://github.com/odoo/odoo/blob/19.0/LICENSE)、[ERPNext](https://github.com/frappe/erpnext)。先借鉴业务关系；复制代码或分发集成前另做许可与维护评估。 |
| 观测 | 本地、最少内容、可清除 | 不默认上传完整 prompt、文档、工具参数、客户信息到 tracing SaaS；观测不能成为第二条数据出境通道。 |

**暂不作为产品内核嵌入：**

- n8n 的 Sustainable Use License 对用途和分发有额外限制，应把它视为有条件的外部自动化选项，不能当作普通宽松许可依赖。[许可证](https://github.com/n8n-io/n8n/blob/master/LICENSE.md)
- Dify 使用带附加条件的 Apache 派生许可，涉及多租户与前端标识等条件；其整个平台也会引入独立状态和执行面。[许可证](https://github.com/langgenius/dify/blob/main/LICENSE)
- 分布式 Mesh、多租户 SaaS 平台、通用 Agent 市场、自研向量数据库与通用 ERP，均不进入首轮 Demo 关键路径。

以上是选型风险核对，不替代正式分发方案的法律审核。只有真正决定引入某个组件时才进行该组件的深入审核，避免为所有候选制造无谓工作。

### 5.3 每次引入组件必须交付的证据

每个组件试验卡只需一页可复核记录：

1. 锁定版本/commit、许可证与维护状态，说明是否有单独模型/字体/插件许可。
2. 它替代了哪段计划中的工作，为什么仓库现有工具不够。
3. 输入、输出、存储、网络、凭据、遥测与子进程的实际路径。
4. 一个真实运行的合成正例、一个拒绝/故障例；断网与服务不可用时的行为。
5. 替换或移除该组件时，哪些业务数据和接口保持稳定。
6. 晋级、继续试验、停止三选一；不以 star 数或营销文案作为验收。

## 6. 产品路线：先可见，再可用，再可靠地扩大自治

以下是开发顺序，不重定义 ROADMAP 的版本发布条件。合成 Demo 可以提前完成，真实产品的 v0.x 放行仍受原有门槛约束。

| 阶段 | 创始人可见的交付 | 退出条件 | 不能跨过的边界 |
| --- | --- | --- | --- |
| **S0 自动施工基础** | 能看到每卡计划、产物、测试、验收与失败升级 | 模型配置、状态校验与两次失败试演通过 | 不能声称 Goal 原生实现了本项目调度协议。 |
| **S1 固定咨询 Playground** | 中英文练习公司/服务/客户/需求、两次固定更改、检索与下一步 | 现有 v2 计划的产品、隔离、兼容性验收 | 无真实输入、模型、持久化、导入导出。 |
| **S2 完整合成业务演示** | 从需求到报价、审阅、交付计划、账单草稿和跟进 | 架构扩展方案先冻结；完整回放和业务不变量通过 | 不能偷偷把 S1 leaf 改成通用产品后端；模拟批准与回款须标注。 |
| **S3 合成 AI 员工实验** | 真实模型完成分析/起草/检查的限定任务 | 与确定性基线对比，输出、成本、故障与边界都有证据 | 独立实验面；真实模型不因此获得真实业务数据或凭据。 |
| **S4 真实本地业务 Alpha** | 创始人录入真实客户与工作记录，保存/恢复，审阅草稿 | owner、Vault 激活、恢复、存储清单与 RFC 0004 门槛全部成立 | 不以警告框、fixture token、标志位代替门槛。 |
| **S5 一个真实受控外部动作** | 一次明确批准的发送或集成 | 真实凭据 broker、确切效果绑定、审计、最小恢复、不确定状态处理 | 收件人和内容变更使授权失效；Indeterminate 不自动重发。 |
| **S6 可重复经营** | 每日计划、复盘、应收、专业包、可替换技能 | 重复用户路径、恢复演练与真实采用证据 | 新行业、新地域、新节点均按需求重新验证。 |

### 6.1 S1 的范围已定，不得被本蓝图扩大

执行[独立 Playground v2 计划](../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md)。旧的 minimal-graph-v1 文档已经 superseded，不能恢复其中更改既有 Workspace/UI 的路线。

固定业务例子：North Star Operations；Reporting clarity sprint；初始 $2,500，固定改为 $3,500；Acme / Alex Chen；每周报告耗时六小时；预算 $3,000–$5,000；财务批准约束；下一步为 30 分钟 scoping call。

只允许 CorrectOfferPrice、PromoteAcmeToCustomer、ShowReportingSearch、Reset 四个封闭动作。无价格参数、自由文本、客户 ID、检索词或自定义 patch。重启复原固定样例；这不叫恢复功能。

### 6.2 S2 的十分钟演示脚本

这是**提议中的扩展场景**，必须先由 S2-00 决定独立实验面或正式修订后的边界：

1. 首页看见 Acme 的需求与今天的下一步。
2. 查看问题、预算、采购约束和缺失信息。
3. 生成/选择报价草稿，显示来源、范围、金额和假设。
4. 创始人修改或退回；批准的是具体版本。
5. 从该版本形成交付里程碑与验收条件。
6. 创建账单草稿并显示“待开具/待收款”等准确状态。
7. 展示跟进建议和下一次行动。
8. 查看该链路的变化与依据；重置后可重复演示。

“十分钟”是设计目标，尚无实测。演示成功不等于真实顾问愿意付费。五名顾问的可用性观察是后续验证目标，需要实际参与者；没有观察就保留 Target，不由模型补写评价。

### 6.3 业务模型应先冻结的细节

| 对象 | 最小字段/关系方向（待对应设计卡冻结） | 关键不变量 |
| --- | --- | --- |
| Company / Offer | 公司目标、服务、币种、金额规则、服务范围 | 金额不采用浮点自由计算；币种明确。 |
| Relationship / Opportunity | 组织/联系人、机会阶段、下一步、关联记录 | Lead→Customer 不等于已签合同或已回款。 |
| Discovery | 问题、约束、证据、假设、待澄清项 | 模型推断与客户确认分别标记。 |
| ProposalVersion | 输入快照、范围、金额、有效性、前一版本 | 修改产生新版本；旧批准不可跟随最新正文。 |
| Engagement / Milestone | 所依报价、交付物、期限、验收标准 | 完成声明必须有可核对结果；范围变化显式处理。 |
| InvoiceDraft / PaymentRecord | 所依业务对象、金额、状态、来源 | 草稿、开票、应收、已收分别记录；不能从预期收入自动生成已收事实。 |
| Decision / ActionProposal | 被审阅版本、建议理由、权限请求、结果引用 | 业务同意、系统权限与实际执行结果分离。 |

真实税率、会计规则、合同条款和地域义务不由 Demo 默认推断。先支持明确的草稿与状态，专业知识包随后按版本化来源和专业审阅开发。

## 7. 真实数据和真实模型的放行依赖

[RFC 0004](../../rfcs/0004-data-sovereignty-boundaries.md)、[RFC 0005](../../rfcs/0005-dual-root-vault-and-recovery.md)和[ROADMAP 的依赖说明](../../ROADMAP.md)继续承担规范责任。

~~~text
1A engine
  → 1C0 owner authentication / one-use approval
  → 1B0 backup mechanics 与 1C1 key domains
  → 1D freeze legacy + equivalent PendingV2 candidate
  → 1B1 clean restore qualification of that exact candidate
  → ActiveV2 activation，关闭旧写入路径
~~~

真实数据产品门槛是这些条件的合取，不能挑一个 demo 测试代替其余条件。RFC 0006 的 synthetic owner/effect fixture 也不能自动升级为已实现的产品 1C0。

此外：

- 真实业务数据进入模型前，必须关闭整个持久化清单：业务库、附件、checkpoint、向量、缓存、队列、日志、崩溃转储、备份和导出。
- RFC 0004 的 trusted provenance、确定性变换、目的绑定、privacy compiler 与真实计算位置约束不能由调用方提供的标签代替。
- 无可信本地计算时应明确不可用，不静默切云。
- Provider secret 属于 Credential Broker，不进入模型、普通业务 Vault 或普通备份。
- 如外部效果可能已经发生却无法确认，记录 Indeterminate，由专门核对路径解决，不盲目重试。
- Owned Mesh 维持 Research，测得真实需要后再制定单独协议。

这些工作在原 backlog 中已有大量任务。先复用对应条目与诊断，不能为“新计划”再排第二套 Vault、授权事务、撤销或 freshness 实现。

## 8. 全局施工入口

**唯一施工入口：[docs/handoff/codex/README.md](../handoff/codex/README.md)。**

文档集按事实类型明确唯一维护位置：本文件维护产品愿景和选型依据；既有 RFC 维护安全协议；backlog 维护任务队列；任务卡维护精确施工范围与验收；Codex protocol/contracts 维护执行和记录规则。不要在多个文档中维护同一规则的副本。

从 [里程碑索引](../handoff/codex/milestones.md) 查看阶段方向，从 [角色与 Goal](../handoff/codex/models-and-goals.md) 查看 Luna medium、强模型逐卡验收和两次失败升级的运行方式。第一批实现从 S0-01/S0-02 开始；S0-00 的文档契约由本次文档集提供，运行器尚待实现与试演。

当前发现一个必须先处理的工程约束：Playground 的 Task 1 检查精确锁定两个源文件、完整 token 形状和零依赖。增加动作/模块/serde 会被正确拒绝。S1 的普通功能施工前，必须由强模型冻结并独立审阅边界检查的窄迁移；不能让 Luna 删除检查以取得绿色。

## 9. 交付目标

成功应当有三类证据：一张真实任务完成 worker→报告→独立 review→验收；一个创始人可操作的咨询 Demo；一个随产品范围增长、经实际验证的数据与执行边界。

本次交付的是完整的施工文档集。组件接入、角色配置、校验器、开发 Goal 和定时任务均不会因文档生成而自动激活。后续从冻结卡逐项实现，再按照记录证明运行机制已经可用。
