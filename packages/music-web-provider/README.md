# 音乐网页音源公共实现

供小白音乐 WASM 适配器使用的 ID 查询、独立匿名会话、跳转和音频校验。协议参考 musicdl 的 xiaobai.py；许可见 LICENSE.musicdl。站点地址、User-Agent、描述符由各适配器提供，平台业务留在 guest，宿主无站点分支。

请求使用 Host 1.1 的 `follow_redirects`、零值无限 `timeout_ms` 和响应最终 URL；可访问任意合法 HTTP(S) 目标，不固定总响应大小、请求头数量、Cookie 数量或媒体总大小。每次解析独立 Cookie，不保存账号或 URL；站点 User-Agent／Cookie／Referer 只发送给同源站点，音频探测和返回资源不附带这些自定义头，以匹配现有 URI 播放管线。媒体校验使用 Range 分块，仅保留当前块，随机 seek 跨块时重新取回，不整曲载入内存；使用 Lofty 校验 MP3、FLAC、AAC／ALAC 的实际格式、时长和音质，遵守音质降级设置并拒绝试听、未知完整性与不支持 Range 的资源。播放资源复用宿主现有租约、缓存及失效规则。

```powershell
cargo test --manifest-path packages/music-web-provider/Cargo.toml --locked
```
