# QQ 音乐读取适配器

来源 `qqmusic`，协议以 Rust 实现并编译为 WASM Component。应用通过统一 AdapterManager 加载；网络、时钟、安全随机数、WebSocket、设备状态和账号凭据经既有通用 Host 访问。运行不需要 Node、外部服务或本地端口。

当前源码包含搜索、曲目／专辑／艺人／歌单读取、个人曲库、基于“我喜欢”歌单的只读收藏分页、QQ 音乐 App／微信扫码、凭据刷新，以及标准／HQ／FLAC 明文播放资源解析。搜索遇到轻量接口错误码 `2001` 时，仅回退一次同平台桌面搜索。尚未声明收藏写入、歌单写入、推荐、下载、歌词或加密音频转换能力。

Rust 组件构建、模型／密码／协议回归、受控 Host 测试和 Windows 宿主回归已通过，公网搜索与歌曲详情已验证。QQ／微信已通过真实二维码初始化及取消清理，QQ 同时验证 MQTT 连接与订阅；真实手机确认、登录后的公网曲库和实际音频播放仍需验证。完整结果及复验方法见 [验收记录](ACCEPTANCE.md)。

## 文件与边界

| 文件                             | 职责                                             |
| -------------------------------- | ------------------------------------------------ |
| `src/lib.rs`、`src/component.rs` | 统一 JSON transport、Host 绑定、组件生命周期     |
| `src/catalog.rs`、`src/model.rs` | 目录业务、公开 DTO、分页和明文资源校验           |
| `src/auth.rs`、`src/auth/`       | 账号、Cookie、刷新、扫码与 MQTT 协议             |
| `src/device.rs`                  | Android 设备身份、QIMEI 与会话引导               |
| `src/crypto.rs`                  | 有界 RustCrypto 包装，不向宿主增加密码或平台方法 |

账号与不透明凭据统一交给 AccountManager；设备状态写入适配器专属 Host 分区，保存时剔除 `session*` 字段。曲目 MID、数字歌曲 ID 和音频 media ID 分开处理；缺少 MID 的目录曲目按数字 ID 补全，明确不存在的曲目保留不可用占位。游标绑定请求和页大小。

播放仅接受已识别的明文音质前缀与扩展名，并以曲目目录中的完整大小核对资源 HEAD。拒绝试听、未知完整性、加密文件和禁止的音质降级。播放地址与凭据不进入公开元数据或诊断正文；缓存与失效沿用宿主规则。

## 构建

准备项目要求的 Rust 工具链、`wasm32-unknown-unknown` target 和 cargo-component 0.21.1。在仓库根目录执行：

```powershell
node adapters/qqmusic/scripts/build.mjs
```

构建使用本目录 Cargo.lock，生成 `dist/adapter.component.wasm`，并将 Component、描述符与许可复制到应用随包资源。正常 Tauri 开发／打包入口调用同一脚本；QQ 构建不再需要 npm 协议包、ComponentizeJS、WAC 或组合密码组件。其他 JS 适配器仍按各自构建要求安装依赖。

## 测试入口

```powershell
cargo test --manifest-path adapters/qqmusic/Cargo.toml --locked
node --test adapters/qqmusic/tests/component.test.mjs
```

Component 测试必须先重新构建 Rust 产物；jco 与 WASIShim 沿用 `packages/wasm-crypto` 的锁定开发工具依赖。Rust 模型单测和受控 Host 测试不能替代真实手机扫码、刷新、音质与播放验收。

## 协议依据与许可

平台协议依据 `@yakult-green-tea/qq-music-api@3.1.4`，保留 [MIT 许可](LICENSE.qq-music-api)。密码算法使用 RustCrypto，MQTT 数据包使用 mqttbytes 编解码，日期使用 time；依赖版本由 Cargo.lock 固定，构建脚本收集 WASM 生产依赖的许可证为 `LICENSE.dependencies`。
