# netease-qq-music-api 接入研判

日期：2026-10-12。结论：**可用，但需要补齐；适合作为 QQ 音乐原生适配器的平台客户端基础，不能直接称为完整 QQ 音乐适配器。** 本文记录源码与本地编译验证，不代表平台接口、二维码登录或会员资源已经实测可用。

## 审查版本与依据

- 仓库克隆至 `C:/Projects/NonsPlayersProject/netease-qq-music-api`；固定审查提交 `11d8c5c0e23fb74169292516592b5efe9ac59540`，提交日期 2026-04-28，标题 `docs(readme): avoid printing login token`。这是本次克隆取得的 HEAD，不保证此后仍为最新版本。
- 按项目约定先使用 Context7 `resolve-library-id` 查询 `AstronW/netease-qq-music-api`。返回的候选是 Node.js 与 Python QQ API 项目，没有该 Rust 库的匹配索引，因此未用不相关库的文档替代；以下直接核对该仓 README、manifest 和源码。
- 该仓 [AGENTS.md](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/AGENTS.md) 已阅读。本次未修改其实现。
- Nons 接入边界以 [ADR 0006](../adr/0006-music-provider-adapters.md)、[ADR 0007](../adr/0007-current-and-supplementary-sources.md)、`packages/music-adapter-sdk` 和 `adapters/netease` 为准。

## 包、许可与运行方式

这是实际的 Rust library，包名 `netease-qq-music-api`，crate 名 `netease_qq_music_api`，版本 `0.1.0`、edition 2024、声明 Rust 1.85。公开 `MusicClient` 提供链式异步请求；内部直接实现 QQ HTTPS 请求、签名与 MQTT over WebSocket 登录，无需另部署 Node/Python API 服务。[Cargo.toml](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/Cargo.toml)、[客户端入口](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/client.rs)、[QQ 请求实现](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/api.rs)。

`Cargo.toml` 声明 MIT；[LICENSE](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/LICENSE) 为 MIT License、Copyright (c) 2026 AstronW，要求在副本或主要部分保留版权与许可通知。这里确认的是软件许可；仓库 README 明确说明其不授予平台内容、账号、数据或版权使用权。[README](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/README.md)。

依赖包含 reqwest 0.13、Tokio、async-tungstenite 0.29、rumqttc 0.25、image 0.25、time 0.3.47，以及 AES、ECB、摘要、二维码与序列化相关库。没有 QQ-only Cargo feature：`MusicClient::new()` 同时建立网易云与 QQ 客户端。这会引入已有网易云实现以外的重复客户端及依赖；若采用，优先请求上游增加平台 feature 或通过保留许可的有范围补丁裁剪，而不将宿主改为依赖此库的跨平台 DTO。

声明 MSRV 1.85 并不等于锁定依赖可在 1.85 编译。`cargo metadata --locked` 显示本次 lock 的 image 0.25.10、time 0.3.47/time-core/time-macros 均声明 Rust 1.88；Nons 当前适配器 Rust 1.95 可以覆盖这项要求。本次只验证 Rust 1.95.0 的 Windows 原生构建，未验证最低版本或 macOS/Linux。

## 实际 QQ 能力与 Nons 契约覆盖

下表中的“有”表示存在请求、响应解析与映射实现，而不是仅有 README 或空壳方法；本次检查未发现 QQ 接口以 `todo!`/`unimplemented!` 代替实现。缺失能力不能因为库同时支持网易云而推定 QQ 支持。

| Nons 能力或操作                               | 该库 QQ 实现                                                             | 接入判断                                                 |
| --------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------- |
| `read_track`                                  | `detail().song()`，QQ 使用字符串 songmid；歌曲包含名称、歌手、专辑及封面 | 可映射，需补时长、别名与权限数据                         |
| `resolve_playback`                            | `playback().url()` 调用 GetVkey，按候选音质取非空 purl                   | 可作为基础，必须补实际资源限制语义                       |
| Search：歌曲、歌单、专辑、艺术家、建议        | 均有 QQ 请求实现，搜索使用 offset/limit 转页码，结果含 more              | 可映射宿主不透明游标；固定分页大小与去重                 |
| Browse：详情、歌单/专辑曲目、艺术家歌曲       | 有；艺术家歌曲分页，专辑请求固定最多 1000 首，歌单详情一次返回           | 不能宣称所有列表已有真正的续载；补分页与上限             |
| Browse：艺术家专辑列表、曲目附加资料          | 未发现对应 API                                                           | 需要补齐；非必选操作明确 Unsupported                     |
| Account：二维码登录、状态、刷新               | 有二维码创建、MQTT 事件、凭证交换与刷新                                  | 有基础；需会话所有权、账号资料、取消与持久化接线         |
| Account：Profile / 平台退出                   | 未发现对应 API                                                           | Profile 必须补；本地退出仍由 AccountManager 处理         |
| UserLibrary：摘要、用户歌单/专辑/艺术家、历史 | 未发现对应 API                                                           | 完整账号体验的主要缺口                                   |
| Favorites：喜欢与收藏读取/写入                | 未发现对应 API                                                           | 完整账号体验的主要缺口                                   |
| PlaylistWrite：创建/改名/删除/增删歌曲        | 未发现对应 API；`playlist()` 是详情、分类和广场列表                      | 不能把“有歌单模块”理解为可维护个人歌单                   |
| Recommendations                               | 推荐歌单、热词、榜单、歌单分类和广场列表                                 | 部分覆盖；推荐歌曲等另补                                 |
| PrivateFm                                     | 未发现对应 API                                                           | 不作为首阶段完整 QQ 必选能力                             |
| Lyrics                                        | LRC 原文与译文，base64 解码与时间戳归一化                                | 沿用宿主统一歌词服务；不替换现有 QQ QRC 与 AMLL 优先策略 |

覆盖依据：[QQ API 全部实现](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/api.rs)、[QQ 响应与映射](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/models.rs)、[公开数据模型](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/models.rs)。Nons 对应请求见 `packages/music-adapter-sdk/src/business.rs`，基础契约见 `adapter.rs`。

## 接入前必须处理的差异

### 播放可靠性与资源语义

QQ `get_url` 自动加入较低音质候选，最后还有 `RS02` 候选，不接收 Nons 的 `allow_downgrade`。Hi-Res 与 Lossless 当前请求同一 `F000` 文件前缀，不能按请求名宣称获得 Hi-Res。模型按文件名前缀返回音质等级，可以用于映射，但不包含试听区间、到期时间、资源大小或媒体参数；空 URL 也可能以成功的 `UrlResult` 返回，且模型拼接固定 QQ CDN 地址而未使用响应的 CDN 列表。接线须检查空 URL、身份、实际音质与播放权限；不能把所有非空 URL 直接标为 `PlaybackExtent::Full`。需保留足够原始权限/试听数据，按宿主允许降级设置选择候选，未知到期沿用宿主的短期有效租约。[播放解析](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/api.rs#L404)、[URL 映射](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/models.rs#L514)。

`Song` 与 QQ `TSong` 均未保留时长、原始数字 songid、别名、会员/地区/试听标记；Nons 播放时间线和跨来源候选匹配不能长期用零时长代替真实数据。songmid、数字 songid、媒体 mid 各自保持语义；播放文件名不能未经真实平台验证就假定始终以 songmid 生成。[曲目模型](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/models.rs#L68)。

### 网络与错误预算

QQ HTTPS 客户端已有 15 秒总超时和连接超时，但内部固定 `.no_proxy()`，公开入口没有注入 HTTP client 或配置超时/代理的接口。响应直接 `.json::<T>()`，没有应用级解压后字节上限；结果转换完成后才做 DTO 限制，无法防止解析阶段的内存放大。需要在 SDK 内增加有界读取，最好提供可配置客户端/transport，而不是只在 Nons 外层加 timeout。

QQ 主要响应模型未保留通用平台错误码；鉴权失效、限流、资源不存在等可能变成反序列化/网络错误或空结果。Nons 需要明确 ErrorCode 映射；库原始错误不直接进入用户日志或 UI。宿主 RequestContext 继续执行期限、取消、实例和账号代次检查。[HTTP 实现](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/api.rs#L20)、[错误类型](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/error.rs)。

MQTT 已有连接 5 秒、订阅 5 秒、事件等待 1.5 秒、单次调用预算 6 秒和最多 3 次重定向；未发现用户设定的较小帧/消息/包大小预算，`Packet::read` 的 max_size 参数为 None。需要有界登录会话表、显式 WebSocket/MQTT 数据上限及取消后销毁会话；不能称现实现已满足宿主全部资源预算。[MQTT 实现](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/platform/tencent/mqtt.rs)。

### 账号管理器与会话生命周期

QQ token 已支持 Serde 序列化/反序列化，含 music_id、music_key、refresh_token、refresh_key、login_type 与有效期，可作为 AccountManager 保存的不透明凭据，适配器自行解析与刷新。token 派生 Debug 含敏感字段，不得用于日志；QQ账号资料与昵称/头像没有现成 API，需补齐才能完成 AccountRecord。

公开 `LoginSession<'a>` 借用 `&MusicClient` 并持有 MQTT 状态，二维码 key 私有。Nons `BeginLogin` 与 `PollLogin` 是分离请求，不能简单把借用会话存回同一个持有 client 的适配器而形成自引用，也不能泄漏 client 为 static。建议补上游 API，令会话拥有共享 client（例如 Arc/clone），公开不敏感会话句柄，由 QQ 适配器有界保存并在超时/停用/重载时销毁；成功交出 credential，账号管理器完成持久化和选择。[登录 API](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/client/login.rs)、[凭证模型](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/models.rs#L437)。

## 推荐落地方式与验收门槛

新建与网易云平行的 `adapters/qqmusic` 原生源码包，通过共享 `MusicAdapter` 和单来源 `ProviderAccounts` 接入；固定 `qqmusic` 来源标识及 songmid 实体 ID，平台客户端只作为适配器内部依赖。保留现有网易云 SDK，不为接入 QQ 顺带替换已验证的网易云实现。本方案不需要外置 API 服务，但原生库随宿主构建更新，不是可运行时安装的 WASM 包。

分两步：先补安全/资源语义和登录所有权，完成搜索、详情、账号、播放闭环；再补 QQ 用户资料、个人曲库、收藏与歌单写操作，才达到用户要求的完整适配器。平台没有的可选能力显式不可用，不用另一平台数据伪造。

必要验收包括匿名及已登录搜索、四种实体 ID、分页/大歌单、二维码过期与取消、凭证刷新/失效/多账号选择、会员与非会员、无资源/试听/地区限制、允许及禁止音质降级、定位与无缝预加载、资源过期重取、超大响应和 MQTT 帧、停用/重载及账号切换旧结果、Windows/macOS/Linux 编译与实机播放。平台真实请求测试与现有静态测试分开报告。

只提供音源的部分能力适配器走独立解析能力契约，不能拿这个 QQ SDK 的“能取 URL”省略 Nons 歌曲来源与解析器身份。聚合搜索结果显式标注曲目所属平台；若播放交给其他解析器，另外说明资源解析提供方。

## 本次验证

- Windows 原生环境，`rustc 1.95.0 (59807616e 2026-04-14)`。
- `cargo metadata --format-version 1 --locked` 通过；临时元数据文件已清理。
- `cargo check --locked --lib` 通过。
- `cargo test --locked --lib` 通过：36 passed、0 failed、36 ignored；没有启用 ignored 网络/凭证测试，执行的是参数、签名、歌词归一化与 MQTT 事件等离线单元测试。
- 没有执行真实二维码登录、平台歌曲解析或音频请求，没有测量内存、响应速度、接口成功率或会员音质。编译与单元测试通过不能证明平台协议当前稳定可用。
