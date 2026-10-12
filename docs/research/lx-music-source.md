# LX 音源接入研判

日期：2026-10-12。本文是研究建议，不是已接受的架构决策或已实现功能。

## 审查范围

- 仓库：[pdone/lx-music-source](https://github.com/pdone/lx-music-source)，克隆位置 `C:\Projects\NonsPlayersProject\lx-music-source`。
- 审查提交：`489a2e36ecb8b7c4a58ae45d8a1918c0c0591da1`。
- 静态检查 README、版本文件、可读脚本与混淆脚本中的接口引用；没有执行音源脚本，没有请求音源服务或音频，没有验证服务可用性和实际音质。
- 官方协议依据：[LX 自定义源说明](https://lxmusic.toside.cn/desktop/custom-source)。Context7 检索了 `/lyswhut/lx-music-desktop`，细节以官方文档和源码为准。

## 仓库实际提供什么

这是十组 LX 自定义音源 JS 脚本的分发集合，不是完整音乐平台 SDK、播放器或服务端。`CreateLatest.py` 只复制各版本文件为 `latest.js`。README 明确内容来自网络，各脚本使用不同第三方服务。

标准流程是脚本通过 `globalThis.lx.send(inited, ...)` 声明平台、动作及音质，通过 `on(request, ...)` 接收请求。主要动作 `musicUrl` 接收平台、歌曲信息和请求音质，返回音频 URL。

可读样本证明：

- [Huibq](https://github.com/pdone/lx-music-source/blob/489a2e36ecb8b7c4a58ae45d8a1918c0c0591da1/huibq/latest.js)：声明 `kw/kg/tx/wy/mg`，只实现 `musicUrl`，用 `hash ?? songmid` 请求远端 URL 服务。
- [ikun](https://github.com/pdone/lx-music-source/blob/489a2e36ecb8b7c4a58ae45d8a1918c0c0591da1/ikun/latest.js)：只实现 `musicUrl`，另外有更新检查；存在非标准来源及音质声明，不能当作所有脚本共同契约。
- [幻音](https://github.com/pdone/lx-music-source/blob/489a2e36ecb8b7c4a58ae45d8a1918c0c0591da1/huanyin/latest.js)：除播放解析外有咪咕 `search`，但结果 ID 由时间和随机数生成；播放时再按元数据搜索。不能直接满足稳定曲目身份与持久化要求。
- [QDY](https://github.com/pdone/lx-music-source/blob/489a2e36ecb8b7c4a58ae45d8a1918c0c0591da1/qdy/latest.js)：包含自定义 `qsvip` 来源及搜索、播放、歌词扩展，需要按专用协议评估和转换。
- [聚合](https://github.com/pdone/lx-music-source/blob/489a2e36ecb8b7c4a58ae45d8a1918c0c0591da1/juhe/latest.js)：初始化从服务端取得能力；服务端还可以返回第二段 HTTP 请求指令。因此权限必须在每次真实请求时检查，不能只检查初始域名。

官方标准平台键是 `kw/kg/tx/wy/mg/local`；在线平台标准动作是 `musicUrl`，`local` 还可声明 `lyric/pic`。仓库里的搜索、自定义来源与额外音质属于脚本扩展，不能以“兼容 LX 标准”承诺全部支持。

本次克隆未发现 LICENSE；收集仓库不能代表各作者授予再分发许可。随包发布或移植代码前需分别核对来源许可。服务端能否长期使用也不能由 JS 文件的存在证明。

## 接入 Nons 的什么

首选接入播放资源解析：曲目引用及元数据 → LX `musicUrl` → 受控 `PlaybackResource` → 既有音频核心。

搜索、曲库、歌单、账号和收藏继续由对应音乐适配器提供。特定脚本确有搜索或歌词时，另做能力转换；不能把所有脚本声明为完整音乐来源，也不能从音源支持平台列表推断其支持平台搜索。

歌曲平台身份与解析脚本身份应分开：`netease` 是来源，Huibq 等是可选择的资源解析实现。切换脚本不应改变歌曲引用；跨平台匹配需保存关联，不可复用网易云 ID 作为其他平台 ID。

## 两条实现路线

### 只支持选定服务

以一个可读脚本为样本，把其网络请求和响应转换为独立原生解析组件。这不需要运行任意 JS，但只支持选定协议；服务更新时需要维护代码。

先验证 `wy` 可沿用现有网易云元数据与歌曲 ID。现有管理器禁止重复注册同一来源，不能再注册一个 `netease` 适配器。可在网易云适配器实现中组合解析器；若要允许用户安装和切换独立解析器，则需另行定义宿主资源解析扩展契约并记录 ADR。不能把通用 LX 运行时硬塞进网易云平台包。

### 支持导入 LX JS

需要新增独立于 WebView 的受控后端 JS 执行环境，使用成熟引擎，并按官方协议提供 `globalThis.lx` 的事件、HTTP、环境、版本、脚本信息和加密/字节工具；定时器、Promise、取消和清理也需要明确实现。

不能把脚本直接作为现有 React 前端执行，也不能把 `.js` 当 Wasmtime Component 加载。若通过 WASM 包装 JS 引擎复用现有运行设施，仍需实现 JS 与 LX API，且现有外部音乐 WASM 桥尚不开放 Host Capability。

混淆、原文自检、非标准动作和动态初始化导致兼容性须逐脚本验证；不得一开始承诺所有脚本可用。导入保存原始字节与内容哈希，更新作为新版本重新校验和授权，不静默执行远程最新脚本。

## 宿主需改变的边界

1. **资源解析扩展**：当前 `MusicAdapter` 强制提供 `read_track` 和 `resolve_playback`，仅解析 URL 的脚本缺少单曲读取；需要组合适配器或独立解析器契约。参见 [共享契约](../../packages/music-adapter-sdk/src/adapter.rs)、[ADR 0006](../adr/0006-music-provider-adapters.md)。
2. **多来源播放与持久化**：实际旧队列 `TrackSource` 仍只有网易云数字 ID 和本地文件。接 QQ/酷我等须迁移曲目、队列、存储及菜单 DTO 到稳定来源与不透明 ID，兼容已有数据。参见 [旧模型](../../packages/music-adapter-sdk/src/legacy.rs)、[播放 facade](../../app/src-tauri/src/music/service.rs)、[前端模型](../../app/src/lib/player.ts)。
3. **曲目信息与音质转换**：不同平台需要 `songmid/hash` 等不同字段，不能用同名字段猜测身份。音质标识映射后，声明的音质只能表示请求能力，不能证明实际返回质量、完整时长或播放权限。
4. **网络与执行预算**：域名、重定向、嵌套请求逐次检查；绑定解析器实例代次，限时、限内存、限并发和响应大小，停用取消，播放与预加载保留独立预算。脚本不进入音频线程，不默认接触宿主账户凭证。参见 [职责边界](../adr/0005-plugin-capability-boundary.md)、[运行边界](../adr/0003-runtime-plugins.md)。
5. **播放资源验证**：URL 留在后端，宿主控制生命周期及缓存；未知有效期设上限，失败重新解析，日志过滤地址和密钥。当前 [资源校验](../../app/src-tauri/src/music/manager.rs) 拒绝非空音频请求头；若返回资源需要头部认证，另改音频传输。解析 API 的请求头不等于音频请求头。
6. **歌词**：专用歌词扩展接入既有歌词服务、来源顺序和缓存，不能让脚本直接接管渲染或绕过页面隐藏规则。参见 [缓存约束](../cache-and-local.md)。
7. **管理**：增加解析器导入、能力查看、平台配置、版本与授权、启停、故障展示；来源与解析实现分别标识。同平台多个解析器按用户配置选择，明确回退行为。

## 建议实施顺序

先选择一个协议可读且可授权使用的脚本，仅实现网易云元数据到播放地址再到既有音频核心的闭环，测试真实音质、有效期、定位、预加载、失败及停用。此阶段不证明其他服务可用。

用户确需直接导入和切换 `.js` 时，再建设 LX 兼容运行时与独立解析器管理。需要其他平台搜索、歌单及完整曲库时，继续接入对应音乐适配器并迁移实际队列模型；不依赖这个脚本集合自动补齐。
