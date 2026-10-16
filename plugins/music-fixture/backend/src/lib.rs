#[rustfmt::skip]
mod bindings;
use bindings::{nons::plugin::host, Guest};
use serde_json::{json, Value};
struct Fixture;
fn ok(data: Value) -> Result<String, String> {
    Ok(json!({"status":"ok","data":data}).to_string())
}
fn error(code: &str) -> Result<String, String> {
    Ok(json!({"status":"error","data":{"code":code,"retryAfterMs":null}}).to_string())
}
fn track(reference: Value) -> Value {
    json!({"reference":reference,"title":"Fixture","artist":"Artist","album":"Album",
        "aliases":[],"artists":[],"albumReference":null,"durationMs":1000,"cover":"","associations":[]})
}
impl Guest for Fixture {
    fn initialize() -> Result<(), String> {
        Ok(())
    }
    fn shutdown() -> Result<(), String> {
        Ok(())
    }
    fn call(method: String, args: String) -> Result<String, String> {
        if method == "music.descriptor" {
            return ok(
                json!({"source":"fixture-radio","displayName":"外部测试来源","description":"验证通用音源能力的测试平台","platforms":["fixture-radio","netease"],"contractVersion":1,"capabilities":["search","browse"]}),
            );
        }
        let args: Value = serde_json::from_str(&args).map_err(|_| "bad args")?;
        let request = &args["request"];
        match method.as_str() {
            "music.read-track" => {
                match request["id"].as_str().unwrap_or("") {
                    "loop" => loop {
                        core::hint::spin_loop();
                    },
                    "memory" => {
                        let bytes = vec![1u8; 320 * 1024 * 1024];
                        core::hint::black_box(bytes);
                    }
                    "trap" => panic!("fixture trap"),
                    "invalid-json" => return Ok("invalid".into()),
                    "oversized" => return Ok("x".repeat(65 * 1024)),
                    "wrong-source" => {
                        return ok(track(
                            json!({"source":"netease","kind":"track","id":"same-id"}),
                        ))
                    }
                    "host-denied" => {
                        return match host::call("account.read", "{\"source\":\"netease\"}") {
                            Err(_) => error("permissionDenied"),
                            Ok(_) => error("internal"),
                        }
                    }
                    _ => (),
                }
                ok(track(request.clone()))
            }
            "music.business" if request["operation"] == "search" => {
                let (id, next) = match request["page"]["cursor"].as_str() {
                    None => ("广播:α/first", json!("continuation:β/page")),
                    Some("continuation:β/page") => ("广播:β/last", Value::Null),
                    _ => return error("invalidData"),
                };
                ok(
                    json!({"type":"tracks","data":{"items":[track(json!({"source":"fixture-radio","kind":"track","id":id}))],"nextCursor":next,"total":null,"description":null}}),
                )
            }
            "music.resolve-playback" => {
                let id = request["track"]["id"].as_str().unwrap_or("");
                ok(
                    json!({"metadata":{"actualQuality":"fixture","extent": if id == "preview" { json!({"kind":"preview","startMs":0,"endMs":1000}) } else { json!({"kind":"full"}) },"expiresAtMs": if id == "expired" { json!(1) } else { Value::Null }},"url":"https://example.com/fixture.wav?private=fixture","headers": if id == "headers" { json!([["Authorization","fixture"]]) } else { json!([]) }}),
                )
            }
            "music.resolve-resource" => {
                if request["track"]["reference"]["source"] != "netease"
                    || args["session"]["source"] != "fixture-radio"
                {
                    return error("invalidData");
                }
                ok(
                    json!({"metadata":{"actualQuality":"fixture-cross","extent":{"kind":"full"},"expiresAtMs":null},"url":"https://example.com/alternate.wav","headers":[]}),
                )
            }
            _ => error("unsupported"),
        }
    }
}
bindings::export!(Fixture with_types_in bindings);
