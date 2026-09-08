# 产品模型凭据：本地配置与交接

**2026-09-09。**本文件是产品模型凭据录入步骤、本地条目标识和连接状态含义的唯一说明。架构与权限要求引用[产品蓝图 §7.1](../../product/founder-os-execution-blueprint.zh-CN.md#provider-boundary)和适用 RFC；开发用 Codex 的模型配置仍见 [models-and-goals](models-and-goals.md)。返回[施工总入口](README.md)。

## 当前状态

首个产品提供者选择 Anthropic / Claude。创始人表示已有 API key，但本仓库**尚无可用的 Anthropic 设置页、凭据导入命令或真实模型连接**。本文没有读取、保存或验证任何真实 key，也不激活 API 调用。S3-M 完成前，可以由创始人先在本机保管凭据；这与应用已经配置完成是两回事。

不要将 key 发到聊天、任务卡、Git 文件、截图、issue 或 worker 报告。配置时只需要把明文交给本机的受信任保管界面；代码审阅者和模型无需看见它。

## 现在可以做：存入 macOS 钥匙串

1. 用 Spotlight 搜索并打开 **钥匙串访问 / Keychain Access**。选择本机的 **登录 / login** 钥匙串；若未显示列表，可按 `⌘1` 打开钥匙串查看器。
2. 选择“文件 → 新建密码项目”，或按 `⌘N`。这是本地通用密码条目，不要求创建网站登录账号。
3. 填写下表，在密码框中由创始人自行粘贴 key，然后保存。输入时不用向本会话共享画面或显示密码。

| 字段 | 本项目约定 |
| --- | --- |
| 钥匙串 | `login`，本机保管 |
| 项目名称 / Keychain Item Name | `sovereign-founder-os.anthropic.mvp` |
| 账户名称 / Account Name | `founder` |
| 密码 / Password | 创始人自己的 Anthropic API key；不写入本文或聊天 |

保留系统的访问控制，不设置“允许所有应用访问”。未来 broker 的访问配置必须先用测试条目验证，不为了省去授权提示而给通用 shell、脚本解释器或任意 worker 永久读取权限。若界面与这里不同，先保留 key 在原有保管工具，不改用带明文的终端命令。

这些名称是**非秘密的本地录入约定**，不是已实现 API 或已冻结的 broker 接口。S3-M 设计/实现要用虚假 secret 验证 UI 创建的条目如何映射到系统查询属性，并通过条目冲突、缺失和访问拒绝测试，再接入真实条目。存在同名条目时不得由脚本静默覆盖或任选一个。

保存后只需告知“已存入钥匙串”；不要发送 key、截图或复制“显示密码”的内容。本轮不读取钥匙串，也不检查你的其他凭据。

Apple 官方说明支持[用 `⌘N` 创建密码项目](https://support.apple.com/en-ca/guide/keychain-access/kyca699a9058/mac)，并说明了[钥匙串查看器及访问控制](https://support.apple.com/en-gb/guide/keychain-access/kyca1085/mac)。系统保管能减少明文散落，但不能保证已失陷的主机绝对安全。

## 提供者账户与 key 的选择

优先使用只服务本 MVP 的凭据和适用的工作区范围，保留轮换/撤销路径；不要使用 Admin key 代替普通模型调用凭据。个人开发和共享/自动运行工作负载的凭据身份需按提供者当前规则选择；现有 key 的类型、到期与工作区范围在接入时核对，不凭前缀或模型自述认定。

Anthropic 当前文档要求 API key 保存在 secret manager；多工作区身份凭据还可能需要明确的 workspace 选择，具体接入按[官方认证说明](https://platform.claude.com/docs/en/manage-claude/authentication)核对。账户、工作区和 credential-handle 归受保护配置，不抄入公开仓库或审计事件。具体模型 ID 及调用预算由受审配置记录；有 key 不等于有可用额度或已经批准所有费用。

后续产品设置入口应只展示“未配置、已保存、需要解锁、连接验证通过、连接失败/需要重新配置”等状态，不提供默认显示明文 key 的按钮。连接验证需要显式、受限的合成请求；条目存在、访问获准、服务商接受请求、完整应用调用链验收通过要分别记录。错误必须脱敏，验证时间与模型配置变化后重新检查。

## 为什么不放进 GitHub repository secret 再取回

GitHub 的 [Get a repository secret](https://docs.github.com/en/rest/actions/secrets#get-a-repository-secret) 返回名称、创建/更新时间等元数据，不返回解密后的值。Actions 可按工作流权限注入 secret，但不是把它打印、上传或传回本地的通道。

本机 MVP 使用本地保管入口；将来确实需要 CI API 调用时，再设计独立、范围受限的 CI 身份与注入方式。不得新增“输出 secret 以确认保存成功”的调试步骤，也不把 key 放进 `export` 命令、shell 启动文件、进程参数或普通 `.env` 作为本文的默认路径。

## 接通之后如何使用

按 [S3 模型连接前置卡](milestones.md#model-connection) 和产品蓝图 §7.1 交付真实协议、broker 与验收后，员工只请求受约束的模型调用；broker 在获准边界内使用凭据。明文不进入模型上下文、前端、普通 worker 的环境或报告。

凭据保管和 API 连通都不赋予真实客户数据出网权限。初次 MVP 仅使用经过批准的合成业务内容及公开资料；法律 RAG 的查询、检索片段与云模型输入仍按各自的数据边界审查。真实数据、签约、发信或资金操作各自满足对应阶段与授权要求。
