# QQ 音乐读取适配器

来源 `qqmusic`，环境中性 JS 协议运行于 ComponentizeJS 引擎，Rust 密码 Component 在构建时组合成一个交付文件。应用通过统一 AdapterManager 后台加载；产品运行不需要 Node、外部服务或本地端口。

协议依赖固定为 `@yakult-green-tea/qq-music-api@3.1.4`，保留 MIT 许可。MD5、RSAES-PKCS1-v1_5、AES、HKDF、SHA-256、HMAC 均由 `packages/wasm-crypto` 提供。构建闭包检查拒绝 Node 内建模块、服务外壳与被替换的 JS 密码实现重新进入产物。

实现搜索、详情、浏览、个人曲库读取、QQ 音乐 App／微信扫码及明文标准／HQ／FLAC 播放资源。账号与不透明凭据统一进入 AccountManager；设备状态不保存登录会话密钥。设备引导与二维码创建分为通用的 prepare-login／beginLogin 调用，每次维持 2 亿 fuel，界面等待完整二维码。未声明收藏、歌单写入、推荐、下载、歌词或加密音频转换能力。曲目 mid、数字歌曲 ID 与音频 media ID 分开处理；游标绑定查询和页大小。播放资源须匹配完整文件名与已知完整大小，拒绝试听、未知完整性、加密文件和禁止的音质降级。

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

Windows 调试版曾测得 Component 编译／加载约 112–143 秒；搜索 5 条约 0.28–0.34 秒。该数据是单次调试环境观察，不能推断发布版启动、内存峰值或长期稳定性。真实手机登录、实际播放与跨平台验证须按验收结果单独记录。
