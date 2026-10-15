# 通用适配器 Host

`wit/host.wit` 定义 `nons:host/transport@1.0.0`。宿主同时复用功能插件的 Component runtime 和 HTTP 基础设施；加载上下文分别绑定，音乐协议不进入宿主。

- HTTP：GET／HEAD／POST，合法自定义头，原始字节与逐条 Set-Cookie。请求正文 64KiB、响应正文 2MiB、头 16KiB、期限最多 15 秒；宿主管理 framing，不跟随重定向。
- 网络：按包声明域名授权，拒绝私网、回环、保留地址、未授权端口和 URL 凭据。HTTP／WSS 直连，不继承代理。WSS 每上下文最多 2 连接、帧 16KiB、发送队列 16、事件队列 32；宿主处理 TLS／Ping／Pong，guest 处理应用协议。
- 凭据：read／replace／invalidate 绑定 Store 中账号与实例代次，不能指定别人的账号。replace 推进账号代次，旧业务结果失效；invalidate 持久化失败返回错误。publish 只供登录挑战完成，交由 AccountManager 保存，凭据不返回 WebUI。
- 非敏感状态：按来源隔离，最多 16 个键，每值 8KiB、总量 64KiB；取消或代次变化拒绝交付／写入。
- 标准能力：选定 WASI clocks、random、io 子集，不提供文件系统、进程、环境变量或原始 sockets。

`Account` Component 的登录调用顺序为 `music.prepare-login`（返回 null）、`music.account/beginLogin`（返回完整挑战），两阶段共用 Store、账号上下文和外层期限，各自执行单次 fuel 预算。宿主不解释准备内容，准备失败时不展示占位二维码。

普通调用使用独立 Store；登录挑战最多 180 秒、4 个会话，占查询并发额度。完成、取消、到期、停用或重载释放 Store、连接与敏感状态。普通功能插件仍使用原 JSON／资源预算，不因适配器扩容而扩容。

聚合适配器临时预算：32MiB 包、256MiB 总线性内存、4 个 memory、每次调用 2 亿 fuel、初始化／调用 15 秒。编译耗时和应用总内存不等于 guest 调用期限或线性内存预算，需分别实测。
