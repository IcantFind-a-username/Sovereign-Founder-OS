# Playground 双语 catalog 契约

**Revision 1 · 主线程独立审阅接受；S1-03接口Frozen，G03已形成独立卡，仍需实际验收。**仅S1-G03建议与S1-03增量；不改既有动作/DTO契约。
依 [standalone-v2](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md)
和 [Playground契约](playground-contract.md)。S1-02目前Frozen但未获本设计确认实现；
S1-03必须等待S1-02、先行G03实际接受。真实业务/AI后续Goal不变。

## 唯一数据接口

新增 `crates/consultant-playground/src/catalog.rs`，仅静态数据，不接收输入：

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct CatalogEntry {
    key: &'static str,
    en: &'static str,
    zh: &'static str,
}
pub(crate) const CATALOG: [CatalogEntry; 27] = [ /* 下表顺序的27项 */ ];
```

每项唯一形状为 `CatalogEntry { key: "固定key", en: "英文", zh: "中文" },`。
字段私有；CATALOG与类型crate可见，无constructor/lookup/Deserialize/Locale状态。
未来同crate HTTP可直接 `serde_json::to_string(&catalog::CATALOG)`，结果为27对象数组，
每对象JSON键恰为key/en/zh；本轮不增加route/server或JSON包装层。
`lib.rs`在现有domain声明后仅新增 `#[cfg_attr(not(test), allow(dead_code))] mod catalog;`。
manifest/lock沿S1-02两项依赖不变；不得再建JS词典。未来页面直接消费此数组，按
key及本地en/zh选择文字，用textContent显示；语言选择不写graph/session或业务数据。

## 完整27键与文案

顺序固定；四个语义key和lead/customer与现DTO值完全一致。所有文案无占位符，
两语言占位符集合均为空；名称、邮箱、金额只从DTO取，不复制到catalog业务字段。

| key | en | zh |
| --- | --- | --- |
| page_title | Consultant Playground | 顾问练习场 |
| boundary_notice | Practice with this example only. You cannot enter or save your own business or customer data here. Real-data setup is unavailable in this preview. | 请仅使用此示例练习。你无法在此输入或保存自己的业务或客户数据。此预览版尚不支持真实数据设置。 |
| company_label | Company | 公司 |
| offer_label | Offer | 服务方案 |
| relationship_label | Relationship | 客户关系 |
| discovery_label | Discovery | 需求分析 |
| price_label | Price | 价格 |
| organization_label | Organization | 客户公司 |
| contact_name_label | Contact | 联系人 |
| contact_email_label | Email | 邮箱 |
| stage_label | Stage | 阶段 |
| problem_label | Problem | 问题 |
| budget_label | Budget | 预算 |
| constraint_label | Constraint | 限制条件 |
| next_step_label | Next step | 下一步 |
| reporting_clarity_sprint | Reporting clarity sprint | 报告梳理短期项目 |
| weekly_reporting_takes_six_hours | Weekly reporting takes six hours | 每周编制报告需要六小时 |
| finance_must_approve | Finance must approve | 需要财务批准 |
| thirty_minute_scoping_call | 30-minute scoping call | 30分钟需求范围沟通 |
| lead | Lead | 潜在客户 |
| customer | Customer | 客户 |
| action_correct_price | Correct offer price | 修正服务价格 |
| action_promote_customer | Mark as customer | 标记为客户 |
| action_show_reporting_search | Show reporting search | 查看报告相关检索 |
| action_reset | Reset example | 重置示例 |
| language_en | English | 英语 |
| language_zh | Simplified Chinese | 简体中文 |

本表是实际内容验收源；生产仅CATALOG一份运行数据，测试的冻结预期不作为运行词典。
后续错误、反馈、guidance等新key等待相关行为冻结，不能为尚未确定的route错误预铺文案。

## 先行S1-G03窄门契约

现有精确两文件闭包不允许catalog.rs；S1-03不能同时修改自己的验收门。需先
按 [S1-G03卡](cards/S1-G03.md) 执行并独立审阅接受后才执行S1-03。
G03依赖S1-02接受，不得与尚未停止的先前gate writer并行修改。
未来G03精确写集仅：

- `crates/consultant-playground/tests/support/boundary.rs`
- `crates/consultant-playground/tests/physical_boundary_source.rs`
- `crates/consultant-playground/tests/support/fixtures/catalog-production.rs.txt`（新增）

G03依赖S1-02接受及所有先前gate writer已停止。旧生产不改；现有所有domain语法、
metadata/source-root/symlink/path/manifest拒绝保留。仅新增上述完整catalog生产fixture，
原lib形状与新增catalog声明形状二选一；实际源闭包只接受旧2文件或新3文件，新3文件
必须与新lib声明配对，不能忽略未声明/未知源文件。fixture人工来自本契约，不读候选刷新。
保留 `source_boundary(path: &Path, source: &str) -> Result<(), SourceBoundaryError>`，
新增 `validate_catalog_shape(tokens: &[RustToken]) -> Result<(), SourceBoundaryError>`，
复用RustLexer/精确terminal test wrapper/path检查；错误新增CatalogProductionShape、
CatalogTestModuleShape。共享wrapper helper可加label/kind参数，不复制扫描器。

**数据位置可变，代码结构不可变**：对catalog生产tokens先剥唯一终结cfg(test)模块，
与fixture完整比较；仅每行en/zh值的54个位置允许不同的普通双引号Rust字符串literal。
位置由固定fixture标定，不由候选内容寻找；key的27个字面量、顺序、长度27、derive、
类型/字段/可见性、初始化结构和其余每个token均精确固定。普通字符串须是RustLexer
产生且首尾为双引号的Literal；raw/byte/C字符串、宏、表达式、拼接和任意尾随token
均拒绝。中英文文本内容由S1-03测试按上表精确核对；换文案无需改变安全grammar。
这不是放宽副作用边界：恶意文字仍只是数据，生产没有eval/模板/HTML执行路径。

G03新增命名测试：`catalog_gate_accepts_static_text_variants_only`（旧代码、新fixture、
仅en/zh变化通过）；`catalog_gate_rejects_structure_key_and_code_mutations`（改key/
长度/类型/字段/pub/derive、Deserialize、自定义serializer、加入函数/IO/env/process/
unsafe/include/path、literal换表达式、extra token拒绝）；
`catalog_gate_preserves_wrapper_and_source_closure`（改/漏wrapper、漏catalog声明、
未知源文件、symlink/root逃逸拒绝）。全部原恶意变异回归仍执行且核具名错误。

G03及S1-03必需命令均为：

```bash
cargo test -p sovereign-consultant-playground --locked
cargo clippy -p sovereign-consultant-playground --all-targets --locked -- -D warnings
git diff --check
./scripts/test_changed.sh
```

无新依赖、IO、文件/环境读取、模型、权限或真实业务输入；产品graph/read_model不改。
复用RustLexer/boundary、production_sources/source_root、ManifestFixture/JsonParser及
serde/test_changed；新工具无，不引入通用schema/翻译框架或第二份字典。
复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。
