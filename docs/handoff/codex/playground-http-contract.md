# Playground 纯HTTP handler契约

**Revision 1 · 主线程独立审阅接受，接口Frozen；G05已形成独立架构卡，仍需实际验收。**范围仅S1-G05建议→S1-05，不启动server或声明前置通过。
依据 [standalone-v2 Task 3](../../superpowers/plans/2026-08-14-consultant-playground-standalone-v2-implementation.md)、
[domain/DTO](playground-contract.md)、[catalog](playground-catalog-contract.md)、
[teaching输出](playground-guidance-contract.md)。真实业务/AI后续Goal不变。

## 接口、存储与信任边界

新增http.rs，lib.rs仅在既有catalog/domain之后声明
`#[cfg_attr(not(test), allow(dead_code))] mod http;`。以下类型/字段可供未来同crate传输层使用：

```rust
pub(crate) struct PlaygroundHttpHandler { session: std::sync::Mutex<PlaygroundSession> }
pub(crate) struct HttpRequest<'a> {
    pub(crate) method: &'a str,
    pub(crate) target: &'a str,
    pub(crate) headers: &'a [(&'a str, &'a str)],
    pub(crate) body: &'a [u8],
}
pub(crate) enum AssetRoute { Index, Styles, I18n, App, ConsultantUi, Favicon }
pub(crate) enum HandlerOutcome { Json(HttpResponse), Asset(AssetRoute) }
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) headers: &'static [(&'static str, &'static str)],
    pub(crate) body: ResponseBody,
}
// ResponseBody派生Serialize并使用serde(untagged)，仅下列两种。
pub(crate) enum ResponseBody { State(StateResponse), Error(ErrorResponse) }
impl PlaygroundHttpHandler {
    pub(crate) fn new() -> Self;
    pub(crate) fn handle(&self, request: HttpRequest<'_>, bound_port: u16) -> HandlerOutcome;
}
```

唯一handler字段就是Mutex<PlaygroundSession>；new只用固定PlaygroundSession::new，
不接root/path/Store/Vault/backend/policy/clock或可注入session。不存请求/port/locale/
候选JSON，不持有借出的MutexGuard；响应仅拷贝DTO与静态catalog引用。
bound_port是未来传输层实际绑定端口的受信参数，不是HttpRequest字段；不能从Host/
Origin/body/env推导expected，也不能让请求覆盖。0属传输配置错误；CLI若以后允许
绑定0，传输层须传实际分配的非零port。此模块不读取环境、不bind/connect或调用模型。

headers保留重复项；名称按ASCII不区分大小写且不trim，值仅外层SP/HTAB可trim。传输层可正常
解析HTTP framing，但不得合并/丢弃Host、Origin、Content-Type重复项；若库已拒绝则
保留拒绝，不把多头伪装成单头。未使用的请求头不授予能力；不读取/设置cookie或认证。
Host/Origin仅是此合成loopback服务的请求边界，不构成用户认证或产品授权。

## 八条精确路由与拒绝顺序

| Method | 原始target | 结果 |
| --- | --- | --- |
| GET | / | Asset(Index) |
| GET | /assets/styles.css | Asset(Styles) |
| GET | /assets/i18n.js | Asset(I18n) |
| GET | /assets/app.js | Asset(App) |
| GET | /assets/consultant-ui.js | Asset(ConsultantUi) |
| GET | /favicon.svg | Asset(Favicon) |
| GET | /api/playground/consultant | 200完整StateResponse |
| POST | /api/playground/consultant/action | 验证/应用闭合动作后200完整StateResponse |

Asset只是给未来传输层的闭合派发标识，不是占位200响应；本轮不含资源字节、不读文件。
未来资产卡只把六个标识映射编译期内嵌资源，不能新增路径。Index=text/html、Styles=
text/css、三JS=text/javascript，均带charset=utf-8；Favicon=image/svg+xml。资产最终响应
同样no-store/nosniff/no-CORS，不由本卡伪造资产或修改旧UI。

handle严格按以下次序短路，所有拒绝不修改session且不反射输入：

1. bound_port=0 →500 invalid_configuration。
2. 恰好一个Host；trim后等于 `127.0.0.1:<bound_port>`，否则400 invalid_host。
   仅port80额外允许Host=`127.0.0.1`，适配浏览器省略默认端口。拒绝localhost/IPv6/
   通配/尾点/前导零port/用户信息/逗号列表/CR/LF/NUL；禁止以请求Host定expected。
3. Origin可缺失（命令行客户）；有则恰一项且等于 `http://127.0.0.1:<bound_port>`，
   port80亦允许 `http://127.0.0.1`；其他/重复/null/https/外源→403 origin_forbidden。
   精确同源浏览器POST允许；不处理preflight、不发任何Access-Control-*。
4. target必须非空origin-form，以单一/开头，无?、#、%、反斜线、CR/LF/NUL，
   无//或`.`/`..`段；不decode/normalize/redirect，违者400 invalid_target。
   其余非八条精确target→404 not_found（包括尾斜线/大小写/别名）。
5. 已知target对应method必须精确GET或POST；包括HEAD/OPTIONS在内其他方法→405
   method_not_allowed，并按该target加Allow: GET或POST；未知target先404。
6. body实际字节>256→413 payload_too_large，检查在JSON解析与锁之前；不信Content-Length。
7. 任意Content-Encoding头→415 unsupported_media_type，不解压或实现内容编码。
   随后GET任何非空body→400 unexpected_body；GET空体忽略Content-Type，不解析JSON。
   POST必须恰一Content-Type，trim后ASCII不分大小写精确application/json或
   application/json; charset=utf-8；缺失/重复/其他→415 unsupported_media_type。
   其他语法变体不隐式扩展media type解析器。
8. POST空/malformed/超深/非UTF8/重复字段/未知字段/多JSON值/错误类型/非法action
   →400 invalid_action_request。全部语法与闭合action验证成功才获取Mutex。
9. GET API或有效POST锁poison→503 session_unavailable；不into_inner/clear_poison/
   自动reset/recover。资产派发不获取锁。合法POST在同一锁内apply与构造响应快照，
   GET在同一锁内构造全部快照，离开handle前释放；不同handler从各自固定fixture开始。

正常HTTP framing错误由未来传输层拒绝，不在handler重写HTTP parser。传输层读取body
至多257字节，足以判定>256；遇超限须返回413并结束该请求/连接，不能先无界read_to_end。
未读取到完整body/错误framing不得包装成成功空请求。handler自身仍检查实际slice长度。

## 闭合请求与typed JSON

唯一POST对象为 `{ "action": "CorrectOfferPrice" }` 等四个现有精确action名称，
只允许一个action字段。`ActionRequest`派生Deserialize并deny_unknown_fields；
其私有WireAction派生Deserialize且 `serde(try_from = "String")`，TryFrom<String>
只match四个名称，再私有穷尽映射到PlaygroundAction。不可直接用默认enum反序列化，
以免接受 `{ "action": { "CorrectOfferPrice": null } }`；null/数组/数字/对象拒绝。
复用serde_json::from_slice，错误统一invalid_action_request；不手写JSON/parser/schema。
JSON合法转义按解码值判定，重复转义字段名也由serde重复字段检查拒绝。

StateResponse/ErrorResponse均pub(crate)且派生Serialize，字段私有；精确字段如下：

```rust
pub(crate) struct StateResponse {
    profile: &'static str, real_data_enabled: bool, persistence: &'static str,
    state: PlaygroundReadModel,
    catalog: &'static [CatalogEntry; 32],
    teaching: PlaygroundTeachingReadModel,
}
pub(crate) struct ErrorResponse {
    profile: &'static str, real_data_enabled: bool, persistence: &'static str,
    error: ErrorCode,
}
// ErrorCode派生Serialize、serde(rename_all = "snake_case")，仅上述11个错误：
// InvalidConfiguration, InvalidHost, OriginForbidden, InvalidTarget, NotFound,
// MethodNotAllowed, PayloadTooLarge, UnexpectedBody, UnsupportedMediaType,
// InvalidActionRequest, SessionUnavailable（合计11个）。
```

全部响应固定profile=synthetic_playground、real_data_enabled=false、persistence=none；
state/teaching沿现有完整DTO（含自身成熟度字段），catalog是唯一静态32行，无DTO→domain。
外层成功JSON恰6字段，错误恰4字段；serde(untagged)不新增State/Error标签。
所有JSON响应头恰 `Content-Type: application/json; charset=utf-8`、
`Cache-Control: no-store`、`X-Content-Type-Options: nosniff`，仅405另加上述Allow。
无Set-Cookie/CORS/Location/自由输入message；未来传输层可加HTTP framing必要长度，
不篡改此应用头。ResponseBody直接serde_json::to_vec，保留Result，不unwrap或反射错误。
这些固定可序列化类型不含用户字符串；序列化故障是传输内部失败，不能提交另一个动作
或假成功。输出编码及socket故障的关闭行为由后续传输卡冻结，不新增第12个业务错误。

## S1-G05先行窄门契约

[S1-G05架构卡](cards/S1-G05.md) 依赖S1-04实际接受、先前gate writer停止；先审门再派S1-05。精确未来写集：

- `crates/consultant-playground/tests/support/boundary.rs`
- `crates/consultant-playground/tests/physical_boundary_source.rs`（仅现inventory/lib配对接线，不追加大批测试）
- `crates/consultant-playground/tests/physical_boundary_http.rs`（新增）
- `crates/consultant-playground/tests/support/fixtures/http-production.rs.txt`（新增）

闭包从3源增加到精确catalog/domain/http/lib四源，旧3源仍绿色；新闭包必须与新lib
声明配对，保留其他所有旧生产grammar/manifest/source-root/path/symlink/wrapper约束。
source_boundary沿现签名增加http分支；新增HttpProductionShape/HttpTestModuleShape，
复用RustLexer与共享wrapper/path helper。http完整生产fixture须按本契约的接口、
短路顺序、常量和闭合映射人工写定，并作为G05候选独立审阅；不得从后续生产候选刷新。
只接受该完整token形状；仅明确std::sync::Mutex及serde/既有domain/catalog引用可出现。
不是开放任意std/同步/IO/HTTP库或泛用路由/授权框架；不改RustLexer/manifest/依赖。
G05只改测试，无生产handler。若完整fixture无法遵守本契约，提交具体设计缺口，不
让S1-05自由改验收标准；源码/行为正确性仍由S1-05真实unit tests验证。

G05新增：`http_gate_accepts_old_and_pinned_next_closure`；
`http_gate_rejects_authority_body_and_action_widening`（非唯一Mutex字段、从Host定expected、
放宽Host/Origin/body上限、enum-map、额外action/外部backend、移除deny_unknown_fields、
锁poison自动恢复、CORS/自由消息/IO/env/network均拒绝）；
`http_gate_preserves_path_wrapper_and_module_rejections`（额外module/路径/symlink/测试wrapper
及尾随代码拒绝）。保留所有既有恶意变异与具名错误断言。

G05及S1-05命令沿卡；tiny_http暂不依赖/调用。后续传输直接复用handler与八路派发，
可另行精确准入现CLI已有tiny_http0.12，但不能改旧UI/Workspace或把监听端口变为业务输入。
复用serde/serde_json、内部session及两DTO、统一catalog、RustLexer/boundary/source closure/
test_changed；新产品模块是纯handler，新增通用工具无。
复用以上，禁止重新实现同类工具；需要新工具先在简报回复中申报。
