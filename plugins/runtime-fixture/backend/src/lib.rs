#[rustfmt::skip]
mod bindings;
use bindings::{nons::plugin::host, Guest};
struct Fixture;
impl Guest for Fixture {
    fn initialize() -> Result<(), String> {
        Ok(())
    }
    fn shutdown() -> Result<(), String> {
        Ok(())
    }
    fn call(method: String, _args: String) -> Result<String, String> {
        match method.as_str() {
            "work" => {
                let mut count = 0u32;
                for _ in 0..50_000_000 {
                    count = core::hint::black_box(count) + 1;
                }
                Ok(count.to_string())
            }
            "loop" => loop {
                core::hint::spin_loop();
            },
            "memory" => {
                let memory = vec![1u8; 80 * 1024 * 1024];
                core::hint::black_box(&memory);
                Ok("null".into())
            }
            "trap" => panic!("fixture trap"),
            "invalid-json" => Ok("not json".into()),
            "host-wait" => host::call("wait", "null"),
            _ => Ok("null".into()),
        }
    }
}
bindings::export!(Fixture with_types_in bindings);
