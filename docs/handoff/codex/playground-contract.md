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

## S1-G02→S1-02：可序列化单向 DTO（已独立接受）

本节已由主线程独立审阅并冻结；旧动作行为/fixture不变。本轮直接实现JSON输出所需
DTO，不做临时非序列化版本。G02合并本阶段必要source与manifest窄门，旧代码须绿色。
生产grammar是原fixture+动作，仅精确替换 `struct PlaygroundSession`、
`enum PlaygroundAction` 为 pub(crate)，new/apply 为 pub(crate) fn；再追加下块。
DTO及read_model为pub(crate)，供未来同crate HTTP消费；DTO字段私有，graph及其字段
保持私有；lib.rs不变、不re-export。读取返回独立Copy快照，无graph引用或反向构造。
15字段声明顺序及JSON key固定，无serde rename/flatten/default/skip/custom serializer。

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct PlaygroundReadModel {
    profile: &'static str,
    real_data_enabled: bool,
    persistence: &'static str,
    company_name: &'static str,
    offer_name_key: &'static str,
    offer_price_usd_cents: u32,
    relationship_organization: &'static str,
    relationship_contact_name: &'static str,
    relationship_contact_email: &'static str,
    relationship_stage: &'static str,
    discovery_problem_key: &'static str,
    discovery_budget_min_usd_cents: u32,
    discovery_budget_max_usd_cents: u32,
    discovery_constraint_key: &'static str,
    discovery_next_step_key: &'static str,
}
impl SemanticKey {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReportingClaritySprint => "reporting_clarity_sprint",
            Self::WeeklyReportingTakesSixHours => "weekly_reporting_takes_six_hours",
            Self::FinanceMustApprove => "finance_must_approve",
            Self::ThirtyMinuteScopingCall => "thirty_minute_scoping_call",
        }
    }
}
impl PlaygroundSession {
    pub(crate) fn read_model(&self) -> PlaygroundReadModel {
        PlaygroundReadModel {
            profile: "synthetic_playground",
            real_data_enabled: false,
            persistence: "none",
            company_name: self.graph.company.name,
            offer_name_key: self.graph.offer.name_key.as_str(),
            offer_price_usd_cents: self.graph.offer.price_usd_cents,
            relationship_organization: self.graph.relationship.organization,
            relationship_contact_name: self.graph.relationship.contact_name,
            relationship_contact_email: self.graph.relationship.contact_email,
            relationship_stage: match self.graph.relationship.stage {
                RelationshipStage::Lead => "lead",
                RelationshipStage::Customer => "customer",
            },
            discovery_problem_key: self.graph.discovery.problem_key.as_str(),
            discovery_budget_min_usd_cents: self.graph.discovery.budget_min_usd_cents,
            discovery_budget_max_usd_cents: self.graph.discovery.budget_max_usd_cents,
            discovery_constraint_key: self.graph.discovery.constraint_key.as_str(),
            discovery_next_step_key: self.graph.discovery.next_step_key.as_str(),
        }
    }
}
```

只有DTO实现Serialize；graph/session/action及领域类型均不实现Serialize/Deserialize，
无DTO→graph/session/Workspace转换。key是固定catalog标识，后续翻译不改graph。
不写JSON编码器；调用既有serde_json::to_value/to_string即可，错误按Result返回调用方。
manifest只允许原零依赖集合或**完整二项**普通依赖：`serde = { workspace = true }`、
`serde_json = { workspace = true }`。复用根 `serde version="1", features=["derive"]`
及 `serde_json="1"`；Cargo.lock当前版本分别1.0.228、1.0.150，仅添加leaf依赖边，
不升级包/校验和。Cargo metadata逐项固定name、req=`^1`、kind/target/rename/registry
均null、source=`registry+https://github.com/rust-lang/crates.io-index`、optional=false、
uses_default_features=true、features分别恰为[derive]/[]；path必须缺失或null。
拒绝单项/第三项/改kind/source/version/features/target/optional/rename/defaults；
package features仍空，publish=false、无build script、单一lib及source-root规则不变。
本节仅放开两个精确依赖与明确crate可见性，其他永久边界保持；HTTP/UI/server、
catalog、检索/guidance、CLI未冻结。真实业务与AI后续Goal不变，不宣称S1已完成。
