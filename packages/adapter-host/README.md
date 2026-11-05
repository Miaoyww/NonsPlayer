# 通用适配器 Host

`wit/host.wit` 定义 `nons:host/transport@1.1.0`。契约版本 1.1.0。宿主同时复用功能插件的 Component runtime 和 HTTP 基础设施；加载上下文分别绑定，音乐协议不进入宿主。

适配器构建应直接使用本目录的 `host.wit`；record 字段顺序参与 Component ABI，副本不一致时会出现导入名称相同但实例化失败。

- HTTP：任意合法方法、合法自定义头、原始字节正文与逐条 Set-Cookie；宿主原样交付，不限制主机，由共享 HTTP 实现处理 framing。请求与响应正文默认不设大小限制，调用方按需检查；`follow-redirects` 控制是否跟随重定向，跟随不设帧数上限但遇到重复实际 URL 时终止，响应返回实际最终 URL；`timeout-ms` 为 0 时不设期限，正值设置显式期限。
- 网络：目标不限主机、IP、端口和 URL 凭证，私网与回环地址可用；适配器 Manifest 不含网络权限或目标主机字段。HTTP／WS 直连，不继承代理。WS／WSS 帧与消息大小默认不限；发送队列 16、事件队列 32，连接和事件等待由调用方通过 `timeout-ms` 显式约束，0 表示一直等待。宿主处理 TLS／Ping／Pong，guest 处理应用协议。
- 凭据：read／replace／invalidate 绑定 Store 中账号与实例代次，不能指定别人的账号。replace 推进账号代次，旧业务结果失效；invalidate 持久化失败返回错误。publish 只供登录挑战完成，交由 AccountManager 保存，凭据不返回 WebUI。
- 非敏感状态：按来源隔离，最多 16 个键，每值 8KiB、总量 64KiB；取消或代次变化拒绝交付／写入。
- 标准能力：选定 WASI clocks、random、io 子集，不提供文件系统、进程、环境变量或原始 sockets。

`Account` Component 的登录调用顺序为 `music.prepare-login`（返回 null）、`music.account/beginLogin`（返回完整挑战），两阶段共用 Store、账号上下文和外层期限；运行时不再给单次调用设 fuel 或期限配额。宿主不解释准备内容，准备失败时不展示占位二维码。

普通调用使用独立 Store；登录挑战最多 180 秒、4 个会话，占查询并发额度。完成、取消、到期、停用或重载释放 Store、连接与敏感状态。普通功能插件使用同一执行配置，不因适配器不同而不同。

执行器不再限制包大小、总线性内存、memory 数量、fuel、调用期限或 JSON 大小。仍保留的是 epoch 让出：每 10 毫秒把 guest 计算交回异步执行器，不计数也不拒绝。连接调度、显式网络期限、凭据、状态与 WASI 子集规则继续有效，详见 [执行与组合模块](../../docs/wasm-adapter-runtime.md)。编译耗时与应用总内存需实测，不构成 guest 的额度。
