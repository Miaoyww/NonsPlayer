# 音乐网页音源公共实现

供煎饼搜与小白音乐 WASM 适配器共用的 ID 查询、独立匿名会话、域名授权、跳转和有界音频校验。协议参考 musicdl 的 jbsou.py 与 xiaobai.py；许可见 LICENSE.musicdl。站点地址、User-Agent、描述符和域名权限由各适配器提供，平台业务留在 guest，宿主无站点分支。

每次解析独立 Cookie，不保存账号或 URL；站点 User-Agent／Cookie／Referer 不发送给音频 CDN；CDN 探测不附带这些自定义头，校验成功返回无自定义头的资源，以匹配现有 URI 播放管线。最多三次重定向、八次媒体读取，音频探测总量最多 256KiB。使用 Lofty 校验 MP3、FLAC、AAC／ALAC 的实际格式、时长和音质，遵守音质降级设置并拒绝试听、未知完整性与不支持 Range 的资源。播放资源复用宿主现有租约、缓存及失效规则。

```powershell
cargo test --manifest-path packages/music-web-provider/Cargo.toml --locked
```
