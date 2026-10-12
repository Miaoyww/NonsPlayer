# go-music-api 补充 QQ 适配器研判

日期：2026-10-12。结论：**可以补充 QQ 个人歌单和喜欢歌曲的读取协议，但不能直接补齐完整 QQ 适配器。建议先作为协议参考与验证对象，暂不把整个 Go HTTP 服务随包接入。** 搜索、播放等已有能力继续优先采用原生 Rust 客户端；本次只做源码研判，没有启动服务、登录、请求真实音乐资源或验证平台接口可用性。

## 审查版本与范围

- `go-music-api` 已克隆至 `C:/Projects/NonsPlayersProject/go-music-api`，固定提交 `bacdfbe6cf6a5ba7331463d2039e3aac915c627f`。
- 该仓是 HTTP 包装，QQ 实现在依赖 `music-lib`；`go.mod` 固定 `v1.1.1-0.20260814153305-59dd7753bbc8`。为避免只看路由误判，另克隆 `C:/Projects/NonsPlayersProject/music-lib` 并检出依赖提交 `59dd7753bbc8ddba6cb5c859ee93b4d98401e833`。[依赖声明](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/go.mod)。
- 两个 Go 仓的文件清单均未发现 `AGENTS.md`。按项目要求先查询 Context7 `guohuiyuan/go-music-api`，返回的是其他 Node/Python QQ API，没有该库匹配索引；改用上游源码，不把其他项目文档当作本库依据。
- 对照的 Rust 库为 `AstronW/netease-qq-music-api` 提交 `11d8c5c0e23fb74169292516592b5efe9ac59540`，详见 [Rust 客户端研判](netease-qq-music-api.md)。Nons 职责与凭证边界以 [ADR 0006](../adr/0006-music-provider-adapters.md) 和 [ADR 0007](../adr/0007-current-and-supplementary-sources.md) 为准。

## QQ 实际覆盖

下表的“存在”表示看到了实际请求与响应映射，不代表线上验证通过。仓库多平台能力不能推定为 QQ 的能力。

| 能力                                   | Go API / music-lib 实现                                                                  | 对 Rust 库缺口的价值                     |
| -------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------- |
| 登录用户资料、头像、昵称、稳定账号身份 | 无独立用户资料 API；歌单条目含创建者字段，登录返回 Cookie                                | 不能补完整 Profile                       |
| 本人创建歌单                           | `GetUserPlaylists(page, limit)` 请求 `fcg_user_created_diss`                             | 可补读取协议                             |
| 本人收藏歌单                           | 同一方法附加 `fcg_get_profile_order_asset.fcg`，`reqtype=3`，标记 `extra.collected=true` | 可补读取协议，但错误静默忽略             |
| 我喜欢的歌曲                           | 虚拟歌单 `profile:favorites`，同一 asset 接口 `reqtype=1`                                | 可补读取；详情只取第 1 页 300 首         |
| 私人目录歌单歌曲                       | `profile:dir:<dirid>` 请求 `fcg_musiclist_getinfo.fcg`                                   | 可参考，固定 `from=0,to=500`，缺完整续载 |
| 喜欢 / 取消喜欢、收藏 / 取消收藏歌单   | 未发现实现或路由                                                                         | 不能补写操作                             |
| 歌单创建、编辑、删除、增删歌曲         | 未发现实现或路由                                                                         | 不能补 `PlaylistWrite`                   |
| 歌曲搜索 / 详情 / URL / LRC            | 均有实际请求与映射                                                                       | 与 Rust 大部分重复，不建议因此增加服务   |
| 专辑搜索 / 专辑歌曲                    | 有；HTTP 详情只返回歌曲数组，底层 ParseAlbum 可返回专辑摘要                              | 大部分重复，模型仍需补齐                 |
| 艺术家搜索、详情、歌曲 / 专辑列表      | QQ 包和 HTTP 路由均未发现                                                                | 不能补艺术家体验                         |
| 推荐、分类标签与分类歌单               | 有读取实现                                                                               | 分类能力可作补充参考                     |

主要证据：[QQ 个人歌单实现](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/user_playlist.go)、[歌单分派与分类](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/playlist.go)、[专辑实现](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/album.go)、[歌曲及详情](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/qq.go)、[完整 HTTP 路由](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/router/router.go)。

个人歌单页会重复尝试加入“我喜欢”，并混合创建和收藏列表，返回统一数组而无总数、独立游标和分组状态。喜欢摘要失败或收藏查询失败被忽略，不能把缺条目解释为用户确实没有该资产；创建列表请求失败却会使整个方法失败。接 Nons 前需拆分状态和游标，避免空结果掩盖错误。[GetUserPlaylists](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/user_playlist.go#L21)。

## 原生 Rust 登录凭证能否复用

Rust `TencentLoginToken::to_cookie()` 输出 `uin=music_id; qqmusic_key=music_key; qm_keyst=music_key; tmeLoginType=login_type`，可直接传入 Go 的 `qq.New(cookie)`，因此**格式可以连接，语义互通仍未验证**。[Rust 凭证类型](https://github.com/AstronW/netease-qq-music-api/blob/11d8c5c0e23fb74169292516592b5efe9ac59540/src/models.rs#L437)。

Go 的 `normalizeQQUIN` 优先使用 `wxuin`，再使用 `uin`、网页登录别名、`musicid` 等，并把该值送到 `hostuin` / `loginUin` / `userid`。它没有显式区分 QQ 号码与 QQ 音乐内部 MusicUin。其微信登录自己合成 `musicid`、`musickey`、`qqmusic_key`、`qm_keyst`，再归一化为 `uin`；这证明作者有处理多种 Cookie 的意图，不能证明 Rust 原生登录返回的 MusicUin 在每个旧网页资产接口都能工作。需要用同一原生登录凭证验证本人创建、收藏、我喜欢三组接口，以及 QQ / 微信账号身份，必要时补资料查询或身份映射。[Cookie 身份选择](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/user_playlist.go#L623)、[微信登录与 Cookie 映射](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/login.go#L392)。

Nons 应继续让 QQ 适配器管理平台登录和刷新，账号管理器保存不透明 token。不得另存一份 Go `cookies.json` 作为账号真相；不能因使用 Go 的登录路由而建立第二套账号管理器。Go 原实现是一平台一个全局 Cookie 字符串，没有 Nons 的账号 / 会话代次隔离。[CookieManager](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/service/factory.go)。

## 不宜直接采用的行为

- QQ 歌曲搜索固定第 1 页 10 首；无会员时直接过滤 `payplay=1` 曲目，会员探测错误也按非会员处理。这会把“能搜到但当前无权播放”变成“没搜到”，与显式补充搜索目标冲突。应保留结果，独立展示授权 / 播放限制。[Search](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/song.go#L20)、[会员探测](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/account.go)。
- 聚合搜索并发请求后 `wg.Wait()`，所有来源完成才一次返回；吞掉各源错误，并按完成顺序拼数组。没有分源进度 / 失败状态、逐源游标、查询取消或稳定排序，不适合直接成为 Nons 搜索调度器。[UnifiedSearch](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/handler/music.go#L310)。
- 统一 Song 把歌手拼成字符串，专辑用 Playlist 表达，缺结构化艺术家引用、试听期限等字段；播放 URL 自动按高到低选择，返回值仅 URL，不保留实际音质与有效期。目录 / 普通歌单的部分歌曲映射还丢失数字 songid。[模型](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/model/song.go)、[URL 解析](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/download.go)。
- 通用 HTTP 使用 2 分钟超时、无 context 取消、无响应大小上限；部分登录使用默认客户端或未设置 Timeout。部分 QQ 查询是明文 HTTP，并附带账号 Cookie。若用于 Nons，必须按宿主预算改造期限、响应上限、并发与会话隔离，核验端点 HTTPS 支持。[HTTP 实现](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/utils/request.go)、[登录请求](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/qq/login.go)。
- 服务默认 `:8080`，路由没有身份校验，且公开 Cookie 读写接口。若采用 sidecar，需私有绑定、随机端口 / 鉴权、移除公开 Cookie 路由和明文文件持久化。[入口](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/main.go)、[路由](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/router/router.go)。

## 集成路径比较

| 路径                    | 优点                                                       | 新增成本与建议                                                                                                                                       |
| ----------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust 调 Go sidecar HTTP | 保留上游实现，便于独立更新                                 | 新增三平台二进制、进程与端口生命周期、凭证 IPC、故障恢复；还需改安全边界、分页 / 取消。只补几个读接口时不划算，暂不推荐随包                          |
| Go C ABI / FFI          | 不必提供 HTTP 服务                                         | 上游未提供 C ABI；需自行包装、跨语言内存与错误 / 取消协定、Go runtime 和交叉编译维护。解决不了缺失业务能力，当前不推荐                               |
| QQ 原生组件补协议       | 复用 Rust 请求预算、统一账号 / 缓存 / 错误模型，不新增服务 | 将缺失的个人资产读取协议实现到 QQ 适配器或上游 Rust 客户端，并保留适用许可；写操作、资料和艺术家仍须另找可靠依据。符合当前原生架构，优先验证这一方向 |

这些是设计成本判断，未测量新增进程内存、请求延迟或二进制体积，不把预期当性能结果。无需替换已有网易云客户端，也不改变独立播放核心或歌词服务。

两个 Go 仓根 LICENSE 均为 AGPLv3；Nons 根 LICENSE 为 GPLv3。HTTP、FFI、代码移植对许可义务的影响需按实际分发 / 组合方式核对，不能把 HTTP 隔离视为自动免除 AGPL 义务；本文不作组合许可证的法律判定。[Go API LICENSE](https://github.com/guohuiyuan/go-music-api/blob/bacdfbe6cf6a5ba7331463d2039e3aac915c627f/LICENSE)、[music-lib LICENSE](https://github.com/guohuiyuan/music-lib/blob/59dd7753bbc8ddba6cb5c859ee93b4d98401e833/LICENSE)。

## 下一步验证边界

优先验证同一 Rust 原生登录 token 能否读取 QQ 用户资料、本人创建 / 收藏歌单和喜欢歌曲，完成全量分页，并区分空列表、凭证失效和平台失败。其后补 QQ 的喜欢 / 收藏写入、歌单写入和艺术家专辑列表；这些不能以本库覆盖推定完成。第三个音源服务尚未选定，本库的跨平台搜索和换源也不自动等于第三个解析器：其返回的歌曲来源与解析实现必须继续分别声明，匹配替代播放保留显式决策。
