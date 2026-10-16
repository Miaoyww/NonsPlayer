# 煎饼搜备用音源适配器

来源 `jbsou`，使用 Rust 构建为 WASM Component，由应用的统一 AdapterManager 后台加载。它只按已有歌曲 ID 为网易云、QQ 音乐、酷狗和酷我曲目解析备用音源，不提供搜索、账号或歌词能力。

适配器为每次解析创建独立匿名会话，复现煎饼搜网页所需的访问 Cookie，并限制跳转次数、响应大小、媒体探测请求数和总读取量。返回地址必须落在清单声明的域名范围内；站点 Cookie 不会发送给音频 CDN。

解析结果通过 HTTP Range 有界读取，并使用 Lofty 校验 MP3、FLAC、AAC 或 ALAC 的真实格式、时长和音质。时长不符、试听资源、禁止的音质降级、无 Range 支持、未知格式和越权跳转均会被拒绝。可用性取决于上游站点及具体歌曲。

构建：

```powershell
node adapters/jbsou/scripts/build.mjs
```

构建需要 Rust `wasm32-wasip2` 目标。产物写入 `adapters/jbsou/dist`，并复制到应用的 bundled adapters 目录。协议参考 musicdl 的 jbsou 实现；许可文本见 `LICENSE.musicdl`。
