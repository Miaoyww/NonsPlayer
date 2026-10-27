# QQ Rust 适配器验收记录

## 当前状态

QQ 的目录、账号、设备引导、扫码和明文资源解析使用 Rust；构建入口使用 cargo-component，宿主业务契约、来源标识和权限声明沿用现有定义。Windows 上的 Rust 组件构建、自动化回归、公网搜索及歌曲详情已通过。

QQ／微信均已完成真实二维码初始化和取消清理；QQ 同时验证了设备引导、MQTT 连接与订阅。手机确认、登录后的公网曲库／刷新／资源解析、实际音频播放、macOS/Linux 和 Android 仍未完成验收。QQ 手机确认测试在二维码期限内未收到确认，按未验证记录；受控 Host 的双扫码与播放用例不替代这些实测。

## 已执行的验证

| 检查                          | 结果                                                                                         |
| ----------------------------- | -------------------------------------------------------------------------------------------- |
| QQ Rust 库测试                | 25 项通过：4 项模型、8 项数字转换、13 项密码向量与边界                                       |
| Rust Component 构建与接口检查 | 通过；只导入 transport、诊断、WASI wall-clock/random，导出 initialize/shutdown/call          |
| Component 受控 Host 测试      | 28 项通过；包含双扫码确认、刷新、分页、三档明文资源校验、MQTT 拆包／粘包／终态保护与数量限制 |
| Windows 宿主库回归            | 354 项通过，12 项显式忽略                                                                    |
| 原生 QQ 大歌单预算            | 显式测试通过；25 首结果单次观察耗时 2 ms                                                     |
| 公网 QQ 搜索与详情            | 显式测试通过；“晴天”返回 5 首，首曲详情身份一致                                              |
| 前端测试与生产构建            | 471 项测试通过，TypeScript／Vite 构建通过                                                    |
| Windows 主程序构建            | cargo build 通过                                                                             |
| 发布脚本回归                  | 27 项通过                                                                                    |
| 公网 QQ／微信二维码与取消     | 两种方式均创建真实二维码，取消后挑战表为空、临时账号库没有保存账号                           |

测量环境：Windows 11 x64 10.0.26300、Intel Core Ultra 9 275HX、24 个逻辑处理器、31.4 GiB RAM、Rust 1.95.0；QQ 使用优化的 WASM 构建，原生测试使用 debug 宿主。组件为 830,239 字节。一次隔离编译缓存的原生公网测试中，组件冷加载 515.8 ms、同进程缓存加载 32.5 ms，搜索 315.9 ms；均为单样本观察，不是均值或分位数，也不代表整个应用启动、首声或运行内存。

修正代码后应重新运行受影响的构建与测试，并更新以上结果；未执行的平台和人工场景保留为未验证。

## 执行顺序

从仓库根目录执行以下步骤。每次修正代码后，先重新格式化，再重新运行受影响的构建与测试。

1. 阅读根目录 AGENTS.md、`docs/architecture.md`、`docs/cache-and-local.md` 和适配器相关 ADR。查看工作区差异，确认源码变更完整且没有夹带其他改动。
2. 执行 `pnpm --dir app format`，先修复格式化错误。该入口包含 QQ Cargo.toml；格式化产生的改动也要检查。
3. 准备 Rust `wasm32-unknown-unknown` target 与 cargo-component 0.21.1。为 Node Component harness 安装 `packages/wasm-crypto` 的锁定 npm 开发依赖。QQ 本身不需要 npm 协议库。
4. 执行下列本机库测试、Component 构建和受控 Host 测试。所有命令应退出 0；失败时修正代码，保持现有业务边界和权限上限。

```powershell
cargo test --manifest-path adapters/qqmusic/Cargo.toml --locked
node adapters/qqmusic/scripts/build.mjs
node --test adapters/qqmusic/tests/component.test.mjs
```

5. 检查实际 Rust Component 的导入、导出与体积：导出 `initialize`、`shutdown`、`call`，导入既有 `nons:host/transport`、诊断 Host 及所声明的 WASI wall-clock/random 子集；没有 JS 引擎、额外文件系统、进程、网络或环境接口。确认依赖许可随包复制。不要用本地残留的旧 WASM 代替新构建。
6. 执行宿主库回归及显式 QQ 分页预算测试。大型歌单在既有查询期限和 JSON 上限内返回，不因为查询占满而耗尽播放预算。

```powershell
cargo test --manifest-path app/src-tauri/Cargo.toml --lib --locked
cargo test --manifest-path app/src-tauri/Cargo.toml --lib --locked music::external::acceptance::qq_component_large_playlist_respects_page_and_execution_budget -- --ignored --nocapture
```

7. 在有公网和可操作手机的环境执行下列真实接口测试。它使用隔离的临时账号库，不修改正式账号库。测试中的可选扫码不等于自动完成；两种扫码方式都应由人实际确认。

   只验证二维码初始化与取消时，额外设置 `NONS_QQ_QR_BOOTSTRAP_ONLY=1`；执行手机确认前应清除此变量。

```powershell
$env:NONS_QQ_QR_METHOD = 'qq,wechat'
cargo test --manifest-path app/src-tauri/Cargo.toml --lib --locked music::external::acceptance::qq_component_live_search_and_optional_login -- --ignored --nocapture
Remove-Item Env:NONS_QQ_QR_METHOD
```

8. 执行应用相关前端检查与打包构建，确认随包资源使用新 Rust 产物。完成下方功能表和跨平台检查后，执行 `pnpm --dir app format:check`，以全部通过为格式验收标准。达到必要验证后，再按仓库约定拆分提交，作者使用 Miaoyww，邮箱沿用仓库配置。

## 必须核对的功能

| 场景             | 通过条件                                                                                                                             |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 四类搜索         | 关键词、类型、页大小和游标正确；`2001` 仅回退一次桌面搜索；其他错误不循环重试                                                        |
| 曲目身份         | MID／数字 ID／media ID 分离；数字补全保持原队列引用，找不到的歌曲仍保留不可用位置                                                    |
| 专辑、艺人、歌单 | 详情与分页正确，公开歌单、自建、收藏及“我喜欢”不互相混淆；分页数量和 JSON 大小有界                                                   |
| 曲库与收藏       | QQ／微信均能读取本人创建、收藏歌单、收藏专辑和喜欢状态；系统喜欢目录不重复列为自建歌单                                               |
| 双扫码           | prepare-login 不提前开放订阅；QQ 收到 CONNACK／SUBACK 后显示二维码；微信二维码与确认交换完成                                         |
| 登录生命周期     | 等待、已扫码、确认、失败、过期清楚；切换来源、退出页面、取消、停用和重载释放连接与临时状态，迟到事件不能发布凭据                     |
| 凭据边界         | 发布只经 AccountManager；重启可恢复当前凭据；过期刷新推进账号代次并拒绝旧结果，刷新拒绝使会话失效；凭据保存失败不宣称登录成功        |
| 协议字节         | QIMEI、HKDF/AES/RSA/HMAC 与固定向量一致；设备状态不含 `session*`；畸形响应、超大输入和错误参数不 panic                               |
| MQTT             | 拆包、粘包、SUBACK 失败、QoS／属性与关闭事件正确；缓冲与事件数量有界，取消和期限错误不能被重试吞掉                                   |
| 明文播放         | 标准／HQ／FLAC 的文件请求、实际音质、media ID、CDN 规范化文件名正确；HEAD 与目录完整大小一致；试听、加密、未知长度及禁止降级明确拒绝 |
| CDN              | 无 sip 时的测速与备用域名选择有界；慢／坏候选不越过宿主期限；取消、账号变化、停用后不交付资源                                        |
| 诊断             | 可见稳定阶段和上游错误码，日志、公开结果及失败信息不包含 Cookie、凭据、签名 URL 或完整响应                                           |

Rust 模型单测已迁入 `src/model/tests.rs`，Component harness 位于 `tests/component.test.mjs`。密码固定向量可参照 `packages/wasm-crypto/src/lib.rs` 中现有用例，但必须对本适配器实际链接的实现补足验证。现有测试覆盖不足的生命周期、分包和异常响应场景应补测试后执行。

## 平台与性能

Windows 优先验证应用启动、真实手机登录、实际音频播放与混合队列。macOS/Linux 至少完成编译和基本播放；Android 按项目现有支持范围单独记录结果。没有执行的平台明确列为未验证。

记录工具链、系统与硬件、构建配置、是否命中 Wasmtime 编译缓存、样本量和统计口径，分别测量组件体积、冷编译／加载、热加载、缓存命中首声及后台播放内存。总内存包括相关 WebView 和 WASM；编译缓存磁盘字节不能当成运行内存。CDN 探测在当前同步 Host 导入下逐个执行，需测量首声影响，不能从 Rust 源码语言直接推断加速幅度。
