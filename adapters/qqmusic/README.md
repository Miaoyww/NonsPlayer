# QQ 音乐读取适配器

来源 `qqmusic`，环境中性 JS 协议运行于 ComponentizeJS 引擎，Rust 密码 Component 在构建时组合成一个交付文件。应用通过统一 AdapterManager 后台加载；产品运行不需要 Node、外部服务或本地端口。

协议依赖固定为 `@yakult-green-tea/qq-music-api@3.1.4`，保留 MIT 许可。MD5、RSAES-PKCS1-v1_5、AES、HKDF、SHA-256、HMAC 均由 `packages/wasm-crypto` 提供。构建闭包检查拒绝 Node 内建模块、服务外壳与被替换的 JS 密码实现重新进入产物。

实现搜索、详情、浏览、个人曲库读取、QQ 音乐 App／微信扫码及明文标准／HQ／FLAC 播放资源。账号与不透明凭据统一进入 AccountManager；设备状态不保存登录会话密钥。设备引导与二维码创建分为通用的 prepare-login／beginLogin 调用，界面等待完整二维码。音乐适配器不设 fuel 指令配额，按时间片让出执行并受调用期限限制。未声明收藏、歌单写入、推荐、下载、歌词或加密音频转换能力。曲目 mid、数字歌曲 ID 与音频 media ID 分开处理；游标绑定查询和页大小。播放资源按返回文件的音质前缀、明文扩展名与目录中的完整大小验证，允许 CDN 规范化资源文件名；拒绝试听、未知完整性、加密文件和禁止的音质降级。

## 构建

准备 Rust `wasm32-unknown-unknown` target、Node／pnpm、WAC，以及项目既有原生依赖。分别安装本目录和 `packages/wasm-crypto` 锁定依赖（`npm ci`）。设置 `NONS_WAC_BIN` 为 WAC 路径；Windows 本地默认 `.local/tools/bin/wac.exe`。

在仓库根目录运行：

```powershell
node adapters/qqmusic/scripts/build.mjs
node --test adapters/qqmusic/tests/*.test.mjs
```

生成 `dist/adapter.component.wasm`，复制 Component、描述符和许可到应用资源。正常 Tauri 开发／打包入口先执行此构建；首次环境仍需安装上述工具。Node 仅用于构建和测试。

## 验证口径

Component 测试执行真正组合的 JS／Rust 产物，用受控 Host fixture 验证搜索、详情、双扫码初始化、订阅就绪、设备状态及释放；不代表真实手机登录完成。原生显式测试 `music::external::acceptance::qq_component_live_search_and_optional_login` 验证 Wasmtime 与公网接口；`NONS_QQ_QR_METHOD=qq,wechat` 依次等待用户扫码。该测试使用隔离的临时账号库与测试凭据存储，不修改正式账号库。

2026-10-15 本机调试构建验收：Windows 11 x64（10.0.26300）、Intel Core Ultra 9 275HX、24 个逻辑处理器、31.4 GiB 内存；共享编译池仅使用两个工作线程。组件大小 13,771,829 字节。以下均为单次观察，不是平均值或分位数，不代表发布版、整个应用启动或其他机器性能：

| 场景               | 测量范围                                        | 耗时      |
| ------------------ | ----------------------------------------------- | --------- |
| 优化前、无编译缓存 | 原生适配器完整加载，含编译、实例化及描述符验证  | 137.83 秒 |
| 优化后、空缓存     | 相同组件的原生适配器完整加载                    | 8.77 秒   |
| 优化后、已有缓存   | 新引擎重新加载并执行描述符验证                  | 0.459 秒  |
| 另一进程、已有缓存 | 单独测量 `Component::new`，命中计数确认磁盘复用 | 0.443 秒  |

独立的空缓存编译测得 9.348 秒；与完整加载的测量属于不同运行。该次缓存目录共 3 个文件、约 21.5 MiB，不是内存占用。实现复用 Wasmtime 原生缓存与 512 MiB 软限制，并优化调试构建中主要编译器依赖；缓存位置、失效与回收见[缓存约定](../../docs/cache-and-local.md#wasm-编译结果缓存)。

通过源码／编译配置失效、损坏恢复、引擎重建命中及目录不可用回归，原生库测试 190 项通过（8 项显式跳过）；大歌单受控 Host 调用通过。相同轮次的公网搜索返回 `SourceUnavailable`，不能据此声称在线搜索与播放验收通过。真实界面冷启动、手机登录、实际播放、峰值内存及跨平台性能仍须单独验证。
