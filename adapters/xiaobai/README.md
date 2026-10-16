# 小白音乐备用音源适配器

来源 `xiaobai`，以 Rust WASM Component 接入统一 AdapterManager，随应用构建并在适配器设置中独立启停。为网易云、QQ 音乐、酷狗和酷我按已有歌曲 ID 解析音频，不参与搜索，不提供账号或歌词能力；曲目身份保持原平台。

协议依据 [musicdl 的 XiaoBaiMusicClient](https://github.com/CharlesPikachu/musicdl/blob/master/musicdl/modules/common/xiaobai.py)，站点为 `https://music.90svip.cn/`。与煎饼搜共用 `packages/music-web-provider` 中的网页请求和音频校验，保留独立站点和域名授权。完整时长和真实音质均须通过检查，试听与未知完整性拒绝；Hi-Res 首选但返回有损时，只有开启允许音质降级才接受。公开 CDN 音频以不附加站点自定义头的请求探测，通过后交付 URI 播放管线。实际可用性取决于上游及歌曲。

```powershell
node adapters/xiaobai/scripts/build.mjs
cargo test --manifest-path adapters/xiaobai/Cargo.toml --locked
```

构建需要 `cargo-component` 和 Rust `wasm32-unknown-unknown` 目标，产物为 `dist/adapter.component.wasm` 并复制到应用的 bundled-adapters。上游许可见 LICENSE.musicdl，构建同时生成依赖许可告知。

真实宿主在线验收（构建两个 Component 后运行）：

```powershell
./scripts/native.ps1 -Task test -ExtraArgs @('--lib','--locked','web_providers_component_live_lemon','--','--ignored','--nocapture')
```

该验收使用独立临时数据库及空账号，检查小白音乐与煎饼搜对 QQ 版 Lemon（`000akynZ2Rbro5`）的真实 WASM／Host 调用、实际提供器归属、320 kbps 完整资源，以及禁止降级时拒绝 Hi-Res 请求。它不代表全部歌曲、其他三平台或桌面出声已通过实机验收。
