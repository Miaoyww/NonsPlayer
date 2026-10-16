# GD音乐台独立音源适配器

GD音乐台通过统一 ResourceProvider 接入单曲搜索、备用取址和“歌曲来源”。支持十个平台的请求与身份映射，实际可用性由上游和歌曲决定，不保证所有声明平台都能取得可播放资源。

## 协议与平台身份

核对本地 musicdl 的 `musicdl/modules/common/gdstudio.py` 后，改用网站接口 `https://music.gdstudio.org/api.php`，而不是覆盖范围较小的公开 API。网站每次请求先读取 `/time`，使用十秒时间桶、当前主机、网站版本 `2026.09.25` 和 URI 编码的关键词／歌曲 ID 计算签名。协议摘要有一个不同于标准 MD5 的轮常数，不能直接换成普通 MD5。

签名由适配器内独立实现，未引入 musicdl 运行时或复制其实现代码；宿主继续只提供通用 HTTP、时间和随机数能力。网站升级可能使签名失效。使用 .org 是因为本机 Node 对 .xyz 的直接连接异常；没有改变宿主的网络安全策略。

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

结果保留平台原始 ID，不按歌名自动换平台。JOOX 每页固定 30 项、YouTube 每页固定 20 项，按宿主逻辑页的偏移重新切片，跨上游页时继续请求，避免遗漏。搜索读取已有时长；缺失时播放前复用既有曲库补齐。哔哩哔哩仅为当前选择的单个 BVID 读取公开详情校验时长，多分 P 不按总时长冒充当前音轨。封面未声明，缺失时保留占位。

仅支持单曲搜索与资源解析，不声明账号、收藏、歌词、下载、歌手／专辑／歌单搜索。十个平台仍是同一个独立音源的查询渠道，不注册完整平台业务或专用账号存储。

## 资源和执行预算

- 标准／高音质请求 128／320，无损／高解析度请求 999；依据实际格式与码率报告结果，并执行是否允许降级的设置。
- 复用 music-metadata 与 strtok3 解析 MP3、FLAC、AAC／ALAC MP4。稀疏 tokenizer 支持跳过媒体数据、读取尾部 moov；每次 HTTP Range 最多 64KiB，最多 8 次、合计 256KiB，单次解析器分配最多 64KiB，总文件最大 512MiB，不下载整首歌曲。
- 时长与可信曲目时长差异不得超过 2 秒或 1%，取较大值。该校验只验证格式、音质和时长，不验证音频指纹或录音版本。未知时长、过大标签／索引、损坏数据、已知试听、空地址及不支持 Range 的服务明确失败。
- API 和所有重定向均检查 manifest 的域名授权，最多 3 次跳转。网站请求头只送到网站接口，音频探测不带网站头、Cookie 或平台凭据；返回 URI-only 播放资源。
- 单次 Host HTTP 最多 4 秒，搜索与解析继续受宿主整体期限、并发、内存和响应大小预算约束。429 返回限流错误，不自动重试。临时地址不进入普通缓存、队列或诊断报告。

## 构建与验证

```powershell
npm ci --prefix adapters/gdstudio
npm ci --prefix adapters/qqmusic --include=dev
npm ci --prefix packages/wasm-crypto --include=dev
node adapters/gdstudio/scripts/build.mjs
node --test adapters/gdstudio/tests/*.test.mjs
# 实网顺序探测，每平台最多两首；报告不保存临时播放 URL
node adapters/gdstudio/scripts/probe.mjs Hello
# 限定平台和报告路径
node adapters/gdstudio/scripts/probe.mjs Adele qobuz,applemusic .local/gdstudio-expansion/alternate.json
# 真实 Wasmtime Host；没有此环境变量时只验收网易云既有路径
$env:NONS_GDSTUDIO_ALL='1'
./scripts/native.ps1 -Task test -ExtraArgs @('--lib','--locked','gdstudio_component_live_search_and_resource','--','--ignored','--nocapture')
```

`pnpm --dir app adapters:build` 将 Component、manifest 和依赖许可复制到随包目录。安装后不依赖 Node、Python、LX 环境或独立服务进程，可在设置 → 音乐适配器独立启停。

## 2026-10-16 实网记录

环境：Windows x64，Node 24.15.0，顺序搜索 Hello、每个平台前两首；Qobuz 与 Apple Music 另用 Adele 复测。以下是当时样本结果，不能推算整个平台命中率。

| 平台          | 搜索             | 音频头、音质与完整时长校验                       |
| ------------- | ---------------- | ------------------------------------------------ |
| 网易云        | 有结果           | 两首 FLAC 通过，包括 24-bit 样本                 |
| JOOX          | 有结果           | 一首 24-bit FLAC 通过，另一首空资源              |
| 酷我          | 有结果           | 一首 FLAC 通过，另一首格式不可识别               |
| TIDAL         | 有结果           | 两首 AAC 320k 通过                               |
| Qobuz         | 两个关键词均为空 | 未取得可验证资源                                 |
| Apple Music   | 有结果           | 空资源，或元数据块 391335 字节超出解析分配预算   |
| 哔哩哔哩      | 有结果           | 样本解析失败或音轨与视频时长不一致，拒绝声明完整 |
| YouTube Music | 有结果           | 样本空资源，或音频 Range 请求返回 403            |
| Embeat        | 有结果           | 两首 AAC 320k 通过                               |
| QQ音乐        | 有结果           | 两首 FLAC 通过                                   |

纯协议与真实 jco Component 回归覆盖十个平台身份、签名、JOOX／YouTube 跨页、空资源、错误、音质降级、FLAC 解析、MP3 完整时长、网站头隔离和未授权地址。项目真实 Wasmtime Host 中又逐项测试十个平台：网易云、JOOX、酷我、TIDAL、Embeat、QQ音乐均取得 GD 资源票据（共 11 首通过）；Apple Music 两首分别校验失败／空资源，哔哩哔哩两首网络失败，Qobuz 与 YouTube 本轮搜索为空。此前网易云 320k 路径也单独通过。实网 probe 的 HTTP 超时为 12 秒，不能代替 Host 4 秒单请求与整体 12 秒预算的验收。

没有执行扬声器播放、跨平台原生构建或长期稳定性测试。原生 cargo 全集成测试还存在既有 Windows output 测试导入错误，相关 Host 验收使用 `--lib`。

服务署名：GD音乐台（music.gdstudio.xyz／music.gdstudio.org）。服务政策和依赖许可继续包含在构建产物中；不随包分发 GD 服务端代码。具体服务规则见[官方接口说明](https://music-api.gdstudio.xyz/api.php)。独立契约和生命周期见[资源适配器说明](../../docs/resource-providers.md)。
