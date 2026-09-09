# Playground 固定检索与教学建议契约

**Revision 2 · 主线程独立审阅接受，接口Frozen；G04仍需独立卡及实际验收。**只覆盖S1-G04建议→S1-04；不宣称前置卡已完成。
依据 [standalone-v2 Task 2](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md)、
[现有domain/DTO契约](playground-contract.md)与[统一catalog](playground-catalog-contract.md)。
主线程分派/独立验收；真实业务与AI后续Goal不变，不要求自动S0 harness。

## 四状态内的产品裁决

仍只有CorrectOfferPrice、PromoteAcmeToCustomer、ShowReportingSearch、Reset四动作，
价格两值×Lead/Customer四状态；无沟通完成字段、第五动作、客户联系或任何外部效果。
guidance是只读教学建议，不能授权执行；直接观察session内部price/stage，不能接收
外部DTO、价格、query、ID或其他参数来驱动规则。

| 当前状态 | next_step_key | suggested_action | detail_key | completion_key |
| --- | --- | --- | --- | --- |
| 250000 + Lead | guidance_correct_price | CorrectOfferPrice | null | null |
| 250000 + Customer | guidance_correct_price | CorrectOfferPrice | null | null |
| 350000 + Lead | guidance_promote_customer | PromoteAcmeToCustomer | null | null |
| 350000 + Customer | guidance_review_scoping_call | null | thirty_minute_scoping_call | guidance_example_changes_complete |

这实现顺序“修价→转客户→查看已记录沟通建议→示例总结”。最后状态按next/detail/
completion顺序同时显示建议与总结；总结只表示本示例的两项修改完成，不表示已安排/
进行客户沟通或真实业务完成。不能再增加隐藏确认步骤把建议变为已执行事实。
Reset恢复首行；动作顺序任意但建议始终价格优先。ShowReportingSearch只影响未来
页面是否展示结果，不写session；检索结果始终是以下两条，无索引、排序、打分或query输入：
`(offer_label, reporting_clarity_sprint)`、`(discovery_label, weekly_reporting_takes_six_hours)`。
它们直接复用既有semantic key，顺序固定，不搜索真实数据，也不声称搜索完整企业资料。

## 唯一可序列化输出

新增于domain.rs、旧生产块后及原terminal tests前；不修改既有15字段read_model。
接口 `pub(crate) fn teaching_read_model(&self) -> PlaygroundTeachingReadModel`；仅外层
DTO及方法crate可见，所有字段/内部类型私有。派生Serialize，无Deserialize/反向转换。
以下片段是G04唯一新增domain生产grammar，空白/普通注释规则沿既有RustLexer：

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
struct ReportingHit { section_key: &'static str, fact_key: &'static str }
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
struct ReportingSearch { query_key: &'static str, hits: [ReportingHit; 2] }
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
struct Guidance {
    next_step_key: &'static str,
    suggested_action: Option<&'static str>,
    detail_key: Option<&'static str>,
    completion_key: Option<&'static str>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct PlaygroundTeachingReadModel {
    profile: &'static str,
    real_data_enabled: bool,
    persistence: &'static str,
    search: ReportingSearch,
    guidance: Guidance,
}
impl PlaygroundSession {
    pub(crate) fn teaching_read_model(&self) -> PlaygroundTeachingReadModel {
        let guidance = if self.graph.offer.price_usd_cents != 350_000 {
            Guidance { next_step_key: "guidance_correct_price",
                suggested_action: Some("CorrectOfferPrice"), detail_key: None, completion_key: None }
        } else if self.graph.relationship.stage == RelationshipStage::Lead {
            Guidance { next_step_key: "guidance_promote_customer",
                suggested_action: Some("PromoteAcmeToCustomer"), detail_key: None, completion_key: None }
        } else {
            Guidance { next_step_key: "guidance_review_scoping_call", suggested_action: None,
                detail_key: Some("thirty_minute_scoping_call"),
                completion_key: Some("guidance_example_changes_complete") }
        };
        PlaygroundTeachingReadModel {
            profile: "synthetic_playground", real_data_enabled: false, persistence: "none",
            search: ReportingSearch { query_key: "reporting_search_query", hits: [
                ReportingHit { section_key: "offer_label", fact_key: "reporting_clarity_sprint" },
                ReportingHit { section_key: "discovery_label", fact_key: "weekly_reporting_takes_six_hours" },
            ] },
            guidance,
        }
    }
}
```

JSON字段名/声明顺序与上块一致，Option::None显式为null，不skip/flatten/custom序列化。
外层固定profile/real_data_enabled/persistence标识保持响应成熟度说明。未来HTTP直接
用serde_json序列化；suggested_action只是两种现有action ID或null，不等于调用授权。
本卡不新增HTTP包装、路由、HTML、UI状态、JSON工具或产品IO。

## 统一catalog追加五键

原27键及全部文案保持；本轮在同一CATALOG末尾按下表顺序追加，长度27→32。
原27键规范继续唯一位于catalog契约，本节仅定义增量；不能另建JS/搜索/guidance词典。
所有占位符集合仍为空；每个输出*_key必须在统一数组恰好命中一次。

| key | en | zh |
| --- | --- | --- |
| reporting_search_query | reporting | 报告 |
| guidance_correct_price | Correct the offer price to match this example. | 修正服务价格，使其符合本示例。 |
| guidance_promote_customer | Mark the lead as a customer in this example. | 将本示例中的潜在客户标记为客户。 |
| guidance_review_scoping_call | Consider the recorded scoping call as a suggested next step. | 将已记录的需求范围沟通作为建议的下一步。 |
| guidance_example_changes_complete | The two changes in this example are complete. No client call has been scheduled or made. | 本示例的两项修改已完成。尚未安排或进行任何客户沟通。 |

## 先行S1-G04窄门建议（供主线程冻结；未创建卡）

G04必须在S1-03实际接受、先前gate writer停止后独立执行并接受；不能与当前G03
并发争写，也不能让S1-04放宽自己的验收。G04未来精确写集仅：

- `crates/consultant-playground/tests/support/boundary.rs`
- `crates/consultant-playground/tests/physical_boundary_teaching.rs`（新增）
- `crates/consultant-playground/tests/support/fixtures/teaching-domain-additions.rs.txt`（新增）
- `crates/consultant-playground/tests/support/fixtures/catalog-guidance-production.rs.txt`（新增）

新测试独立为integration test文件，复用support/boundary与RustLexer；不再向已超过
1000行的physical_boundary_source.rs追加测试，不复制parser或source闭包扫描器。
不改生产/manifest/lock/lib。新domain形状=既有已固定DTO生产fixture+本契约代码块，
保留全部旧形状；新catalog fixture=原固定27行+本节5行、数组长度32，保留原27形状。
fixture由已接受文档人工固定，绝不从候选源码生成。新增const类型均为&str并用
include_str读取；沿source_boundary现签名和RustLexer，不新增parser/通用框架。
catalog仅en/zh普通字符串位置沿G03规则可变（新形状64处），32键、顺序、结构、
derive及所有其他tokens固定；不能因新key开放任意key/代码。源闭包仍同3文件，
domain/catalog wrapper与path/symlink/root检查不变。新旧两种catalog可接受不表示
32-key实现完成；S1-04内容及跨输出key测试强制新完整交付，避免漏加key。

G04新增命名测试：`teaching_gate_accepts_old_and_complete_next_shapes`（旧生产绿色，
新两fixture通过）；`teaching_gate_rejects_guidance_and_search_mutations`（参数/&mut/
外部DTO输入、写graph、错优先级、假call-done/第五动作、改hit/数量、自由query、
Deserialize/反向转换、IO/env/process/unsafe/宏/任意函数均拒绝）；
`teaching_catalog_gate_rejects_unknown_keys_and_structure`（32键缺/改/换序/额外键、
长度/表达式/可执行内容拒绝；只en/zh数据变化通过）。复用具名DomainProductionShape/
CatalogProductionShape及既有wrapper/path错误，全部现有恶意回归继续运行。

G04及S1-04必需命令相同：

```bash
cargo test -p sovereign-consultant-playground --locked
cargo clippy -p sovereign-consultant-playground --all-targets --locked -- -D warnings
git diff --check
./scripts/test_changed.sh
```

复用内部graph/semantic key/serde/read_model、统一catalog、RustLexer/boundary/
source closure与test_changed；无检索框架/JSON工具/新依赖，IO/模型/产品授权边界不变。
复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。
