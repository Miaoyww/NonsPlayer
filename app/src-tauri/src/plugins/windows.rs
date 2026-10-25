//! Plugin-owned windows load a declared export through the local application entry.
use super::capability::Context;
use crate::model::AppResult;
#[cfg(windows)]
use serde::Deserialize;
#[cfg(windows)]
use serde_json::json;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct Windows {
    entries: Arc<Mutex<BTreeMap<String, Entry>>>,
}

struct Entry {
    plugin: String,
    generation: u64,
    #[cfg(windows)]
    label: String,
    #[cfg(windows)]
    window: Option<tauri::WebviewWindow>,
}

#[cfg(windows)]
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Options {
    id: String,
    title: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    x: Option<f64>,
    y: Option<f64>,
    transparent: Option<bool>,
    decorations: Option<bool>,
    always_on_top: Option<bool>,
    ignore_cursor_events: Option<bool>,
}

#[cfg(windows)]
impl Options {
    fn parse(value: &Value) -> AppResult<Self> {
        let options: Self =
            serde_json::from_value(value.clone()).map_err(|_| "插件窗口参数无效")?;
        if !super::manifest::valid_id(&options.id)
            || options
                .title
                .as_ref()
                .is_some_and(|title| title.len() > 512 || title.contains('\0'))
            || [options.width, options.height]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || !(32.0..=8192.0).contains(&v))
            || [options.x, options.y]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 100_000.0)
            || options.width.is_some() != options.height.is_some()
            || options.x.is_some() != options.y.is_some()
        {
            return Err("插件窗口尺寸、位置或标识无效".into());
        }
        Ok(options)
    }
}

impl Windows {
    pub async fn call(&self, context: &Context, operation: &str, args: &Value) -> AppResult<Value> {
        context.check(Some("window:manage"))?;
        context.check(Some("ui"))?;
        #[cfg(not(windows))]
        {
            let _ = (operation, args, &context.window_exports);
            Err("插件独立窗口当前仅支持 Windows".into())
        }
        #[cfg(windows)]
        {
            let options = Options::parse(args)?;
            if !context.window_exports.contains_key(&options.id) {
                return Err("插件未声明此窗口".into());
            }
            let label = format!(
                "plugin-window-{}-{}-{}",
                context.id, context.generation, options.id
            );
            if operation == "windows.open" {
                self.open(context, options, label).await
            } else {
                let window = self
                    .entries
                    .lock()
                    .map_err(|_| "插件窗口注册表不可用")?
                    .get(&label)
                    .and_then(|entry| entry.window.clone())
                    .ok_or("插件窗口尚未打开")?;
                match operation {
                    "windows.update" => {
                        if options.transparent.is_some() || options.decorations.is_some() {
                            return Err("透明和窗口边框只能在创建时设置".into());
                        }
                        if let Some(title) = options.title {
                            window.set_title(&title).map_err(|e| e.to_string())?;
                        }
                        if let (Some(width), Some(height)) = (options.width, options.height) {
                            window
                                .set_size(tauri::LogicalSize::new(width, height))
                                .map_err(|e| e.to_string())?;
                        }
                        if let (Some(x), Some(y)) = (options.x, options.y) {
                            window
                                .set_position(tauri::LogicalPosition::new(x, y))
                                .map_err(|e| e.to_string())?;
                        }
                        if let Some(value) = options.always_on_top {
                            window.set_always_on_top(value).map_err(|e| e.to_string())?;
                        }
                        if let Some(value) = options.ignore_cursor_events {
                            window
                                .set_ignore_cursor_events(value)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    "windows.show" => window.show().map_err(|e| e.to_string())?,
                    "windows.hide" => window.hide().map_err(|e| e.to_string())?,
                    "windows.start-dragging" => {
                        window.start_dragging().map_err(|e| e.to_string())?
                    }
                    "windows.close" => {
                        window.destroy().map_err(|e| e.to_string())?;
                        let mut entries =
                            self.entries.lock().map_err(|_| "插件窗口注册表不可用")?;
                        if entries
                            .get(&label)
                            .is_some_and(|entry| entry.label == window.label())
                        {
                            entries.remove(&label);
                        }
                    }
                    _ => return Err("未知插件窗口操作".into()),
                }
                context.check(Some("window:manage"))?;
                Ok(Value::Null)
            }
        }
    }

    #[cfg(windows)]
    async fn open(&self, context: &Context, options: Options, label: String) -> AppResult<Value> {
        {
            let mut entries = self.entries.lock().map_err(|_| "插件窗口注册表不可用")?;
            if let Some(entry) = entries.get(&label) {
                if let Some(window) = &entry.window {
                    window.show().map_err(|e| e.to_string())?;
                    return Ok(json!({"id":options.id,"label":window.label()}));
                }
                return Err("插件窗口正在创建".into());
            }
            if entries.len() >= 16
                || entries
                    .values()
                    .filter(|entry| entry.plugin == context.id)
                    .count()
                    >= 4
            {
                return Err("插件窗口数量已达上限".into());
            }
            entries.insert(
                label.clone(),
                Entry {
                    plugin: context.id.clone(),
                    generation: context.generation,
                    label: format!("{label}-{}", uuid::Uuid::new_v4().simple()),
                    window: None,
                },
            );
        }
        let context = context.clone();
        let entries = self.entries.clone();
        let task_label = label.clone();
        let result = tauri::async_runtime::spawn_blocking(move || -> AppResult<Value> {
            let native_label = entries
                .lock()
                .map_err(|_| "插件窗口注册表不可用")?
                .get(&task_label)
                .ok_or("插件窗口已失效")?
                .label
                .clone();
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            query
                .append_pair("pluginWindow", &context.id)
                .append_pair("generation", &context.generation.to_string())
                .append_pair("window", &options.id);
            let query = query.finish();
            let allowed_query = query.clone();
            let dev_origin = tauri::is_dev()
                .then(|| {
                    context
                        .app
                        .config()
                        .build
                        .dev_url
                        .as_ref()
                        .map(|url| url.origin().ascii_serialization())
                })
                .flatten();
            let mut builder = tauri::WebviewWindowBuilder::new(
                &context.app,
                &native_label,
                tauri::WebviewUrl::App(format!("index.html?{query}").into()),
            )
            .title(options.title.as_deref().unwrap_or(&context.id))
            .inner_size(
                options.width.unwrap_or(640.0),
                options.height.unwrap_or(360.0),
            )
            .transparent(options.transparent.unwrap_or(false))
            .decorations(options.decorations.unwrap_or(true))
            .always_on_top(options.always_on_top.unwrap_or(false))
            .visible(false)
            .on_navigation(move |url| {
                let local = matches!(
                    url.origin().ascii_serialization().as_str(),
                    "http://tauri.localhost" | "https://tauri.localhost" | "tauri://localhost"
                ) || dev_origin
                    .as_ref()
                    .is_some_and(|origin| *origin == url.origin().ascii_serialization());
                local
                    && matches!(url.path(), "/" | "/index.html")
                    && url.query() == Some(allowed_query.as_str())
            })
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
            if options.transparent == Some(true) {
                builder = builder.background_color(tauri::webview::Color(0, 0, 0, 0));
            }
            if let (Some(x), Some(y)) = (options.x, options.y) {
                builder = builder.position(x, y);
            }
            if let Some(data) = crate::infrastructure::directories::portable() {
                builder = builder.data_directory(data.join("webview"));
            }
            let window = builder.build().map_err(|e| e.to_string())?;
            let closed_entries = Arc::downgrade(&entries);
            let closed_key = task_label.clone();
            let closed_label = native_label.clone();
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    if let Some(entries) = closed_entries.upgrade() {
                        if let Ok(mut entries) = entries.lock() {
                            if entries
                                .get(&closed_key)
                                .is_some_and(|entry| entry.label == closed_label)
                            {
                                entries.remove(&closed_key);
                            }
                        }
                    }
                }
            });
            let finish = (|| -> AppResult<()> {
                let mut entries = entries.lock().map_err(|_| "插件窗口注册表不可用")?;
                context.check(Some("window:manage"))?;
                let entry = entries.get_mut(&task_label).ok_or("插件窗口已失效")?;
                window
                    .set_ignore_cursor_events(options.ignore_cursor_events.unwrap_or(false))
                    .map_err(|e| e.to_string())?;
                entry.window = Some(window.clone());
                window.show().map_err(|e| e.to_string())?;
                Ok(())
            })();
            if let Err(error) = finish {
                let _ = window.destroy();
                return Err(error);
            }
            Ok(json!({"id":options.id,"label":native_label}))
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|value| value);
        if result.is_err() {
            self.entries
                .lock()
                .map_err(|_| "插件窗口注册表不可用")?
                .remove(&label);
        }
        result
    }

    pub fn revoke(&self, plugin: &str, generation: u64) {
        let removed = if let Ok(mut entries) = self.entries.lock() {
            let labels: Vec<_> = entries
                .iter()
                .filter(|(_, entry)| entry.plugin == plugin && entry.generation == generation)
                .map(|(label, _)| label.clone())
                .collect();
            labels
                .into_iter()
                .filter_map(|label| entries.remove(&label))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for entry in removed {
            #[cfg(windows)]
            if let Some(window) = entry.window {
                if let Err(error) = window.destroy() {
                    crate::infrastructure::diagnostics::warn("plugins.windows.revoke", error);
                }
            }
            #[cfg(not(windows))]
            let _ = entry;
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn rejects_unbounded_or_partial_geometry_and_unknown_fields() {
        for value in [
            json!({"id":"test","width":-1,"height":100}),
            json!({"id":"test","width":100}),
            json!({"id":"test","x":0}),
            json!({"id":"test","url":"https://example.com"}),
            json!({"id":"../main"}),
        ] {
            assert!(Options::parse(&value).is_err());
        }
        assert!(Options::parse(
            &json!({"id":"lyrics","width":800,"height":100,"x":-1200,"y":200,"transparent":true})
        )
        .is_ok());
    }
}
