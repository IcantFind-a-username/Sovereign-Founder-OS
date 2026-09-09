# S0-01 attempt 1 · model configuration smoke

日期：2026-09-09
基线：`ebceebf`
执行角色：controller（主线程）
状态：`Candidate`，等待配置加载与角色生命周期验收

## 已观察

- `.codex/config.toml` 与四份角色 TOML 均由 Python `tomllib` 成功解析。
- Luna worker 只读 smoke thread：`01a0833d-1fe6-7813-8b15-103204ebe086`，显式 `gpt-5.6-luna` / `medium`，已完成并返回报告。
- Astra reviewer 只读 smoke thread：`01a0833d-a813-7810-b2cb-e2fcd4f73c87`，显式 `gpt-6-astra` / `high`，已完成并返回报告。
- 两次 smoke 均未修改文件、读取凭据或调用 Anthropic。Astra smoke 确认 reviewer TOML 声明 `sandbox_mode = "read-only"`，但其实际会话权限仍为 workspace-write，因此不能把本次行为当作沙箱强制证明。
- `gpt-6-astra` bootstrap controller 子任务曾被平台置于 `waitingOnApproval`，尚未取得可用于 S0 控制的完整嵌套分派证据。若平台不放行嵌套，按 models-and-goals §8.2 由主线程机械转发完整简报。

## 未验证 / 下一步

尚未证明客户端会自动加载项目 `.codex` 文件、角色名覆盖优先级、真正的 reviewer 只读隔离、嵌套分派和槽位释放。不能仅凭 TOML 存在或模型文字自报完成 S0-01。

下一步是由合法 bootstrap controller 按卡要求记录一次实际 worker→reviewer 生命周期；若嵌套分派继续需要审批，则保留审批阻塞并使用主线程转发，记录实际 caller 与 decision author。完成这些证据后再交独立 reviewer，并决定 S0-01 是 accepted 还是 blocked。

## 恢复摘要

Goal 见 `docs/handoff/codex/models-and-goals.md` §13.1；当前卡 S0-01 revision 2，attempt 1，失败次数 0，spentMs 未计（仅配置/只读 smoke）。当前候选记录为本报告，未产生产品 candidate。下一动作：核对平台审批状态和自定义角色真实加载，或按 §8.2 转发强模型简报。不要读取凭据，不调用 Anthropic，不跳到 S0-02。
