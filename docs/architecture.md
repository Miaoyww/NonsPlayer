# NonsPlayer 框架讨论

本文记录已接受的项目约束。当前实现、运行命令与验收范围见 [实现记录](implementation.md)。

## 已明确的约束

- 最终目标：多流媒体平台并行的综合性跨平台播放器，并支持本地音乐。
- 平台策略：首阶段 Windows 优先，最终先覆盖 Windows、macOS、Linux 三个桌面平台。开发期间持续验证 macOS/Linux 的编译与基本播放；Windows 首先完成完整体验及 SMTC 验收。移动端不纳入当前范围。
- 产品只关注音乐；评论、视频、播客、社交、云盘业务均不实现。
- 已启动多流媒体来源框架化，遵循 [统一音乐适配器](adr/0006-music-provider-adapters.md)。网易云已拆为默认随包的独立原生适配器源码包，通过统一接口接入，由适配器设置管理；原生包随应用编译链接，第三方平台的运行时安装仍待后续实施。
- 性能冲突优先级：播放可靠性、响应速度、内存控制、视觉效果。缓冲和缓存设上限，隐藏歌词页面时停止其渲染；各场景具体性能预算仍待确定。
- 复用现有界面前，先检查能否提取为共享组件；适合时先组件化，再让原入口与新入口使用同一组件。共同的行为和外观由组件维护，场景差异通过少量明确的 props 表达。
- WebUI 优先复用组件，按 ShadcnUI → AnimateUI → MagicUI 的顺序查找适用组件；复用需同时符合交互和性能要求。
- 分段音乐列表统一在滚动到末尾时自动加载后续内容，不显示页码、条目范围或上一页/下一页底栏；续载保留已有内容，失败在列表末尾重试。长歌曲列表采用虚拟渲染，切换查询或筛选后丢弃旧请求结果。
- 优先采用现有库，尤其 Rust 库。必要的重复实现须先告知用户。
- 歌词显示指定使用 [AMLL](https://github.com/amll-dev/applemusic-like-lyrics)。
- 在线歌词依次从 [AMLL TTML DB](https://github.com/amll-dev/amll-ttml-db)、QQ 音乐、网易云获取；未命中、超时、错误、损坏及解析失败继续下一来源。AMLL DB 和 QQ 音乐默认开启，可在设置 → 歌词 → 歌词来源独立关闭；都关闭时仅查询网易云。歌词不阻塞音频。初始 DB 查询总预算 1 秒；官方文件地址与可配置镜像按顺序尝试，最多 3 个，不并发放大请求。
- QQ 音乐按歌名和歌手搜索，包含搜索结果的分组曲目；搜索协议来自 Lyricify-Lyrics-Helper。评分移植 AF-Media-Bar 的 LyricsMetadataScore：标题/歌手/专辑/时长权重 40%/40%/10%/10%，使用 F23 Jaro–Winkler 与偶数舍入，时差 ≤1 秒满分、≥10 秒零分，中间线性衰减。歌手在请求和候选两侧按用户配置的字面量分隔符拆分，最长分隔符优先、忽略大小写并去重；网易云结构化歌手沿用 `/`。完整移植 AF-Media-Bar 的搜索与采纳策略：QQ 达到 80 分直接采纳，低分候选仍取词并参与比较；网易云搜索候选最低 60 分，按真实元数据得分比较，同分优先 QQ。已知网易云播放/绑定 ID 的取词结果在备用阶段优先；AMLL DB 仍保留现有优先顺序。在线查询总预算 12 秒，备用网易云阶段最多 6 秒。QQ 阶段总预算 6 秒、尝试完整去重搜索词列表、每页 20 首；达标曲目优先按数字 ID 下载 QRC，QRC 下载/解码子阶段预算 900 毫秒，失败时按 songmid 查询旧接口 LRC。QQ QRC 解密复用 lyrics-crypto 0.5.0 的算法并增加有界解压，解密在阻塞工作线程执行；响应、单字段解压与合并正文均限制为 2MiB。WebUI 使用已有 `@applemusic-like-lyrics/lyric` 的 `parseQrc` 保留逐词起止时间；前端解析失败也先重试 QQ LRC，再回退网易云。QRC 的独立 LRC/QRC 翻译与发音按行起始时间合并。
- 所有歌曲仅支持在线取词，不读取本地同名歌词文件。本地歌曲沿用 AMLL DB → QQ 音乐 → 网易云：AMLL 需可靠关联或已有绑定 ID；QQ 和网易云都可按元数据搜索并评分，无关联 ID 也可查询。
- 歌词格式解析统一复用 AMLL 的 `@applemusic-like-lyrics/lyric`，`core` 负责渲染。独立译文和音译使用对应的 LRC/QRC/YRC 解析器，优先保留内嵌辅助文本，否则匹配起始时间相差不超过 500 毫秒的最近非空辅助行；超出容差留空，不沿用上一句。此匹配规则参考 AF-Media-Bar 的 `LyricsTextParser`。
- 新鲜缓存立即显示，过期命中立即显示并后台更新。确认缺失约 24 小时后复查，网络错误不能记为永久缺失；缓存有容量上限并支持手动刷新。
- 播放核心首阶段包含 Windows SMTC，完整同步封面、Title、Artist、Album、Position、StartTime、EndTime。
- 音频需要无缝播放、多格式与输出设备选择，支持本地音乐；GStreamer 是优先验证的候选，尚未确定采用。
- 接受随安装包携带经裁剪的必要音频原生依赖，先验证 GStreamer 能力与分发成本再决定是否采用。
- 首阶段无缝验收覆盖本地和网易云、同采样率同声道的连续曲目，包含无损和带有效延迟信息的有损文件；检查输出边界是否引入额外空隙或重复样本。跨参数衔接单列验证，网络断供及元数据缺失时明确能力边界；淡入淡出作为独立功能。
- 播放设置提供播放/暂停渐变与无缝播放独立开关，默认分别关闭和开启，沿用 `playbackOptions.fades` / `gapless` 保存。播放/继续渐入与暂停渐出时长 200 毫秒，复用 GStreamer Controller 线性插值和播放缓冲后的 `audioamplify` 输出节点，以 running time 跨歌曲时间线执行；用户音量独立由 StreamVolume 控制。输出 PCM 按单调时钟限制预处理提前量为 30 毫秒，保留预滚；暂停在渐出结束并保留 80 毫秒设备排空余量后执行，快速继续从当前增益反转，重复暂停不延长等待。停止、清空、手动切歌不等待淡出；移除旧 800 毫秒曲目首尾包络，自动切歌不淡出或重新渐入。无缝播放关闭后跳过预加载与 `about-to-finish` 交接，在 EOS 后切换；Crossfade 与整曲响度均衡后续独立实施。
- 维护 CHANGELOG，为发布 Release Notes 提供依据。

## 页面 shell 与滚动

- 新增工作区页面统一使用 `app/src/components/music/music-page.tsx` 的 `MusicPage` 作为 shell，复用页面间距、滚动区域和播放栏避让；标题优先使用同文件的 `MusicPageHeader`。
- 宿主页面在页面根使用 `MusicPage`；插件页面由 `MusicWorkspace` 的插件入口统一包裹，插件只渲染内容，沿用宿主提供的标题样式。设置弹窗内嵌页面使用弹窗布局。
- 页面内容沿 shell 的滚动区域自然展开，额外滚动区域仅用于需要独立滚动的控件。验收时用超出视口的内容确认末尾可达，并检查播放栏显示时底部操作仍可见。
- 新增 ContextMenu 操作项时提供表达动作的图标，装饰图标设置 `aria-hidden`；没有专用图标时使用 `LayoutGrid`。插件菜单通过 manifest 的 `icon` 声明图标，宿主为省略或未知标识提供 `LayoutGrid` 回退。

## 已确认的首阶段功能范围

- 网易云包含常用歌单创建、编辑、收藏及喜欢歌曲等写操作；先提供有上限的播放缓存，下载管理与完整离线播放后置。
- 本地音乐支持文件打开、目录管理与变化同步、持久化曲库、元数据读取、搜索，以及与网易云歌曲交替播放的混合队列。标签编辑后置。缓存和目录同步规则见 [缓存与本地音乐](cache-and-local.md)。
- 音频输出首阶段覆盖共享模式、设备选择和设备断开后的恢复；独占模式与 bit-perfect 后置，播放核心保留明确输出配置边界。
- 登录首阶段采用二维码登录、会话持久化和失效恢复。
- 音质按用户偏好请求；不可用时根据设置允许降级，并显示实际播放音质。
- 性能基础目标为 Windows 10/11 x64、8GB 内存、集成显卡、1080p。分别测量冷启动、缓存命中后的首声延迟、后台播放与歌词页面占用；内存统计包含应用所属 WebView 进程。具体预算在最小播放验证后确定，再作为开发验收门槛。

## 已确认的框架边界

- 所有在线来源通过统一音乐适配器契约接入，内置网易云与外部适配器使用相同业务接口。本地与在线共用独立播放核心，职责、契约与迁移见 [ADR 0006](adr/0006-music-provider-adapters.md)。
- 音乐适配器属于插件，由独立适配器管理器管理；功能插件沿用现有插件管理器。账号获取、登录与刷新由适配器实现，账号管理器统一保存不透明凭证，职责及安全边界见 ADR 0006。
- 多来源采用“当前来源／补充来源”表达使用角色，适配器保持平行独立，能力范围与角色分开；宿主协调补充搜索，曲目保留真实来源。命名、职责与后续实现边界见 [ADR 0007](adr/0007-current-and-supplementary-sources.md)。
- 明确登录或选择平台账号成功后切换当前来源，账号恢复与刷新不触发切换；聚合搜索逐项显示来源并隔离各来源失败，曲目平台与播放音源分别表达。两完整平台适配器加单音源的目标、暂缓本地聚合歌单及实现边界见 [ADR 0008](adr/0008-account-driven-source-and-search.md)。
- GStreamer 位置和状态为权威信息；系统媒体控制、WebUI 与歌词消费同一播放状态，各入口控制进入同一有界播放命令队列。元数据/队列更新与轻量位置更新分离。
- 插件 SDK 与宿主能力的归属遵循 [插件职责边界](adr/0005-plugin-capability-boundary.md)。

## 已接受的实现方向

- 网易云平台 SDK 归属网易云适配器实现，复用现有 Rust 客户端，避免额外部署 API 服务；宿主通用业务通过统一适配器接口访问，随包分发不赋予专用调用路径。
- 音频引擎提供播放位置和状态的权威信息；WebUI、歌词和系统媒体控制消费同一播放状态。播放控制从各入口进入同一流程。
- 在全面实现前验证 GStreamer 的无缝能力、格式、设备切换及分发成本。

## 当前待决定

1. Windows 10 最低版本，macOS/Linux 后续具体支持范围。
2. 歌词来源的实际命中率、超时预算与网络环境实测。
3. 具体性能预算、测试设备与测量方法。
4. GStreamer 能力和分发成本的验证结果及正式选型。
5. 跨采样率、位深、声道的衔接策略与资源不足时的行为。
6. 系统媒体控制各平台范围、信息缺失策略与时间线语义。
7. 具体音频格式、缓存配额及淘汰、持久化恢复、曲库导入等详细策略。

## 参考资料

- [ncm-api-rs Cargo features](https://github.com/SPlayer-Dev/ncm-api-rs/blob/main/Cargo.toml)
- [AMLL React 与歌词解析](https://github.com/amll-dev/applemusic-like-lyrics)
- [AMLL TTML DB 接入说明](https://github.com/amll-dev/amll-ttml-db#接入到其他项目)：网易云曲目可按 ID 获取 `ncm-lyrics/[ID].ttml`，无需按歌名遍历全库；全库元数据索引也可用于检索。Context7 未检索到该库的匹配索引，此项直接核对官方仓库 README。
- [Auris 无缝播放说明](https://aurisplayer.com/blog/gapless-playback-guide.html)：说明预缓冲、持续输出与编码延迟补偿；实际能力以音频引擎验证为准。
- [GStreamer playbin](https://gstreamer.freedesktop.org/documentation/playback/playbin.html)：提供网络缓冲、解码器选择、自定义音频 sink 及 `about-to-finish`；该回调在 streaming thread 执行，下一首在线资源应提前准备。
- [GStreamer 无缝设计](https://github.com/GStreamer/gstreamer/blob/main/subprojects/gst-docs/markdown/additional/design/playback-gapless.md)：过晚准备会缓冲不足，过早可能遇到资源过期与队列变化。
- [GStreamer Windows 部署](https://gstreamer.freedesktop.org/documentation/deploying/windows.html) 与 [macOS 部署](https://gstreamer.freedesktop.org/documentation/deploying/mac-osx.html)：Rust 绑定仍依赖原生运行时和插件；最终分发体积需实测，部署细节需按采用版本验证。
- [SMTC 时间线](https://learn.microsoft.com/en-us/uwp/api/windows.media.systemmediatransportcontrolstimelineproperties)：Position、StartTime、EndTime 描述媒体内部时间线，还包含 MinSeekTime、MaxSeekTime。
