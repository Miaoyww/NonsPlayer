# GD音乐台独立音源适配器

替代默认接口返回 503 的 Huibq。协议来自本地 `lx-music-source/qdy/latest.js` 的“星海主”分支，并核对 [服务官方说明](https://music-api.gdstudio.xyz/api.php)与[维护者说明](https://github.com/UnblockNeteaseMusic/server/issues/1395)。当前只声明已验证的网易云搜索及 MP3 128k／320k 解析，不因上游列出其它平台就宣称它们可用。

服务署名：**GD音乐台（music.gdstudio.xyz）**。官方接口页标注 CC BY-NC 4.0，仅供学习参考，禁止商用，当前标注限额为每 5 分钟 50 次 API 请求；这些是第三方服务规则，可能变化。NonsPlayer 不托管该服务，也不随包分发它的服务端代码；构建产物包含服务署名和依赖许可。

## 构建与验证

```powershell
npm ci --prefix adapters/gdstudio
# 共用现有 QQ 构建中的 esbuild，以及 wasm-crypto 中的 ComponentizeJS/jco。
npm ci --prefix adapters/qqmusic --include=dev
npm ci --prefix packages/wasm-crypto --include=dev
node adapters/gdstudio/scripts/build.mjs
node --test adapters/gdstudio/tests/*.test.mjs
```

`pnpm --dir app adapters:build` 顺序构建 QQ 与 GD音乐台，复制 Component、manifest 和依赖许可至随包目录。独立音源通过通用 Host 执行，不需要 Node、LX 环境、独立服务进程或平台账号凭据。设置 → 音乐适配器可独立启停。

## 业务范围

- 通过 `types=url` 按稳定网易云 ID 获取播放资源。首选标准音质请求 128k，高音质请求 320k；无损偏好只在允许降级时请求 320k。未实现无损及加密音频转换。
- 通过 `types=search` 搜索单曲，公开引用始终属于 `netease`；音源仅是查询通道及资源提供器，不占用账号／工作区平台身份。按页码与条数映射宿主游标，搜索同时接入单曲聚合页和全屏“歌曲来源”。
- 上游搜索结果不含时长和直接封面 URL。不伪造这些字段：有相同身份的既有曲库元数据时复用；否则保留未知值，播放前只对所选歌曲通过通用曲目接口补齐元数据，不逐首查询整页。不支持没有可靠时长的完整资源验证。
- 歌手／专辑／歌单搜索、账号、收藏、歌词和下载未声明。其它平台的支持需另外验证，不把空地址或请求成功当作可播放。

## 资源与执行约束

- API 和每次跳转均受明确域名授权，最多跟随 3 次跳转；返回其它主机时拒绝，不自动扩大网络范围。取址不携带平台 Cookie 或服务密钥。
- 复用 `music-metadata` 11.16.1 和 `strtok3` 10.5.0 读取最多 64KiB 音频头和 512 字节尾部，核对 Range 总长度、MPEG Layer 3、实际码率和时长。总文件长度最多 512MiB，不把整首音频搬入 guest。
- 时长由成熟解析器读取的帧信息或 CBR 长度估算，与曲目时长差异不得超过 2 秒或 1%，取较大值。这不代表音频指纹或录音版本验证。未知格式、损坏或过大标签、不可验证时长、已知试听、禁止的音质降级及不支持分段探测的服务器明确失败。
- API 429 返回稳定限流错误，不自动重试。查询和播放使用独立并发预算，仍受通用 Host 的域名、时间、内存和响应预算约束；不记录或持久化临时播放地址。

## 验收记录

2026-10-15，本机 Windows x64：

- 对候选服务做小规模检测：Huibq 返回 503，幻音／ikun 的请求出现 TLS 失败，聚合源初始化返回 429。GD音乐台的网易云搜索及两首曲目取址返回有效结果，CDN 返回 206 和 MP3 音频头。该结果只代表本机当时的可用性。
- 独立协议与真实 jco Component 测试覆盖稳定身份、分页、降级、服务错误、音频头／时长、试听拒绝、请求头隔离及未授权地址。
- 项目实际 Wasmtime Host 的受控资源测试通过；真实网络验收经过 ResourceComponent → AdapterManager：搜索 3 首、读取真实平台元数据、强制选择 `gdstudio` 后取得通过校验的 320k 资源。账号库与凭据存储均为隔离临时数据，没有使用正式账号或输出签名 URL。
- 未执行扬声器播放、真实会员受限账号回退的手动验收，以及 macOS／Linux／Android 原生构建。网络服务持续可用性、所有歌曲命中率和 Release 性能均不作保证。

独立契约、来源选择和生命周期见 [资源适配器说明](../../docs/resource-providers.md)。
