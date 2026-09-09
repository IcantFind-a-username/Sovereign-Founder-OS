# Luna 启动就绪验收 · 2026-09-09

**结论：accepted，文档已可用于开工；S0 运行尚未完成。**用户可切换为 gpt-5.6-luna / medium，发送[唯一启动消息](../models-and-goals.md#luna-kickoff)，从 S0-01 组织强模型自举。本文是一次验收记录，不另行定义目标或流程。

- 审阅基线：`3fb0904`。
- 已接受的完整文档候选：`e45be34e3867877402f533728ca520a711129a97`。
- 初次候选：`4bd8fc9`；独立复核发现 scoped gate 可继承缩小范围/跳过自检的环境变量，后续候选已补齐约束与 planned test。
- 本记录与 backlog 勾选是验收后的记录追加；不改变被审阅规范。

## 已闭合的开工条件

| 条件 | 规范与结果 |
| --- | --- |
| Luna 可从第一步启动 | [models-and-goals §8.2](../models-and-goals.md#bootstrap-controller) 明确强模型自举、必要时主线程转发、S0-06 后交接。 |
| 小卡接口可复用 | [contracts v2](../contracts.md) 冻结共享字段函数、完整契约历史、状态恢复与单次 fallback 上限；对应 S0 卡均已同步。 |
| 候选与报告可实际提交 | [protocol §9.6.1](../protocol.md) 区分认领基线、controller 记录、worker candidate、报告及集成提交。 |
| 检查不能仅凭口头绿色 | 明确普通命令、Node TAP、Cargo 测试和 scoped gate 的完成判据；scoped gate 固定基线、自检与日志环境设置。 |
| 后续设计有明确交接 | S1-00 的输出写集包含单张卡；后续阶段通过强模型设计、独立审阅、登记再施工。 |
| 全流程有最终终点 | [MVP 集成阶段](../milestones.md#mvp-integration) 将统一案例、五个员工、返工/失败、法律 RAG 和真实浏览器交付纳入最终验收。 |

## 实际验证与限制

- 独立只读复核绑定上述完整候选，结论 accepted；未发现剩余开工或错误验收断点。复核读取实际 diff、完整规范与已有 gate 源码；无写入。
- 已检查 canonical 文档集的本地链接、锚点、代码围栏、S0 卡 revision/接口引用，以及队列 ID 唯一性；通过。检查为本次 Node 一次性静态核对，不冒充已实现的 S0 校验器。
- `git diff --check 3fb0904..e45be34`：通过。
- `./scripts/test_changed.sh`：ALL GREEN，实际步骤为 gate-self-test、file-size、fmt。本次仅文档改动，无 Cargo test scope；没有声称重新运行完整产品测试。
- 变更范围可由 `git diff --stat 3fb0904..e45be34` 重现；均为 docs 下的协议、卡、索引和队列，无产品源码、依赖或已安装模型配置改动。
- 工具复用：既有 Git、文档读取与仓库 gate；独立复核写入 diff 为 0。新增运行时工具：无。
- `.codex/config.toml`、`scripts/codex-control/state.mjs` 和 CLI 尚不存在；S0 仍须实际实施和验收。文档检查不证明模型分派、升级或恢复已可运行。
- 没有读取凭据、调用 Anthropic、启用 Goal/定时任务、push、PR 或部署。此前“已添加”仍保持[用户确认保管](../provider-setup.md)的含义。

## 交接

唯一入口：[README](../README.md)。第一张卡：[S0-01](../cards/S0-01.md)。启动时按文档核对实际工作树与现有 claim，强模型承担 S0 控制；S0-06 通过后 Luna 接管后续卡。完整目标只在 [models-and-goals §13.1](../models-and-goals.md) 维护，当前队列状态只在 [backlog](../../../backlog.md) 维护。
