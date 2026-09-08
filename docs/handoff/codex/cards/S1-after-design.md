# S1 后续卡：尚未获准施工

**这些任务尚缺 S1-00 的精确接口与门迁移契约，不能派给 Luna。**这里是它们的唯一详细草案；[milestones](../milestones.md) 只作索引。架构冻结某张卡后，将其移入单独文件，并把本页该节替换成链接，避免两个 truth。

所有功能卡共用边界：仅合成 fixture，禁止任意业务输入/持久化/产品依赖/模型；既有 Ui、assets、Workspace、导出和校验不变。controller 分派时必须填写确切 base、白名单、测试与报告，不以此处预计目录充当写入许可。

## S1-G01 — 现有 source checker 的窄迁移

- 角色 architect；依赖 S1-00。
- 预计写入仅 physical_boundary_source.rs、tests/support/boundary.rs 与 S1-00 指定的 fixture；不改生产源。
- 保留扫描入口、lexer、源根/符号链接检查；只接纳下一张动作卡的冻结形状。
- planned：原源通过、批准动作形状通过；额外动作、自由参数、IO、env、路径逃逸、未知模块均拒绝。
- gate：cargo test -p sovereign-consultant-playground --locked、test_changed、独立强模型审阅。

## S1-01 — 四个闭合动作

- 角色 worker；依赖 S0-06、S1-G01。
- 预计仅 domain/动作模块与其测试、最小模块接线；不能修改边界 gate。
- 实现 CorrectOfferPrice、PromoteAcmeToCustomer、ShowReportingSearch、Reset；所有接口由 S1-00 冻结。
- planned：逐字段 before/after、全部可达状态、重复动作与精确重置；任意价格/ID/文本不可表达。
- gate：leaf tests + test_changed；无 source 形状批准则停止，不能自行更新 expected。

## S1-G02 — manifest 检查的精确依赖迁移

- 角色 architect；依赖 S1-00，并在 S1-02 前完成。
- 预计写入 physical_boundary_manifest.rs、tests/support/manifest.rs 及批准 fixture；不提前加生产依赖。
- 仅接纳 DTO 所需批准依赖。保留 unpublished、禁止 build script、单一 lib、source-root 与 kind/target/features 检查。
- planned：非批准名称、来源、kind、target、features/default-features 任一偏差均拒绝；tiny_http 等待 server 阶段另行准入。
- gate：leaf tests + test_changed + 独立 reviewer；不得把“非空一律拒绝”改成“全部接受”。

## S1-02 — 单向 read model

- 角色 worker；依赖 S1-01、S1-G02 及对应 source-closure 批准。
- 预计 read_model 模块、测试、最小模块接线和已准入 serde/serde_json 依赖边。
- planned：固定 profile/real_data_enabled/persistence；graph 不实现序列化；无 DTO→graph/Workspace 构造路径。
- gate：leaf tests + test_changed；必须测试 JSON keys 结构，不用子串判断。

## S1-03 — 双语 catalog

- 角色 worker；依赖 S1-02 和新模块形状批准。
- 预计 localization 模块、catalog 测试与模块接线。
- planned：每个 key 中英文完备、占位符一致、locale 变更不改 graph。
- gate：leaf tests + test_changed。

## S1-04 — 固定检索和 guidance

- 角色 worker；依赖 S1-03 和形状批准。
- 预计 query/guidance 模块与测试。
- planned：固定 reporting hits，所有可达状态的确定性下一步，查询前后 graph 相同；拒绝自由 query/filter。
- gate：leaf tests + test_changed。

## S1-05 — 有界请求与 typed HTTP handler

- 角色 worker；依赖 S1-04 和所需 source/dependency 准入。
- 预计 http 模块和独立测试；handler 只持有 Mutex<PlaygroundSession>。
- planned：空/malformed/duplicate/unknown/超 256 bytes、错误 method/Host/query action 拒绝；成功/error JSON 和 headers 精确。
- gate：leaf tests + test_changed；错误数据不能进入 graph。

## S1-06 — 六个嵌入资源与基础布局

- 角色 worker；依赖 S1-05 和 include/asset 边界批准。
- 预计 leaf assets、资源 manifest、独立 tsconfig 和必要接线。
- planned：精确资源/MIME；375px、44px 控件、语义 HTML、textContent；无业务表单、远程资源、telemetry/storage。
- gate：leaf tests、独立 Playground tsc、test_changed。
- 本卡验收基础布局；完整动作交互留到 server 已存在的 S1-08。

## S1-07 — loopback server

- 角色 worker；依赖 S1-06、S1-00 的生命周期契约与单独 tiny_http/source 准入。
- 预计 server 模块、route tests 和已批准依赖边。
- planned：literal 127.0.0.1，八条规定 method/routes，typed no-store 错误，无 wildcard/alias/outbound。
- gate：leaf tests + test_changed；提供已冻结的测试启动路径供下一卡浏览器验收，尚不改 CLI。

## S1-08 — 浏览器动作和反馈

- 角色 worker；依赖 S1-07。
- 预计 leaf assets 与独立浏览器测试；已存在的 server 提供真实页面。
- planned：四动作、错误、重置、双语切换；仅两 API；实际 keyboard/focus/reduced-motion；无隐藏网络或浏览器持久化。
- gate：实际浏览器路径、leaf tests、Playground tsc、test_changed。

## S1-09 — 增量 CLI 接线与产品兼容性

- 角色 worker；依赖 S1-08。
- 预计 apps/cli/src/main.rs、精确 manifest/lock 依赖边、独立兼容测试；Ui variant/arm、ui.rs、现有 assets/workspace 禁止修改。
- planned：playground 仅 --port，默认 7788；不打开浏览器；原 Ui help/flags/transcript 和导出/校验一致。
- gate：sovereign-cli + leaf tests + test_changed。
- 同目录已有 main.rs/Cargo.lock 写入任务必须结束，不能并行碰共享入口。

## S1-10 — 两个真实进程的隔离证明

- 角色 worker；依赖 S1-09。
- 预计新独立 integration tests 与获准 support；不改既有 gate 条件。
- planned：内容不同的两组 fake data roots；完整 HTTP/stdout/stderr 字节一致、无 canary、roots 递归不变；重启只复原 fixture。
- gate：真实进程测试 + test_changed。不得启动 legacy demo 触碰实际 owner root。

## S1-11 — 独立验收与产品交付

- 角色 reviewer；依赖所有 S1 功能与必要迁移卡。
- 仅报告、产品成熟度和 controller 队列记录；不在验收角色里修源码。
- 完整 standalone-v2 final gate：all-features clippy、workspace tests、release build、Playground tsc、现有 UI tsc、file-size、diff check、两进程和兼容性证明。
- 实际走通中英文路径；没有五名顾问的实际观察就保留 Target，不伪造访谈结果。
- 交付可运行入口、准确能力声明和下一卡。完整 MVP Goal 仍有效时继续 S2-00 的架构冻结，再按受审小卡施工；不把 S1 验收当作整个目标完成。
