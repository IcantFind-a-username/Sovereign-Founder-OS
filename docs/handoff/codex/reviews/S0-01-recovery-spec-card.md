# S0-01-RECOVERY-SPEC 卡级审核归档

日期：2026-09-09  
结论：**accepted — 仅接受设计卡准备范围**

## 绑定

- 基线提交：`24ecdd7529fbf6d06fcb6f5e08b785a07ca6282d`
- 卡 blob：`e656675a40afbcce2cc5a43cfec2b215ce99def5`
- 卡：`docs/handoff/codex/cards/S0-01-RECOVERY-SPEC.md`
- 卡候选仍为新增文件；本归档不代表 Wspec、v3 实现或 S0-01 完成。

## Reviewer 结论

静态核对未发现范围或协议偏差。taskId、revision、parent、architect/reviewer
角色、十文件写集、唯一报告、依赖和冻结输入均符合已接受的恢复设计。Bspec、
Cspec、Wspec、Rspec、Mspec 顺序、七个集成记录路径、v2 自身验收与后续 v3
生效边界一致。pending/null、双 reservation、错误与失败预算、旧历史保留、
evidence-only W 和后续卡修订要求完整；没有宣告 S0-01 完成或用 v3 为自身授权。

## 实际证据

- `git diff --check`：exit 0。
- 新卡 `git diff --no-index --check /dev/null docs/handoff/codex/cards/S0-01-RECOVERY-SPEC.md`：exit 1，无空白诊断；这是新增文件差异的预期返回。
- `TEST_CHANGED_BASE=24ecdd7529fbf6d06fcb6f5e08b785a07ca6282d GATE_SELFTEST_RUNNING=0 ./scripts/test_changed.sh`：exit 0，完成行包含 `gate-self-test file-size fmt`。
- 详细日志：`.harness/test_changed.log`。

## 尚未验证

独立设计 review 尚未作为文件入库；Bspec/Cspec、真实 actors、锁与停止观察、
未来 Wspec/Rspec、正式 CheckRun/Review 和 architect 执行报告均未发生。卡接受
只允许 controller 完成前置记录并冻结后交 architect，不允许直接导入旧事件、
启动 S0-01 attempt 2 或进入 S0-02。

## 复用与新增工具

复用 Git object/diff、现有 `scripts/test_changed.sh` 及文档规范；新增工具：无。

