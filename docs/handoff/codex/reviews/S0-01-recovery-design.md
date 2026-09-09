# S0-01 历史保留恢复设计独立审核归档

日期：2026-09-09  
结论：**accepted — 仅接受恢复设计**

绑定提交：`24ecdd7529fbf6d06fcb6f5e08b785a07ca6282d`  
报告 blob：`4ef02498ef4564e8ebbdee5d578180de5f8eb0af`

独立 reviewer 确认：LegacyPendingEvent 将未知计时严格限定在 import 后首次
start 前，并覆盖 block/resume/revise 路径；RECOVERY-SPEC 按现行 v2、独立
architect attempt 和完整 B/C/W/R/M 执行。旧历史、未知事实和失败额度不被清零、
重分类或追认。设计报告的 `git diff --check` 与 scoped gate 均 exit 0；实际命令
为 `TEST_CHANGED_BASE=24ecdd7^ ./scripts/test_changed.sh`，完成行为
`test_changed: ALL GREEN — steps: gate-self-test file-size fmt — no cargo test scope in this change set`，日志为 `.harness/test_changed.log`。

本接受不代表 v3 已生效，不放行 legacy_import、S0-01 attempt 2 或 S0-02。后续
必须先冻结并完成 `S0-01-RECOVERY-SPEC`，再观察旧 writer/锁和真实分派能力。

复用 Git object/diff 与现有 scoped gate；新增工具：无。

