# 音乐适配器契约与迁移记录

本文记录 [ADR 0006](adr/0006-music-provider-adapters.md) 的契约、已完成接线与后续注意事项。2026-10-10：阶段一至五已完成，包括统一契约、独立适配器注册表、业务与账号迁移、受限外部 WASM 验证、默认网易云原生源码包拆分和适配器设置。历史曲库／队列存储仍保留旧格式；第三方平台的完整权限、运行时包安装及网易云 WASM 化未实现。前文各阶段的“尚未实现”保留历史语境，当前网易云执行方式与管理入口以末尾阶段五记录为准。

2026-10-12：ADR 0007 阶段一开始接线。来源角色配置独立于账号和适配器启停；实际队列及曲目表新增 `online` 兼容项，保存完整统一曲目和真实来源，旧网易云／本地记录不重写。统一详情导航、游标与菜单 DTO 可保留其他平台身份；尚无真实第二平台及聚合搜索。范围与验证见 [实现记录](implementation.md#当前与补充来源基础接线adr-0007-阶段一2026-10-12)。

## 标识与曲目

`EntityRef = { source, kind, id }`，`kind` 为 `track / playlist / album / artist`。`source` 是平台的稳定标识，与包 ID、版本和安装目录无关；`netease` 永久留给网易云，`local` 留给宿主本地曲库，不注册为在线适配器。来源标识为 1–64 字节、以小写字母开头的小写 ASCII 字母、数字和连字符。同一来源同一时间只能注册一个活跃提供者；重复注册拒绝，不按安装顺序覆盖。

`id` 是 1–1024 字节的不透明 UTF-8 字符串，拒绝全空白与控制字符。宿主不转换数字、不裁剪、不改变大小写、不拆分分隔符；`01` 与 `1`、`A` 与 `a` 是不同 ID。`EntityRef::key()` 将来源、实体类别、ID 编码为 JSON 元组，避免拼接分隔符产生碰撞。数字 ID 的解析只发生在对应适配器内部。

`MusicTrack` 包含曲目引用、标题、别名、显示歌手、结构化歌手引用、专辑及其引用、时长、封面和 `associations`。歌手/专辑引用保持曲目来源，跨来源曲目匹配写入 `associations`，不替换身份。本地路径不属于公共曲目；由宿主维护 `LocalBinding`。本地网易云歌词绑定保留为关联，本地歌手和专辑不能因绑定被误认成网易云实体。`MusicTrack::validate` 检查请求身份、实体类别、来源与 64KiB 序列化上限；接线时还要在读取响应之前限制原始响应大小。

## 基础接口与能力组

后端 `MusicAdapter` 的必选方法：

```text
descriptor() -> SourceDescriptor
read_track(EntityRef, RequestContext) -> MusicTrack
resolve_playback(ResolveRequest, RequestContext) -> PlaybackResource
```

返回异步 `Send` future，不依赖 WebUI 挂载，不在音频线程运行。`SourceDescriptor` 声明版本与可选能力组：搜索、浏览、账号、用户曲库、收藏、歌单写入、推荐、私人 FM、歌词和下载。可选组的方法随业务迁移确定；声明能力并不表示阶段一已实现该业务。描述符版本必须与宿主契约版本完全一致；当前拒绝未知版本，不套用功能插件的 `pluginApi` 版本。外部包 Manifest、WIT/ABI 及升级协商留到阶段四，内部和外部将共用语义与契约测试。

`OperationAvailability` 单独表达当前账号/曲目是否可执行及不可用错误码。缺少能力返回 `unsupported`；支持收藏但未登录返回 `unauthenticated`，不能把“平台支持”当作“当前可以操作”。平台特有能力作为后续可选扩展，不扩大基础接口。

分页使用 `PageRequest { cursor, limit }` 和 `Page { items, nextCursor }`。第一页 `cursor = null`，末页 `nextCursor = null`；平台 offset/page/token 由适配器解释。游标最多 4096 字节，与实体 ID 分开。`limit` 为 1–100，返回数量不得大于请求数量，序列化结果最多 2MiB。调用方按来源、查询、账号和实例绑定游标，切换查询后丢弃旧页；这些约束不会更改现有 UI 的 30/50 条分页。

## 错误、会话与取消

`MusicError` 只交付稳定 `code` 与可选 `retryAfterMs`：未登录、不存在、权限不足、地区限制、限流、网络失败、能力不支持、来源不可用、取消、超时、过期上下文、无效数据和内部错误。宿主按错误码提供中文说明，不把原始平台响应直接交付用户或普通插件。诊断需要单独过滤敏感信息；契约刻意没有自由文本诊断字段。

`SessionContext` 包含来源、可空账号 ID 和账号代次。匿名请求也绑定来源及代次。`RequestContext` 增加适配器实例代次、单调时钟 deadline 和共享取消标记。管理器从调用身份构造上下文，不能接受插件自报来源或代次作为授权。请求开始、缓存命中、返回结果、成功写操作通知和预加载发布时都要复核上下文；账号切换、退出、删除、刷新成功及会话失效推进账号代次，停用/重载推进适配器代次。

阶段一的 `check` 是可复用校验函数，尚未接入运行时。阶段二管理器还必须在等待期间主动执行超时和取消；不能依赖适配器主动轮询。适配器停用后拒绝新查询、取消未完成工作、撤销资源和凭证访问；已经交给播放核心且无需适配器继续服务的资源可继续播放。队列保留身份，在来源恢复后重新解析。

## 播放资源与缓存

`ResolveRequest` 包含曲目引用、平台定义的首选音质字符串及允许降级开关。`PlaybackResource` 分开保存公共 `ResourceMetadata` 和后端 `ResourceAccess::Http`。元数据包含实际音质、完整/试听范围与可空绝对 Unix 毫秒有效期；试听明确携带开始和结束毫秒，默认完整播放请求通过 `require_full` 拒绝试听。未知音质命名不跨平台作字符串排序；具体降级顺序由适配器实现。

URL 与请求头只供受控后端消费，不可序列化；`HttpAccess` 的 Debug 整体隐藏。资源交付播放核心前由宿主校验协议、URL/头部大小、重定向和响应预算，建立有界句柄表。`ResourceHandle` 只是身份，单独持有它不授予访问权；`ResourceLease` 同时绑定会话、实例和播放代次、单调时钟最大存活期限。未知平台有效期仍须有宿主期限；已过期、账号变更或重载后的资源重新解析。普通插件与持久化队列不获得 URL、请求头、资源句柄或凭据。

阶段一尚未实现网络校验、句柄表或调度器，不代表现有播放已经受到新租约保护。阶段二须复用现有 HTTP 音频读取和原播放命令队列，关键播放解析与普通查询分开预算。初始契约数据上限是设计限制，未测量延迟/内存收益；超时、并发和资源句柄数量由接线阶段根据现有预算确定并记录。

`CacheScope` 按来源、账号、账号代次、实例代次、操作和规范化业务参数隔离。参数不含凭据或临时播放地址；账号匿名与登录分开。`CacheAdvice` 仅建议最大新鲜时间，容量/TTL 由宿主裁剪。成功写入用 `WriteImpact` 描述受影响操作和实体；宿主在当前上下文下失效结果及旧在途写回，失败不清空已有结果。缓存命中也检查来源是否启用，不能绕过管理器。

现有 `nativeCall`、`runtime-cache.ts`、Rust 歌曲缓存及统一歌词服务继续使用，阶段一不新建缓存。迁移读取/写命令时同步改 [缓存与本地音乐](cache-and-local.md) 的键和失效表；歌词来源与音频来源独立，不为了新增适配器改写歌词回退策略。

## 三类管理器与账号边界

| 模块                             | 独立持有与负责                                                                                  | 不负责                                                       |
| -------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| AdapterManager（阶段二已接内置） | 来源注册表、包发现/安装、版本校验、加载/启停/重载、实例代次、能力调度、取消、资源撤销、运行故障 | 平台登录协议、账号凭据持久化、功能插件注册表、音频解码       |
| AccountManager（阶段三起实现）   | 来源与账号记录、当前选择、系统凭据保存、会话代次、实例范围授权、退出/删除本地清理与状态通知     | 二维码/密码/OAuth 步骤、凭据字段解释、平台刷新与远端退出协议 |
| PluginManager（已有）            | 功能插件安装、授权、运行注册表、贡献与生命周期                                                  | 适配器注册与加载、平台账号持久化                             |

安装/包校验/配置等基础设施可复用，注册表和状态分别持有；不可让 PluginManager 加载适配器再转交。平台协议留在适配器，通用音乐服务负责业务路由；本地目录、索引和权限仍在宿主。

`AccountRef` 是来源与不透明账号 ID，`AccountRecord` 仅含公开信息。适配器登录完成后交付记录和 `OpaqueCredential`（1–64KiB 二进制数据），由 AccountManager 保存；凭据没有序列化/克隆派生，Debug 隐藏内容，但不宣称内存零化或恶意代码隔离。`AccountAccess` 是后端读取/刷新替换的边界，每次调用验证来源、账号、账号代次和活跃实例；阶段一只有 trait，尚无存储实现与跨来源访问验证。

停用不删除账号或凭据；账号退出先隔离旧会话，本地清理与适配器远端退出分别报告。删除保存账号清理该账号记录和系统凭据，不等同于停用来源。普通配置、插件 storage/secrets、文件和浏览器存储不得另存平台凭据。现有显式授权的 `account:credentials` 继续作为敏感兼容入口，不能作为新通用接口前提。原始凭据仍只能交给受信任适配器；此契约不是阻止恶意代码复制/外发凭据的沙箱。

## 历史数据迁移规则

阶段一的 `migration.rs` 提供**只读、无 IO 的转换函数**：`legacy_track`、`legacy_queue`、`legacy_account`。不执行启动迁移，不写数据库，不访问系统凭据库。存量 `Track`、`PlayerSnapshot`、前端 `player.ts` 和公开功能插件 SDK 保持当前格式；统一类型不提前替换旧存储。后续迁移实现必须遵循以下规则。

| 历史位置                                                                                        | 转换与保留规则                                                                                                                                                                                                        |
| ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SQLite `tracks.value` 的 `kind: netease` 与数字 ID                                              | 按 source 的 ID 转为十进制字符串 `netease/track/<id>`，不从 key 或歌名猜 ID；结构化歌手/专辑同样加来源。保留旧 key → 新引用映射，超出 JS 安全整数的 Rust ID 转换不丢精度。                                            |
| `kind: local`、`local:<path-hash>`                                                              | 来源为 `local`，ID 使用原哈希，不重新按平台路径规则计算；原路径和封面资产放在 host-only `LocalBinding`。`neteaseId` 仅转为关联；公共本地曲目封面留空，由宿主根据绑定交付受控封面。                                    |
| `local_origins`、`local_file_stats`、`local_metadata`、`local_members`、`local_playlist_tracks` | 通过同一旧 key 映射在同一事务中更新关联，保留目录来源、文件统计、元数据、歌单顺序；不重扫、不改原始文件。真正迁移这些表留到统一存储切换。                                                                             |
| `settings.player`                                                                               | 转换为 `SavedQueue` 版本 1，保留顺序、重复项、当前索引、随机顺序、音量和设备；复用旧随机状态修复规则。运行状态、position、revision、错误、实际音质与 FM 会话不恢复；重启后停止，播放时重新解析资源。                  |
| 系统凭据 `NonsPlayer / netease-session`                                                         | 当前是单一 Cookie，没有持久账号列表。阶段三由网易云适配器读取/校验登录信息，再把公开 `userId` 转为字符串账号 ID，凭据字节原样交给 AccountManager。网络失败/无法确认身份则保留原条目为待迁移，不虚构账号，不清空登录。 |

`legacy_track` 上限 64KiB，`legacy_queue` 上限 8MiB/1000 首；损坏 JSON、未知来源、无效必要 ID、空本地路径或越界队列索引返回 `invalidData`，不静默过滤队列项。`legacy_account` 只转换已确认的公开 profile，不解析 Cookie。不透明凭据是后续存储归属迁移，不能作为普通 SQLite 设置写入。

持久化切换时先读取并完整转换、校验，再在 SQLite 单一事务内更新曲目、引用关系、队列和 schema 版本，失败回滚并保留旧数据。相同新身份可合并曲库记录，但不能去重播放队列或改变歌单顺序；封面绑定必须一起保存，不能仅保存 `MusicTrack` 后丢失本地封面。迁移标志随事务提交，重启不重复迁移。

系统凭据库与 SQLite 不能组成原子事务：先按来源/账号写新凭据并验证可读，再提交记录与迁移完成标志，最后删除旧条目。中断后通过标志与目标凭据复核恢复，目标已有凭据时不覆盖较新账号状态；删除失败保留可重试状态。尚未确定账号身份时不提交完成标志；日志和迁移备份不得记录原始凭据。此流程需阶段三存储与故障注入测试后才能启用。

## 本轮修改与继续开发注意事项

- 新代码集中在 Rust `music/` 功能目录，`lib.rs` 只导出契约模块；未新增第三方依赖，未改现有播放、账号、缓存与功能插件行为，因此本轮不添加用户可见 CHANGELOG 条目。
- 契约测试覆盖不透明 ID 与反序列化拒绝、能力/账号限制区分、取消/超时/来源及账号/实例代次、分页数量/大小、资源有效期/播放代次/试听、敏感 Debug 隐藏、旧在线与本地曲目、混合重复队列及账号转换、缓存作用域。测试通过只证明这些函数的语义，不能证明尚未接线的管理器授权或运行预算。
- 阶段二先实现 AdapterManager 并注册内置网易云，复用现有 Rust Client；迁移 `song` 和在线 `resolve` 的调用方。本地 NCM 解码继续归宿主，与网易云平台类分离，保留混合队列与无缝预加载回归。账号仍在旧实现期间，通过受控桥接产生会话代次，不在适配器普通存储另存 Cookie。
- 阶段三再实现 AccountManager、存储迁移和各业务能力，公开统一客户端与 TypeScript DTO；更新 `netease.*` 兼容入口说明。阶段四通过测试适配器验证真正的调度/越权拒绝/停用恢复与外部 ABI，再拆包。
- 本轮验证环境与结果见 [实现记录](implementation.md)。未测性能，不把序列化上限当作进程内存预算；macOS/Linux 构建与基本播放仍需对应环境验收。

序列化实现使用既有 Serde，通过 Context7 核对官方 [容器属性](https://serde.rs/container-attrs.html)：字符串新类型使用 `try_from = "String"`，让反序列化经过与构造一致的校验；实体与资源枚举沿用 tagged representation，没有引入自制序列化框架。

## 阶段二修改记录与注意事项（2026-10-10）

### 已接入调用链

- `music/manager.rs` 实现独立来源注册表、版本／重复来源校验、内部启停／重载与请求执行；注册表和生命周期不经过 PluginManager。`music/netease.rs` 实现内置适配器与启动接线；`MusicService` 接收统一管理器，播放 Actor 不依赖 `Netease`。平台响应和数字 ID 仅在网易云适配器／兼容边界解释。
- 插件单曲缓存继续由现有 `plugins/netease.rs` 持有，查询改经 `MusicService → AdapterManager → MusicAdapter.read_track`，没有新建第二份歌曲缓存。缓存命中和写回核对来源启用、会话代次、实例代次以及原清理代次；旧账号和重载前结果不能回填。原 `netease.get-song` 与兼容别名 `music.get-song` 保持公开形状和权限；原始凭证接口仍是独立的显式授权兼容能力。
- 当前在线曲目与下一首预加载都经统一 `resolve_playback`，音质降级、试听与有效期转换为统一资源语义。宿主只接受完整且未过期资源，再校验访问描述并登记后端资源所有权。解析结果进入有界播放命令队列时再次核对；无缝 streaming 回调只执行轻量有效性检查并设置已准备 URI，不查询网络／数据库。暂停过久造成准备资源失效时，播放 Actor 丢弃准备并按原重试预算重新解析。
- 本地文件／NCM URI 准备迁入 `local/playback_resource.rs`，仍使用原双任务解码预算、临时文件复用与释放策略；本地不会因歌词关联 ID 路由到在线适配器。歌词、队列持久化和 WebUI 数据形状保持现有实现。

### 当前预算与访问限制

| 项目             | 阶段二限制                                                                                                                       |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| 单曲读取         | 每来源最多 4 个并行调用、总期限 3 秒；超载直接返回 `rateLimited`，不建立额外等待队列                                             |
| 播放解析         | 独立于查询的每来源 2 个并行调用、总期限 12 秒；沿用当前歌曲与下一首任务取消                                                      |
| 等待中的失效检查 | 管理器每 10ms 检查来源／会话／实例及取消标记，期限独立执行；无需适配器配合，失效后丢弃 future。10ms 是调度间隔，不是实测响应承诺 |
| 元数据与原始响应 | 公开单曲最多 64KiB；单曲／播放解析的原始响应读取最多 2MiB；其他旧业务使用 16MiB 读取上限                                         |
| 后端资源登记     | 最多 16 项存活所有权；弱引用在新资源登记时回收，句柄不开放 IPC；当前和预加载消费者持有不可序列化 token                           |
| 交付有效期       | 平台绝对有效期与宿主 60 秒上限取较短者；不沿用请求 12 秒 deadline 作为准备资源期限。有效期未知也最多准备 60 秒                   |
| HTTP 访问        | 只接受 HTTP(S)、URL 最多 8192 字节，拒绝用户信息和 fragment；请求头非空返回 `unsupported`，不静默丢弃                            |
| 重定向与音频预算 | 平台客户端与内置音频源关闭自动重定向。音频继续使用既有 4MiB 缓冲、12 秒读取期限及最多 2 次范围恢复；不预下载整首、不新建音频缓存 |

目前 URI-only 音频管线仅支持内置网易云返回的无附加请求头资源。要求凭据头的外部适配器必须先补受控传输接线和对应测试，不能把头或带凭据 URL 交付 WebUI／功能插件。`HttpAccess`、资源 token 和 `ResolvedTrack` 的 Debug 隐藏访问数据；临时 URL 不写入持久队列。已经交给播放核心、无需适配器继续服务的资源可播至结束；停用后新读取和未交付预加载被拒绝，账号与队列不删除。

`vendor/ncm-api-rs` 保留此前固定上游 commit `133b65bfe482e41ebccf018870d3fce07bf58eb3` 的源码与许可证，只补读取前大小限制、可配置上限、稳定过大响应错误和重定向策略；没有自建网易云协议／加密实现。普通客户端与用于基础音乐调用的克隆共享原连接池。升级须核对 [补丁记录](../adapters/netease/vendor/ncm-api-rs/PATCHES.md)，重跑有界响应及宿主回归；不要改 Cargo 为未补丁的上游版本后继续宣称网络上限有效。上限是有效载荷限制，不是 JSON 展开、HTTP/TLS 或进程内存测量。

### 后续阶段注意事项

- 阶段二会话桥接只表达既有单会话的代次，`account = null` 不代表凭据一定为空，也不虚构持久账号 ID。扫码成功、退出、确认无 profile 以及适配器收到未登录错误推进代次；网络失败不当作已退出。阶段三用 AccountManager 的真实来源／账号上下文替换桥接，再启用凭据与数据库迁移。
- 内部启停／重载可验证撤销与恢复，但未实现包发现、安装、外部实例替换或用户管理入口；这些随外部接线完成。内置描述符暂不声明尚未迁移的可选能力组；不要把现有网易云搜索／写接口误当成统一 trait 已有方法。
- 旧数字队列和 TypeScript DTO 暂时保留，兼容转换有明确网易云边界；统一管理器和 trait 接受非数字 ID。阶段三公开统一客户端时再清理兼容 DTO，阶段四以测试适配器验证多来源分页、账号与外部执行预算；本轮测试适配器只用于验证已经接线的基础生命周期。
- 继续沿用 `nativeCall` 查询缓存、Rust 单曲缓存和独立歌词服务；新增业务读取／写入时按缓存影响表接入，不能缓存临时播放地址。没有账号／曲库 schema 迁移，也没有改变歌词来源顺序。
- 本机验证结果见 [实现记录](implementation.md)。Windows fakesink 与本地测试不能替代 macOS/Linux 构建、真实账号所有音质、扬声器或 Release 内存／首声延迟实测。

Tokio 的 future 丢弃／期限语义经 Context7 核对；ncm-api-rs 的响应限制查询未命中文档，改查上述固定版本官方源码，确认原客户端整包读取后补丁。HTTP 读取与重定向复用 Reqwest API，没有新建网络框架。

## 阶段三修改记录与注意事项（2026-10-10）

### 业务归属与修改清单

- `music/business.rs` 定义统一业务与账号展示 DTO，`manager.rs` 负责校验、能力／账号判断、请求预算、会话隔离、游标和写入代次。适配器可选 `business`／`account` 方法默认返回 `unsupported`。`netease_business.rs` 复用既有固定版本平台客户端，完成平台转换；没有自建网易云协议或加密库。内置来源声明 Search、Browse、Account、UserLibrary、Favorites、PlaylistWrite、Recommendations、PrivateFm 八个能力组。
- 统一读取覆盖搜索／建议、实体详情、歌单／专辑曲目、艺术家歌曲／专辑、歌曲补充信息、曲库预览、收藏列表／喜欢、历史、推荐歌单／分类／私人雷达与推荐／FM 曲目。写入覆盖喜欢、歌单新建／编辑／删除、添加／移除歌曲及 FM 不喜欢。错误仅交付稳定 `MusicError`，平台原始响应不进入通用 DTO。
- `music/compatibility.rs` 是宿主旧数字 DTO／offset 的迁移桥，所有在线业务都经统一管理器；`application/mod.rs` 不再直接调用平台业务。现有页面、播放快照、TrackSource 和 SQLite 曲库／队列仍保留旧形状；这不表示已完成统一存储迁移。旧“播放整个收藏”桥按页读取、最多 1000 首、整体期限 12 秒，保持顺序和重复项。
- `music/accounts.rs` 及 `accounts/tests.rs` 管理公开记录、当前选择、代次、系统凭据和恢复；`storage.rs` 只保存公开 JSON。`netease/mod.rs` 的平台客户端只读受控会话快照，二维码轮询返回待确认凭据，账号管理器才持久化。`account/` 界面新增保存账号选择／删除及添加账号，退出显示本地和远端结果；异步结果按账号变更事件隔离。
- `features/music/client.ts` 与 `packages/plugin-sdk/music.d.ts` 提供宿主客户端和公开类型；插件 `useMusicSource` 与后端 `music.sources/read-track/query/write` 提供通用能力。`plugins/netease.rs` 复用原 128 条歌曲缓存，以 EntityRef 为身份并复核账号／实例／写代次；没有新增歌曲缓存。灵动岛后端改用 `music.read-track`。

### 账号持久化、恢复和边界

公开索引在现有 SQLite settings 的 `musicAccountsV1`：记录、每来源当前选择、旧凭据迁移标志和待删除墓碑，账号记录与删除墓碑各最多 100 条；连续物理删除失败使墓碑达到上限时，拒绝新增删除并保留待处理账号，避免写出无法恢复的索引。凭据只存于系统 keyring 的 `NonsPlayer / music-account-v1-<SHA256(AccountRef JSON)>`，使用二进制 secret API；不写 SQLite、插件 storage、前端缓存或日志。它是存储命名空间，不能作为插件持有的访问授权。

旧 `NonsPlayer / netease-session` 用 password API 读取后转换为 UTF-8 Cookie 字节，避免 Windows 旧密码 UTF-16 与新 binary secret 混读。只有适配器通过 profile 确认真实 userId 才迁移；网络错误不提交身份或完成标志。顺序是写新 secret、读回验证、提交公开索引与标志、删除旧 secret。写入／读回／公开提交失败保留旧条目；中断后发现已有目标时不覆盖较新凭据，完成标志存在时重启重试旧条目删除。重新登录同一账号若公开提交失败，恢复原 secret。

账号删除先提交墓碑并撤销选择，再清理 secret；清理失败不会在重启时恢复被删除账号，启动重试。退出先准备旧会话的远端操作，再清理本地、推进代次，最后执行远端请求；返回 `LogoutReport { localCleared, localError, remoteError }`。`localCleared=true` 表示宿主已隔离本地账号，系统凭据物理删除仍可能失败并由 localError 提示。删除保存账号不会发送平台退出请求；停用来源也不退出或删除账号。确认会话失效撤销当前选择但保留保存记录，允许重新选择／登录。

适配器只获得绑定 RequestContext 的 AccountAccess，读取／刷新验证来源、账号、代次、实例与期限，刷新成功推进会话代次。账号变更发出 `music-account-changed`，现有喜欢状态、FM 与查询缓存丢弃旧结果。二维码 challenge 最多保留 4 个、180 秒过期，轮询同时核对生成时会话和实例；重新选择账号后旧二维码不能提交登录。当前 AccountRequest/Presentation 是内置二维码展示契约；不同平台的密码／浏览器授权展示和外部访问令牌仍需阶段四设计验证。

### 接口与兼容迁移

宿主 IPC：`music_sources`、`music_context`、`music_read_track`、`music_query`、`music_write`、`music_account`、`music_accounts`、`music_select_account`、`music_remove_account`。查询与写入分开，主 WebView 沿用既有受信任边界。宿主 `musicClient` 查询在缓存前后复核来源作用域；普通插件没有账号管理或原始凭据权限。

插件接口（详见 [插件文档](plugins.md#统一音乐客户端adr-0006-阶段三)）：

| 能力               | 权限                                                    | 参数／返回                                                       |
| ------------------ | ------------------------------------------------------- | ---------------------------------------------------------------- |
| `music.sources`    | `music:metadata`                                        | 来源描述列表                                                     |
| `music.read-track` | `music:metadata`                                        | `{ reference: EntityRef }` → `MusicTrack`                        |
| `music.query`      | 公开读取 `music:metadata`；账号相关读取 `music:library` | `{ source, request: MusicReadRequest }` → tagged `MusicResponse` |
| `music.write`      | `music:write`                                           | `{ source, request: MusicWriteRequest }` → 写入影响范围          |

`useMusicSource()` 对应 `sources/getTrack/query/write`。引用 ID 是字符串，调用方不得 parseInt 或跨来源替换引用；nextCursor 原样回传，同一查询保持参数和 limit，账号、实例或写入代次变化后从首页开始。结果为 `{type,data}`，读取响应没有 write，写响应没有曲目数据。来源缺少能力时返回 unsupported，当前缺少账号时返回 unauthenticated。

`useNetease().getSong(id)`、`netease.get-song` 以及 `music.get-song` 保留旧数字 ID／DTO 形状，已标记 deprecated，内部转统一单曲接口。`netease.account-credentials` 仅保留显式 `account:credentials` 的敏感兼容访问，不作为新的音乐查询或写入前提。新通用能力不得添加到 `netease.*`。历史队列迁移需要后续独立、原子数据库事务与本地绑定迁移，不能仅替换前端 Track 类型。

### 预算、缓存与后续验证

| 项目           | 当前限制                                                                                                                                                      |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 普通单曲读取   | 每来源 4 个并行、3 秒总期限                                                                                                                                   |
| 业务读取／写入 | 与单曲读取共用每来源 4 个并行、12 秒总期限；超载直接 rateLimited                                                                                              |
| 账号协议       | 每来源串行，最多 4 个调用（含执行中）等待，总期限 12 秒含等待                                                                                                 |
| 系统凭据任务   | 账号管理最多 4 个阻塞任务；内置适配器查询 credential 预算 4、播放单独保留 2；许可在阻塞闭包中持有                                                             |
| 请求／响应     | 业务与账号请求 16KiB；业务结果 2MiB；内置平台业务原始响应 16MiB、单曲／播放解析原始响应 2MiB；账号展示 64KiB；单曲 64KiB                                      |
| 分页／聚合     | 契约每页 1–100；内置分类搜索／艺人专辑／收藏列表／推荐歌单固定 30、歌曲搜索最多 50；游标 4096 字节；推荐批次 100、历史和艺人曲目按页截取；旧整收藏桥最多 1000 |

系统凭据 API 与 SQLite 为同步 I/O，spawn_blocking 已运行任务不能被强行中止，许可不会随被丢弃的 future 释放。凭据提交前再检查上下文，旧任务不能发布新选择；正在执行的 OS I/O 仍可能超出异步期限，不宣称 12 秒是账号磁盘处理的硬实时保证。阶段四仍需真实外部执行与不同平台 keyring 验证；现有原生只读探测工具仍使用旧 Cookie，迁移后不代表有已保存账号支持，真实登录验收应使用宿主。

`music_query/read_track` 加入现有 nativeCall 查询缓存，键带来源、会话／实例和写代次；旧在线缓存命中前后也查询 music_context，来源停用或重载不能绕过后端检查。写成功推进来源写代次，旧在途读取拒绝；`music-changed` 与账号事件清理现有查询缓存，失败不触发成功写入通知。新客户端写入使用整查询缓存失效作为保守兜底，兼容调用沿用原命令影响表；公开封面与独立歌词缓存仍遵循原策略。上下文复核增加本地 IPC；未测命中延迟，不将设计预算当作实测性能。

所有上限只是请求、序列化和任务数量限制，不等于 HTTP/TLS、JSON 展开或进程内存上限。未新增依赖。Windows 单元、DOM、WASM 宿主与生产构建记录见 [实现记录](implementation.md#统一音乐适配器adr-0006-阶段三2026-10-10)；真实账号登录、远端写入／退出、macOS/Linux 编译和基本播放、外部 ABI、多来源 UI、不同登录方式与 Release 性能仍需后续验收。本阶段不开放外部包安装，也不将内置受信任访问误称为防恶意适配器沙箱。

### 阶段三后续修复：大歌单预览原始响应预算（2026-10-10）

阶段三曾把 scoped 平台客户端和业务 checked 的原始响应一并压到 2MiB。网易云 playlist_detail 请求包含完整歌单元数据，然后宿主才提取 trackIds 并读取 12 首预览；合法的大歌单会在分页之前被误报 invalidData，普通收藏列表仍正常。回归以 3MiB 合法歌单响应复现相同错误，现恢复业务客户端与业务解码的既有 16MiB 上限；单曲／播放 music_client 和 music_checked 仍为 2MiB，统一 BusinessResponse 仍为 2MiB，不放宽前端／插件返回预算。不能只修改解码后大小检查，否则传输层会先拒绝。

回归覆盖真实本地 HTTP 读取、scoped 客户端、业务判定、单曲严格预算与超过 16MiB 的拒绝。只读真实账号探测未取得当前开发版登录会话，因此没有宣称实机页面已经恢复；用户重试页面时仍需核对实际结果。未新增缓存、协议实现或第三方依赖。

## 阶段四修改记录与注意事项（2026-10-10）

### 修改清单与接线范围

- `music/external.rs` 实现外部后端加载、描述符一致性检查、音乐 JSON ABI 转换及两类独立执行预算。`ExternalAdapter::load(path, expected)` 成功后，后端调用 `AdapterManager::register(adapter, sessionProvider)`；来源由宿主传入的预期描述符和 guest 自报描述符共同核对，不能按功能插件 manifest 自动注册。
- `plugins/music-fixture/backend/` 是独立 Cargo Component 项目，使用公开 WIT，不导入宿主 crate。来源 `fixture-radio`，必选单曲／资源解析以及可选 Search／Browse；没有账号能力。fixture 不随包安装，不发起网络请求，资源 URL 是测试数据，不能据此宣称已经完成真实第二平台接入。
- `infrastructure/wasm_runtime.rs` 提取共享 Component 执行器，开放 crate 内部的 HostHandler／from_component；`plugins/runtime.rs` 只绑定功能插件 Context，外部适配器使用全拒绝 Host，不引用插件运行模块。五个既有音乐探测 example 同步引入共享模块，避免独立编译依赖完整插件系统。三个管理器没有合并，没有向功能插件公开新适配器管理权限。共享加载路径改为最多读取 16MiB + 1 字节再拒绝，避免先整文件读取后检查。
- `pnpm --dir app plugins:fixtures` 同时构建原运行 fixture 与音乐 fixture；format／format:check 加入音乐 fixture。独立 Cargo.lock 提交，生成 bindings 和 target 沿用忽略规则。没有新增宿主依赖、缓存、IPC 或产品入口，本轮无用户可见 CHANGELOG 条目。

### 受限外部 ABI v1

Component transport 复用 `plugins/wit/plugin.wit` 的 `nons:plugin@1.0.0`：initialize、call、shutdown 的签名不变；音乐契约版本仍为 SourceDescriptor.contractVersion = 1，与功能插件版本协商分开。外部桥只调用下表方法：

| method                 | args                                 | 成功 data                                 |
| ---------------------- | ------------------------------------ | ----------------------------------------- |
| music.descriptor       | null                                 | SourceDescriptor，必须与宿主预期完全相同  |
| music.read-track       | 请求封套，request 为 EntityRef       | MusicTrack                                |
| music.business         | 请求封套，request 为 BusinessRequest | BusinessResponse，继续使用 type/data      |
| music.resolve-playback | 请求封套，request 为 ResolveRequest  | 后端 WireResource：metadata、url、headers |

请求封套为 `{ request, session: { source, account, generation }, adapterGeneration }`，不包含凭据、文件路径或宿主 deadline。会话不是 guest 可自行选择的权限；管理器构造并在执行期间及发布时核对它。成功封套为 `{ "status": "ok", "data": ... }`，业务失败为 `{ "status": "error", "data": { "code": "notFound", "retryAfterMs": null } }`。transport 字符串错误／trap／无效 JSON／超大返回只转为稳定 internal，不交付 guest 原始错误文本；合法 JSON 但 DTO 错误返回 invalidData。

TrackPage 的 JSON 是展平的 `{ items, nextCursor, total, description }`，没有额外 page 属性。外部 fixture 的真实反序列化回归发现并修正了此处形状误用。平台游标原样交给 guest，宿主对外包装查询／账号／实例／写代次作用域；测试确认首批 `广播:α/first`、continuation `continuation:β/page` 和最后一页 `广播:β/last`，不是每次返回同一页。

WireResource 仅在后端反序列化；域模型 PlaybackResource／HttpAccess 没有增加 Serialize，Debug 隐藏 URL 和请求头。返回后统一管理器照常拒绝过期、试听和当前音频管线不支持的请求头；域模型不会为了通过 fixture 静默丢弃头。临时 URL 不进入 nativeCall、插件 DTO、查询缓存或队列。

### 执行预算与生命周期

| 项目       | 阶段四实际限制                                                                                                                  |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 编译／读取 | 进程级最多 2 个阻塞任务；文件最多 16MiB，超载直接 rateLimited；许可在阻塞闭包持有                                               |
| 运行并发   | 每外部适配器查询 4、播放解析独立 2，均无额外等待队列；管理器原预算仍有效                                                        |
| Store      | 每次调用新建，共享已编译 Component；单线性内存最多 64MiB、最多 1 个 memory、16 instances、8 tables、每 table 10000 元素         |
| CPU        | 初始化和每次 call 分别 1000 万 fuel；每 10000 fuel 主动 yield，耗尽触发 trap                                                    |
| 时间       | 实例化／initialize／call 各最多 5 秒；统一单曲总期限 3 秒、业务／播放解析总期限 12 秒包含实例化和初始化，管理器每 10ms 复核失效 |
| JSON       | 复用 transport 的 64KiB 请求与响应上限，比业务契约的 2MiB 更严格；外部查询不能依靠增加每页条数越过上限                          |
| Host 能力  | 全部拒绝；没有 WASI、HTTP、文件、配置、storage、secrets、账号读取或其他来源访问                                                 |

请求取消、停用或账号／实例变化会丢弃整个执行 future 及其 Store，不将中断后的 Store 交给下一请求。guest trap 等只影响该请求的 Store，后续请求从干净实例恢复；来源注册和账号选择不会因此删除。descriptor 加载发生在注册前，平台调用阶段由 AdapterManager 统一期限约束；阻塞读取／编译不能被异步 future 强行终止，仍由上述编译许可控制数量，没有宣称硬实时编译期限。

外部桥暂不保存 guest 状态，initialize 每次调用执行，shutdown 不作为持久化或资源清理前提。需要长驻登录状态的适配器不能直接套用此生命周期；应设计显式后端 challenge／账号存储接口，不得把凭据或会话放入 guest 临时全局变量后依赖下次调用。新增 Host Capability 时必须绑定宿主 RequestContext，不能相信 guest 自报来源或账号。资源 URL 仍由既有宿主音频管线读取，后端执行不进入 streaming thread。

上述限制不是进程总内存测量：最多六个并行 Store 的线性内存理论上限合计 384MiB，另有编译器、代码、HTTP/TLS 和宿主分配。独立实例化换取取消／账号隔离，但增加调用成本；未测 Release 首声延迟与内存，不宣称这些数值适合所有平台。实际使用前应测量并调整，不能用此 fixture 的小数据代表真实平台性能。

### 回归与阶段五注意事项

真实外部 Component 回归覆盖非数字／Unicode ID、两页 token、缺少 Favorites 能力、账号代次使旧游标失效、跨来源响应拒绝、账号读取 Host 越权拒绝、过期／试听／带头资源拒绝、停用／恢复／重载、敏感 Debug、查询满载时播放保留容量、取消后许可释放、fuel／内存／trap／非法／超大响应与下一请求恢复、版本／身份不符、编译超载及过大／非法文件拒绝。原 TestAdapter 与 AccountManager 测试继续覆盖在途停用与账号／实例取消、缓存作用域、原始凭据越权拒绝、停用保留保存账号、删除墓碑与旧提交隔离；没有操作开发机真实账号。

阶段五拆网易云前必须补：独立包校验／安装／发现／管理入口；受控 HTTP 域名授权、响应上限、超时和播放保留预算；绑定 AccountAccess 的凭据读取／刷新和登录完成／退出 ABI；有界 challenge 与不同登录展示；长驻执行或明确无状态协议的成本测量。当前 loader 明确拒绝 Search／Browse 以外的能力声明，不能把尚未接线的账号、曲库、收藏等能力先写进描述符。复用功能插件 HTTP 基础设施时也不能赋予整个 Context 或其账号兼容能力。

历史 TrackSource、持久曲库／队列和播放器兼容 DTO 仍保留旧形状；本轮验证统一后端契约，未让任意外部来源在现有数字页面／混合队列 UI 中直接播放。后续接入须单独做原子数据迁移与通用播放入口，继续核对本地绑定、歌词关联、账号／缓存失效。真实外部音频、WebView 释放后播放、跨平台构建／基本播放和真实平台不同登录流程仍需对应环境验收。

Wasmtime StoreLimitsBuilder 与 fuel_async_yield_interval 经 Context7 核对官方 Rust API；memory_size 限制每个线性内存，配合 memories(1) 才得到上述每 Store 上限。实际宿主固定版本继续为 46.0.1，没有新增执行框架。验收结果见 [实现记录](implementation.md)。

## 阶段五修改记录与注意事项（2026-10-10）

### 修改清单与当前归属

- `packages/music-adapter-sdk/` 是无宿主依赖的后端契约 crate；迁入原有 account/adapter/business/identity/resource 定义、错误以及必要的历史 Track/Lyrics DTO 与纯曲目转换。宿主 `music/mod.rs` 重新导出相同类型，契约版本保持 1；播放器快照、队列事务与账号持久化仍留在宿主。
- `adapters/netease/` 是默认随包提供的独立原生适配器源码 crate，拥有 `NeteaseAdapter`、业务/账号转换、平台客户端、搜索/详情/曲库与固定补丁版 ncm-api-rs。`vendor/ncm-api-rs` 从宿主整体迁入包内，协议、许可证、补丁、读取上限及上游 commit 不变。宿主 Cargo 移除直接 ncm-api-rs、cookie、qrcode 依赖；cookie/qrcode 随平台包归属移动。没有新增第三方协议、HTTP 框架、账号存储或播放引擎。
- `music/bundled.rs` 只组装默认包、AdapterManager 与单来源账号桥；适配器不再持有具体 AccountManager。`ProviderAccounts` 绑定来源，读取前后检查会话/实例，账号请求结果仍经统一账号管理提交。系统凭据继续只在宿主保存；不是把 AccountManager、SQLite 或 keyring 源码复制进适配器。
- `music/service.rs` 只处理统一资源与本地播放；旧数字 DTO/offset 转换明确归入 `music/compatibility.rs`。宿主 `netease/mod.rs` 仅保留包 DTO/歌词客户端导入及旧凭据只读探测入口，不包含平台协议实现。独立歌词服务继续复用包内平台客户端，不因音频来源拆包改变歌词来源/缓存。
- PluginManager 与 SongService 移除 Netease 客户端依赖。`netease.account-credentials` 的敏感兼容能力仍需已有 `account:credentials` 授权，通过 AccountManager 的有界阻塞任务实时读取，返回前核对账号和适配器实例；不缓存，不作为通用音乐接口前提。
- `music/settings.rs` 与 `features/settings/pages/adapters.tsx` 接入独立管理入口，设置导航使用 `lucide-react` 的 Blocks；复用 SettingsCard、Button、Switch、设置弹窗滚动 shell 和主题 tokens，显示能力与启停状态，提供刷新及重载。没有混入功能插件的注册表、授权、配置或贡献管理。
- 五个在线只读 example 更新包/契约入口，移除无关宿主模块包含；format/format:check 同时覆盖两个新 crate。适配器独立 Cargo.lock 沿用宿主已锁定版本，没有顺带升级依赖。两个 README 记录构建、职责与执行方式。

### 原生源码包与外部 ABI 的区别

当前默认网易云包是 Rust 原生库，随应用编译和链接。独立源码/依赖/测试可以单独维护，稳定业务接口仍由 AdapterManager 调度；它不是可拖入安装的 WASM/ZIP，不提供运行时原生库替换或卸载。升级包需重新构建应用。运行时可安装的第三方平台包、其受控 HTTP/凭据与登录 ABI 仍未开放；阶段四 fixture 的全拒绝 Host 和 Search/Browse 限制不变。

选择此方式保留现有成熟 SDK、共享连接池和登录协议，避免以受限测试桥冒充完整平台运行时。原生代码必须受信任；没有 Wasmtime 的线性内存/fuel 沙箱，不宣称凭据无法被恶意原生代码复制。阶段四“拆包前须补”的外部能力门槛适用于迁入 WASM 执行器的路线；本阶段没有越过这些门槛向第三方开放权限。

### 设置、缓存与生命周期

| IPC                   | 参数               | 返回/行为                            |
| --------------------- | ------------------ | ------------------------------------ |
| `adapter_list`        | 无                 | `{descriptor,enabled}[]`，含停用来源 |
| `adapter_set_enabled` | `{source,enabled}` | 保存设备偏好后推进启停代次           |
| `adapter_reload`      | `{source}`         | 仅启用来源可重载，推进实例代次       |

以上管理调用通过既有 nativeCall 执行，不加入查询缓存。`musicAdapterEnabledV1:<source>` 存在宿主 SQLite settings，默认启用，启动组装之后恢复；未知来源拒绝，不产生无效设置，数据库保存失败不改变运行启停。设置变更串行，涉及磁盘的操作进入阻塞任务；关闭弹窗后丢弃旧 UI 响应，订阅清理后不会回写。

成功启停/重载发出 `adapters-changed` 和 `music-changed`；当前窗口同时显式清理 nativeCall 查询缓存。缓存命中前后仍核对 music_context，插件 Rust 歌曲缓存实时核对实例，旧代次请求与预加载拒绝发布。状态通知不等于删除账号或队列；停用保留保存账号、选择、混合队列和本地歌词绑定，恢复可重新解析。已交付核心的独立 URL 可播放到结束，下一次解析若来源停用会报不可用。

原生重载只更新管理器实例代次，并非重新 dlopen/释放原生库；共享 HTTP 连接池继续复用。查询/播放凭据预算 4/2、账号阻塞任务预算 4、业务/账号/响应上限沿用前文；这里只记录设计与回归，不把数量限制当作 Release 延迟或进程内存实测。管理操作不会清除歌词磁盘缓存、原始本地文件或平台保存账号。

### 验证与维护注意事项

回归覆盖设备启停重启恢复、停用拒绝重载、未知来源拒绝、写盘失败保留运行状态、账号桥跨来源/旧实例/停用拒绝、恢复凭据及保存记录保留；DOM 回归覆盖保存完成前不改变开关、失败提示、重复操作禁用、缓存失效、过期列表响应与卸载后响应丢弃。独立网易云包保留全部 22 项平台转换与有界大歌单响应回归；统一宿主契约、WASM fixture 和混合队列回归继续执行。

后续新增原生包必须由宿主显式组装并绑定专用 ProviderAccounts，不能复用网易云的原始凭据桥；新增通用业务不得导入平台客户端或扩展旧数字 DTO。迁入外部 WASM 时先实现/验证网络、账号与包权限，不将 native crate 的完整能力直接写入现有受限外部描述符。历史数据库原子迁移、真实第二平台、不同登录展示、跨平台编译/基本播放、真实登录/远端写入/WebView 回收后播放和 Release 性能仍需对应实现与环境验收。实际命令与结果见 [实现记录](implementation.md#统一音乐适配器adr-0006-阶段五2026-10-10)。
