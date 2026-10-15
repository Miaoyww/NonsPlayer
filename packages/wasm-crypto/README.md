# 适配器内部 WASM 密码模块

Rust 实现的通用密码原语，通过 `nons:crypto/primitives@1.0.0` WIT 接口提供字节输入与结果。用于构建时组合到适配器 Component 内部，不由宿主注册为密码 Host，不包含音乐平台协议、网络、账号存储或语言运行时桥。

已实现 AES-128/256-GCM（12 字节 nonce、16 字节 tag，密文末尾附 tag）、AES-128/256-CBC（16 字节 IV、PKCS#7）、HKDF-SHA256、SHA256、HMAC-SHA256、MD5 和 RSAES-PKCS1-v1_5 加密。使用 RustCrypto 库；不自行实现算法。nonce/IV 和安全随机数由调用方提供，不能在生产使用确定性探测数据。CBC 不提供认证，调用方必须遵守协议的完整性要求。

RSA 接收 128–512 字节的公开模数、公开指数和每次重新取得的 32 字节安全随机种子，以 ChaCha20 生成填充随机数；明文上限为模数字节数减 11。MD5 与 PKCS#1 v1.5 仅用于既有协议，不作为新协议的算法选择。

每次调用各输入字节长度之和限制为 64KiB；HKDF 输出长度为 1–8160 字节。限制在参数进入 Rust 后检查，WIT 搬运阶段的分配仍受整个 Store 的总线性内存限制。原语不接收 JSON；key 的 Rust 参数用 Zeroizing 包装，不能据此声称 JS 或 ABI 中的所有副本都已清零。

构建核心模块：

```powershell
cargo build --manifest-path packages/wasm-crypto/Cargo.toml --locked --release --target wasm32-unknown-unknown
```

再用 jco 的 `new` 转为 Component，并用 wac `plug` 连接适配器对上述 WIT 的导入。构建工具版本应锁定；本轮组合验证使用 jco 1.37.0、wac 0.12.0、ComponentizeJS 0.23.0。最终产物必须检查真实导入：密码接口应被内部连接消除，不应出现文件系统、网络或进程导入。此包不引入完整 WASI。

原生测试使用 `cargo test --manifest-path packages/wasm-crypto/Cargo.toml --locked`，覆盖公开 WebCrypto 对照向量、RFC 5869、认证失败和参数/输入限制。还需通过实际组合 Component 的同类测试；原生测试不能替代 WASM fuel、内存和运行期限验收。

组合测试源码位于 `tests/component-guest.js`。安装本目录锁定的 npm 工具依赖及 wac 0.12.0 后，执行 `npm run --prefix packages/wasm-crypto build:fixture`；wac 不在 PATH 时通过 `NONS_WAC_BIN` 指定路径。产物位于忽略目录 `.local/wasm-crypto-fixture`，必须在宿主执行器中以适配器预算运行 `call` 并检查全部结果。此 fixture 固定公开 nonce，只用于测试，不是可用音乐适配器。
