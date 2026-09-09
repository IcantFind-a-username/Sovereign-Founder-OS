# Playground 六资产与浏览器UI契约

**Revision 1 · 主线程独立审阅接受，接口Frozen；G06已形成独立架构卡，仍需实际验收。**范围S1-G06建议→S1-06；代码/浏览器/server未由本设计实现。
依据 [standalone-v2](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md)、
[HTTP契约](playground-http-contract.md)、[32键增量](playground-guidance-contract.md)及
[原27键](playground-catalog-contract.md)。前置必须实际验收，不以Frozen代替完成。

## 资产与传输接口

运行资产恰在 `crates/consultant-playground/assets/`：index.html、styles.css、i18n.js、
app.js、consultant-ui.js、favicon.svg，六个普通文件，无子目录/symlink/其他运行文件。
新 `src/assets.rs` 只导入 `crate::http::AssetRoute`，定义：

```rust
pub(crate) struct EmbeddedAsset {
    pub(crate) content_type: &'static str,
    pub(crate) bytes: &'static [u8],
}
pub(crate) fn asset(route: AssetRoute) -> EmbeddedAsset;
```

asset是穷尽match，无自由path/URL输入：

| AssetRoute | 唯一include_bytes!字面量 | content_type |
| --- | --- | --- |
| Index | ../assets/index.html | text/html; charset=utf-8 |
| Styles | ../assets/styles.css | text/css; charset=utf-8 |
| I18n | ../assets/i18n.js | text/javascript; charset=utf-8 |
| App | ../assets/app.js | text/javascript; charset=utf-8 |
| ConsultantUi | ../assets/consultant-ui.js | text/javascript; charset=utf-8 |
| Favicon | ../assets/favicon.svg | image/svg+xml |

每arm仅构造上述两字段，bytes为对应编译期include_bytes!。lib.rs仅新增与既有
模块同式 `#[cfg_attr(not(test), allow(dead_code))] mod assets;`，顺序由G06固定并实际验证rustfmt稳定，必要时沿既有分组注释保持顺序。
未来transport处理HandlerOutcome::Asset时调用asset，并按HTTP契约加no-store/nosniff，
不加CORS/cookie/redirect。此卡不bind、开浏览器、读磁盘或添加tiny_http。
HTTP八route、四action、11错误、StateResponse六字段与catalog固定32长度全不变。

## 页面与呈现

语义HTML：一个main/一级标题；四个section分别呈现Company/Offer/Relationship/
Discovery，h2与dl/dd表达字段。使用现catalog所有相应label及semantic key，姓名/
公司/邮箱为纯文本，金额由state中的整数cents格式化USD。邮箱不做mailto链接。
展示所有现有业务字段，包括discovery预算上下限、约束和已记录下一步。
页面顶端始终可见boundary_notice，说明仅示例、不能输入/保存真实数据；不塞信任
程序名、权限流程或技术错误码。元数据不符合synthetic/false/none时不呈现业务内容。

固定四个type=button按钮以catalog action_*文字呈现，只分别发送既有四action。
搜索按钮aria-controls指向结果region并同步aria-expanded；首次隐藏。成功搜索后
展示query key与两条固定hits（section label+fact key），不添加搜索输入框/排序/自由query。
下一步region使用next_step_label为标题，按teaching.guidance的next_step_key、非空
detail_key、非空completion_key顺序显示；只把suggested_action用于强调对应现有按钮，
绝不自动触发。最终两行仍是沟通建议与仅示例修改完成，不能显示“已联系客户”。
两语言按钮由language_en/language_zh取文案，aria-pressed表当前选择；无业务表单。

视觉：浅中性背景、深色正文、单一深青色主按钮，清晰卡片边界，避免装饰性大面积
留白。内容max-width约1040px，左右至少16px；≥720px四卡两列，375px单列，动作
自动换行且窄屏全宽。原生系统字体，无远程字体；长文本可换行，无横向滚动。
按钮/语言控件至少44×44px，正常文本对比度目标≥4.5:1，可见3px focus-visible环，
不用颜色单独表达状态。顺序符合阅读顺序、无键盘陷阱，aria-live=polite宣布建议变化；
错误region为role=alert且tabindex=-1。无必须运动效果，prefers-reduced-motion禁动画。
这些是后续实际浏览器验收目标，不声称静态检查已证明布局/对比度/键盘体验。

## 唯一文字来源与最小静态回退

不增加catalog键，不改32数组/DTO或另建JS词典。所有加载后的业务/动作/语言/检索/
guidance文字只来自响应内同一catalog；语言只在内存en/zh中选择，默认en，document.lang
对应en/zh-CN。无navigator语言读取、URL参数、localStorage或cookie保存选择。
为catalog尚不可用时显示有意义内容，仅index.html允许以下独立静态双语区块：

| 用途 | en | zh |
| --- | --- | --- |
| loading | Loading the example… | 正在加载示例… |
| generic failure | The example is unavailable. Reload this page before continuing. | 示例暂时不可用。请重新加载此页后再继续。 |
| noscript | Enable JavaScript to use this example. | 请启用JavaScript以使用此示例。 |

这三块可同时呈现两语言，供启动及请求故障使用；不是key→文案字典，不被复制到JS。
初始HTML title可用静态Playground，成功后由catalog.page_title更新。正常业务区与空
按钮启动时hidden/disabled，不能暴露空控件。故障只显示上述通用提示，不反射JSON/
异常/HTTP自由文本；11个typed错误统一失败，不维护11份错误翻译。

## 客户端接口与状态

三个原生ES modules，零运行依赖；只app.js有一个fetch调用点：

- i18n.js：`createCatalog(entries)`验证32行、key唯一、en/zh非空字符串后返回Map；
  `translate(catalog, key, locale)`仅en/zh，缺key或非法locale抛本地固定错误；
  `formatUsd(cents, locale)`使用Intl.NumberFormat(en-US/zh-CN, USD)，不改原整数。
- consultant-ui.js：`render(snapshot, locale, searchVisible)`、`setBusy(busy)`、
  `showFailure()`；只读完整响应，用静态DOM节点/安全createElement、textContent/
  replaceChildren构建内容，无网络/存储/导航/业务状态修改。
- app.js：`requestState(action = null)`返回Promise<StateResponse>，action仅null或
  四个固定名称；null为GET `/api/playground/consultant`，否则POST唯一action端点，
  body恰JSON.stringify({action})，Content-Type=application/json。非法action在fetch前拒绝。
  `bootstrap()`仅浏览器有document时启动，模块在Node导入不访问DOM/发送请求。

fetch参数固定mode=same-origin、credentials=omit、cache=no-store、redirect=error；
URL均上述字面量，不读输入/URL/env或拼接任意地址。只接受200与符合合同的完整状态
JSON；检查三层metadata、必需字段类型、32项catalog及所有渲染key存在，失败不部分更新。
使用response.json/JSON.stringify原生工具，不建通用schema、JSON或请求框架。
没有polling、自动重试/自动POST、WebSocket/XHR/EventSource/beacon、worker、service
worker、cache API、IndexedDB/storage/cookie、telemetry、远程资源或动态import/eval。

app内存只存最新有效snapshot、locale、busy、searchVisible和失败标记；不写domain。
初次仅一个GET。动作请求期间busy且四按钮禁用、main aria-busy=true，旧快照保留展示
但不乐观更新；同一时刻一个请求，连续点击不能多发。语言选择在已有快照时可用且
不发请求；异步成功用当前locale呈现，避免旧语言覆盖。成功才整体替换snapshot并解锁。
搜索成功设searchVisible=true；修价/转客户保留该标记；Reset成功设false、保留locale，
所有事实以服务器响应为准。动作后焦点回原按钮；建议region宣布变化。
任何HTTP/网络/JSON/字段错误：隐藏旧业务快照/搜索，禁四动作，取消busy并显示/聚焦
静态failure区，不显示成功，不推测POST是否应用，不自动重发。用户刷新页面重新GET
服务器当前状态；刷新不是Reset，不自动重发上次POST。语言/视图状态不持久化。

## G06先行窄门契约

[S1-G06架构卡](cards/S1-G06.md) 依赖S1-05实际接受、旧gate writer停止。精确未来写集：

- `crates/consultant-playground/tests/support/boundary.rs`
- `crates/consultant-playground/tests/physical_boundary_source.rs`（仅闭包/lib接线）
- `crates/consultant-playground/tests/physical_boundary_assets.rs`（新增）
- `crates/consultant-playground/tests/support/source_closure.rs`（新增：迁移现有共享闭包/fixture helper）
- `crates/consultant-playground/tests/support/fixtures/assets-production.rs.txt`（新增）

先将现有source_closure_boundary、错误分类和目录fixture helper原样提取到
support/source_closure.rs，旧source测试与新assets测试共同调用；先确认纯移动回归，
不复制扫描逻辑、不改变旧拒绝语义。然后在同一共享helper加入asset阶段。
新src闭包精确assets/catalog/domain/http/lib五文件，旧四文件保持绿色；lib声明与
闭包配对。fixture只固定小型Rust资产映射和六个include字面量，不复制HTML/CSS/JS
整站作expected。沿source_boundary/RustLexer/shared wrapper/path检查，新增
AssetsProductionShape/AssetsTestModuleShape；include_bytes仅此模块六个固定目标可用，
禁止include!/include_str!/concat!/env!/绝对路径/未知文件/自由参数或运行时文件读取。
六个资产目标及assets根必须非symlink普通文件/真实目录，名单精确、禁止嵌套/额外文件；
复用source_root检查，新增仅六项平面清单断言，不建通用遍历器。旧阶段无assets模块时
允许目录不存在；新模块存在时任何缺失/逃逸拒绝。目录内不放tsconfig或测试文件。

G06新增 `asset_gate_accepts_only_closed_compile_time_mapping`、
`asset_gate_rejects_include_path_file_and_symlink_escapes`、
`asset_gate_preserves_source_module_and_wrapper_boundaries`，均含合法fixture与恶意变异，
复用既有错误断言。测试fixture可建临时六文件树，不能读用户数据。
能力验收另须核全部三JS：唯有app一个fetch点且固定两API，其他模块纯翻译/DOM；
静态imports限三脚本之间，HTML资源URL只指向六资产，CSS无@import/url外部资源，
favicon纯几何SVG，无script/foreignObject/外链。不得以粗关键词扫描宣称完整JS安全。
Node请求spy与能力探针验证路径/方法/无存储及自动调用；独立review核实际代码能力，
后续真实浏览器网络拦截补运行证据。**本门固定代码能力/文件范围，不固定每个像素/
每行文案**；改样式不需要重新复制expected网站，永久边界仍必须保持。

无新依赖/图片生成/网络工具；复用HTTPtyped接口、统一catalog、serde、RustLexer/
boundary/source-root与test_changed。新工具无，仅现物理门的六资产断言；不用新框架。
复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。
