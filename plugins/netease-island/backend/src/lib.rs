#[rustfmt::skip]
#[cfg(not(test))]
mod bindings;
mod links;
#[cfg(not(test))]
use bindings::{nons::plugin::host, Guest};
#[cfg(not(test))]
use serde_json::{json, Value};

#[cfg(not(test))]
struct Island;
#[cfg(not(test))]
fn report(operation: &str, message: &str) {
    // Reporting failure must not turn a recoverable link failure into a plugin fault.
    let _ = host::call(
        "diagnostic.report",
        &json!({"operation":operation,"message":message.chars().take(1024).collect::<String>()})
            .to_string(),
    );
}
#[cfg(not(test))]
fn parse_json(operation: &str, message: &str, text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|error| {
        report(operation, &error.to_string());
        message.into()
    })
}
#[cfg(not(test))]
impl Guest for Island {
    fn initialize() -> Result<(), String> {
        host::call("config.get", "{}")?;
        Ok(())
    }
    fn shutdown() -> Result<(), String> {
        Ok(())
    }
    fn call(method: String, args: String) -> Result<String, String> {
        if method == "event:config-changed" {
            return Ok("null".into());
        }
        if method != "event:clipboard-text" {
            return Err("未知方法".into());
        }
        let config = parse_json("config.parse", "无效配置", &host::call("config.get", "{}")?)?;
        if config["values"]["detectLinks"] == false {
            return Ok("null".into());
        }
        let args = parse_json("clipboard.parse", "无效参数", &args)?;
        let text = args
            .get("text")
            .and_then(Value::as_str)
            .ok_or("缺少剪贴板文本")?;
        if text.len() > 16 * 1024 {
            return Ok("null".into());
        }
        let candidates = links::candidates(text);
        // Prefer an already usable song link before attempting short links.
        let mut id = candidates.iter().find_map(links::song_id);
        let mut remaining = 3;
        if id.is_none() {
            for candidate in candidates {
                match links::resolve(candidate, &mut remaining, |request| {
                    parse_json(
                        "http.parse",
                        "无效 HTTP 响应",
                        &host::call("http.request", &request.to_string())?,
                    )
                }) {
                    Ok(url) => {
                        id = links::song_id(&url);
                        if id.is_some() {
                            break;
                        }
                    }
                    Err(error) => report("link.resolve", &error),
                }
            }
        }
        let Some(id) = id else {
            return Ok("null".into());
        };
        // Network failures are recoverable and must not fault this plugin.
        if let Ok(song) = host::call(
            "music.read-track",
            &json!({"reference":{"source":"netease","kind":"track","id":id.to_string()}})
                .to_string(),
        ) {
            let song = parse_json("track.parse", "无效歌曲信息", &song)?;
            host::call(
                "events.emit",
                &json!({"event":"song-detected","payload":song}).to_string(),
            )?;
        }
        Ok("null".into())
    }
}
#[cfg(not(test))]
bindings::export!(Island with_types_in bindings);
