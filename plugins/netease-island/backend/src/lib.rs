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
        let config: Value =
            serde_json::from_str(&host::call("config.get", "{}")?).map_err(|_| "无效配置")?;
        if config["values"]["detectLinks"] == false {
            return Ok("null".into());
        }
        let args: Value = serde_json::from_str(&args).map_err(|_| "无效参数")?;
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
                if let Ok(url) = links::resolve(candidate, &mut remaining, |request| {
                    serde_json::from_str(&host::call("http.request", &request.to_string())?)
                        .map_err(|_| "无效 HTTP 响应".into())
                }) {
                    id = links::song_id(&url);
                    if id.is_some() {
                        break;
                    }
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
            let song: Value = serde_json::from_str(&song).map_err(|_| "无效歌曲信息")?;
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
