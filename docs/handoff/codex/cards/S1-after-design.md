# S1 增量卡索引

S1-00 首个增量冻结 [动作契约](../playground-contract.md) 及以下配对卡，独立审阅
接受后由主线程串行分派 Luna；自动 S0 harness 不作为产品开发前置。

1. [S1-G01：动作 grammar 的窄门迁移](S1-G01.md) — 旧代码绿色，批准 fixture
   与恶意变异验证完成并接受后，才开放下一卡。
2. [S1-01：四个闭合纯动作](S1-01.md) — 精确 grammar 和完整纯 unit 行为验收。

[S1-G02](S1-G02.md)→[S1-02](S1-02.md) 是待独立接受的可序列化DTO增量候选，
仅配对source/manifest窄门与单向DTO；不声称已实施。

[S1-03](S1-03.md) 的双语catalog接口已冻结，依赖S1-02及先行G03实际接受；
[G03卡](S1-G03.md) 已冻结；精确接口见 [catalog契约](../playground-catalog-contract.md)，
等待S1-02接受后再认领。

[S1-04](S1-04.md) 的只读检索/guidance接口已冻结；等待S1-03和先行G04实际接受。

S1-05–11 及其他依赖开放仍 **Blocked design**：HTTP、
assets、server、浏览器、CLI、进程隔离及最终验收未冻结。沿用 standalone-v2
目标；需要时另给精确接口、依赖/源码门迁移、写集和测试，不能提前施工。
这里不维护第二份详细接口，也不因两张动作卡完成就声明 Playground 已可用。
