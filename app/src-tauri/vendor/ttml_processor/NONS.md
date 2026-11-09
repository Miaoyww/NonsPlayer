# TTML Processor

此目录包含 `ttml_processor` 0.4.0 的库源码，来自 [ttml-processor-rs](https://github.com/apoint123/ttml-processor-rs/tree/4b01f5ab8ce040b4b26413c414654d8b9833a293/crates/processor)，提交为 `4b01f5ab8ce040b4b26413c414654d8b9833a293`。许可见 [LICENSE](LICENSE)。

为兼容项目的稳定版 Rust 1.95，`src/amll/to_amll.rs` 中两处尚未稳定的 `Option::map_or_default` 使用等价的 `map_or_else(Default::default, …)`。打包清单只保留库与其依赖，示例、基准测试和上游测试不包含在该目录。
