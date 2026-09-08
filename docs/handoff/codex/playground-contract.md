# Playground 动作增量契约

**Revision 1 · S1-00 首个增量已由主控独立审阅接受；Frozen。**仅覆盖 S1-G01→S1-01。
依据 [standalone-v2](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md)。
主线程拆卡、验收，Luna 执行小任务；自动 S0 harness 不作为产品开发前置。

## 接口与状态

沿用 `crates/consultant-playground/src/domain.rs` 的全部既有私有类型、字段、
derive、`PlaygroundSession::new() -> Self` 和编译期 fixture；不改变可见性，
`lib.rs` 不变。新增私有 `PlaygroundAction` 和
`PlaygroundSession::apply(&mut self, action: PlaygroundAction) -> ()`，无其他参数。
所有动作在全部四个可达状态均合法，不返回错误。graph 不公开、不序列化。

| 动作 | 唯一变化 |
| --- | --- |
| CorrectOfferPrice | `price_usd_cents` 设为 `350_000`；初值 `250_000`，重复幂等。 |
| PromoteAcmeToCustomer | `relationship.stage` 设为 Customer；初值 Lead，重复幂等。 |
| ShowReportingSearch | graph/session 完全不变，返回 `()`；检索结果投影留给后续冻结。 |
| Reset | 整个 session 等于 `PlaygroundSession::new()`，重复幂等。 |

四状态是价格 `{250_000,350_000}` × 阶段 `{Lead,Customer}`，两修改可交换；
其余字段在非 Reset 动作前后逐项相等。Reset 恢复全部原值，不读取存储。
没有字符串、价格、ID、query、时间、patch 或 JSON 输入。本增量只有纯动作层，
ShowReportingSearch 不宣称已有检索结果或用户界面。

## G01 批准的唯一动作 grammar

生产 token 必须为原 `EXPECTED_DOMAIN_PRODUCTION`，或原 token **按序追加**
以下片段；随后仍是恰好一个终结 `#[cfg(test)] mod tests { ... }`。空白/普通注释
仍按既有 RustLexer 规则忽略。原 fixture grammar 保留，不从当前源码重新采样。

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaygroundAction {
    CorrectOfferPrice,
    PromoteAcmeToCustomer,
    ShowReportingSearch,
    Reset,
}

impl PlaygroundSession {
    fn apply(&mut self, action: PlaygroundAction) {
        match action {
            PlaygroundAction::CorrectOfferPrice => {
                self.graph.offer.price_usd_cents = 350_000;
            }
            PlaygroundAction::PromoteAcmeToCustomer => {
                self.graph.relationship.stage = RelationshipStage::Customer;
            }
            PlaygroundAction::ShowReportingSearch => {}
            PlaygroundAction::Reset => {
                *self = Self::new();
            }
        }
    }
}
```

此片段是文档契约，不是当前产品实现；G01 先固定测试 fixture，S1-01 后实现
同一 grammar。S1-01 命名功能测试强制新增接口存在，不能以旧 grammar 通过边界
检查冒充交付。后续改接口须先独立审阅新的窄门迁移。

## 永久边界与待冻结部分

仅 Task 1 的精确生产 token 集扩展为两个受审形状；两个源文件闭包、源根/
symlink/path 属性拒绝、精确测试 wrapper、零依赖/feature、publish=false、
无 IO/env/process/clock/randomness/network/unsafe/product surface 均保留。
禁止从候选代码刷新 expected、只检查名字/关键字、开放任意 match/body 或自由参数。
不调用 WorkflowRunner、AuthorityStore、AuditLedger、ModelProvider，不改旧 UI、
Workspace、导出、校验、CLI 或产品安全边界。
DTO/可见性扩展、检索结果、guidance、双语 catalog、HTTP、assets、server、CLI
接线和真实进程隔离仍为 Blocked design，需要时再冻结，未授予写权限。
