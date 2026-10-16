# GD音乐台独立音源适配器

GD音乐台只用于按已有平台歌曲 ID 获取替换音源，接入自动备用取址和“歌曲来源”的同平台音源选择。由于聚合搜索会快速消耗上游请求额度，已移除搜索能力：不进入聚合搜索或“歌曲来源”的搜索渠道，不提供关键词查询，不注册完整平台业务或专用账号存储。

## 协议与平台身份

适配器使用网站接口 `https://music.gdstudio.org/api.php?types=url`。取址前读取 `/time`，使用十秒时间桶、当前主机、网站版本 `2026.09.25` 和 URI 编码的歌曲 ID 计算签名。协议摘要有一个不同于标准 MD5 的轮常数，不能直接换成普通 MD5。签名在适配器内实现，未引入 musicdl 运行时；宿主继续提供通用 HTTP、时间和随机数能力。网站升级可能使签名失效。

| 产品平台标识 | GD source | 平台          |
| ------------ | --------- | ------------- |
| netease      | netease   | 网易云音乐    |
| joox         | joox      | JOOX          |
| kuwo         | kuwo      | 酷我音乐      |
| tidal        | tidal     | TIDAL         |
| qobuz        | qobuz     | Qobuz         |
| applemusic   | apple     | Apple Music   |
| bilibili     | bilibili  | 哔哩哔哩      |
| ytmusic      | ytmusic   | YouTube Music |
| embeat       | embeat    | Embeat        |
| qqmusic      | tencent   | QQ音乐        |

保留十个平台的取址映射，实际可用性由上游和歌曲决定，不保证所有声明平台都能取得可播放资源。`searchPlatforms` 为空，调用 `music.search-resources` 直接返回 `unsupported`，不发起 HTTP 请求。账号、收藏、歌词、下载及所有分类搜索均未声明。

## 资源和执行预算

- 标准／高音质请求 128／320，无损／高解析度请求 999；依据实际格式与码率报告结果，并执行是否允许降级的设置。
- 复用 music-metadata 与 strtok3 解析 MP3、FLAC、AAC／ALAC MP4。每次 HTTP Range 最多 64KiB，最多 8 次、合计 256KiB，单次解析器分配最多 64KiB，总文件最大 512MiB，不下载整首歌曲。
- 时长与可信曲目时长差异不得超过 2 秒或 1%，取较大值。未知时长、过大标签／索引、损坏数据、已知试听、空地址及不支持 Range 的服务明确失败；不验证音频指纹或录音版本。
- 缺失时长时复用宿主既有曲库补齐。哔哩哔哩仅为当前选择的单个 BVID 读取公开详情校验时长，多分 P 不按总时长冒充当前音轨。
- API 和所有重定向均检查域名授权，最多 3 次跳转。网站头不传到音频 CDN，不携带平台凭据；返回 URI-only 播放资源。
- 单次 Host HTTP 最多 4 秒，解析继续受宿主整体期限、并发、内存和响应预算约束。429 返回限流错误，不自动重试。临时地址不写普通缓存、队列或诊断报告。

## 构建与验证

```powershell
npm ci --prefix adapters/gdstudio
npm ci --prefix adapters/qqmusic --include=dev
npm ci --prefix packages/wasm-crypto --include=dev
node adapters/gdstudio/scripts/build.mjs
node --test adapters/gdstudio/tests/*.test.mjs
# 受控 Host 预算验收，不访问网络
./scripts/native.ps1 -Task test -ExtraArgs @('--lib','--locked','gdstudio_component_resolves_under_native_host_budget','--','--ignored','--nocapture')
# 可选实网取址：已知网易云 ID 347230，先读平台元数据，不搜索 GD
./scripts/native.ps1 -Task test -ExtraArgs @('--lib','--locked','gdstudio_component_live_resource','--','--ignored','--nocapture')
```

构建更新随包 Component、manifest 和依赖许可；安装后无需 Node、Python 或独立服务进程。在设置 → 音乐适配器独立启停。

回归覆盖搜索能力为空、十个平台搜索调用零 HTTP、平台 ID 取址映射、签名、MP3／FLAC 校验、音质降级、网站头隔离及未授权地址。删除了原有批量搜索探测脚本。2026-10-16 移除搜索之前的实网样本中，网易云、JOOX、酷我、TIDAL、Embeat、QQ音乐曾取得通过校验的资源；其余平台仍有空资源、网络或格式／时长校验失败。该历史样本不保证后续可用性。本次验收使用受控测试，避免继续消耗上游额度；没有执行扬声器播放和跨平台原生构建。

服务署名：GD音乐台（music.gdstudio.xyz／music.gdstudio.org）。服务政策和依赖许可包含在构建产物中，不分发 GD 服务端代码。独立契约和生命周期见[资源适配器说明](../../docs/resource-providers.md)。
