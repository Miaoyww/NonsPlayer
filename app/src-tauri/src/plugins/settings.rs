use super::*;

#[tauri::command]
pub async fn plugin_settings(
    id: String,
    generation: u64,
    operation: String,
    args: serde_json::Value,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> AppResult<serde_json::Value> {
    #[cfg(desktop)]
    use tauri_plugin_dialog::DialogExt;
    if serde_json::to_vec(&args).map_err(|e| e.to_string())?.len() > 65536 {
        return Err("插件设置参数超过 64KiB".into());
    }
    let manager = manager.inner().clone();
    let validate = || -> AppResult<Manifest> {
        let records = manager.lock()?;
        let record = records
            .get(&id)
            .filter(|r| r.descriptor.generation == generation)
            .ok_or("插件已变化，请重新打开配置页")?;
        Ok(record.descriptor.manifest.clone())
    };
    let manifest = validate()?;
    if operation == "grant" {
        #[cfg(target_os = "android")]
        return Err("Android 试用版暂不支持插件外部目录授权".into());
        #[cfg(desktop)]
        {
            if !manifest.permissions.iter().any(|p| p == "files:selected") {
                return Err("插件未申请外部目录权限".into());
            }
            let writable = args
                .get("writable")
                .and_then(serde_json::Value::as_bool)
                .ok_or("缺少目录授权模式")?;
            let app = manager.app.clone();
            let path = tauri::async_runtime::spawn_blocking(move || {
                app.dialog()
                    .file()
                    .set_title("选择授权给插件的目录")
                    .blocking_pick_folder()
            })
            .await
            .map_err(|e| e.to_string())?;
            if let Some(path) = path {
                let _operation = manager.operations.lock().await;
                validate()?;
                let path = path.into_path().map_err(|e| e.to_string())?;
                let files = manager.files.clone();
                let id = id.clone();
                let grant =
                    tauri::async_runtime::spawn_blocking(move || files.grant(&id, path, writable))
                        .await
                        .map_err(|e| e.to_string())??;
                if let Some(key) = args.get("key").and_then(serde_json::Value::as_str) {
                    let result = (|| -> AppResult<()> {
                        if !manifest.permissions.iter().any(|p| p == "config") {
                            return Err("插件缺少配置权限".into());
                        }
                        let definition = manager.configurations.definition(&manifest.id)?;
                        if !definition.fields.iter().any(|f| {
                            f.key == key
                                && f.editor
                                    .as_ref()
                                    .is_some_and(|e| e.kind == "authorizedDirectory")
                        }) {
                            return Err("配置项不是授权目录选择器".into());
                        }
                        let revision = args
                            .get("revision")
                            .and_then(serde_json::Value::as_u64)
                            .ok_or("缺少配置修订号")?;
                        let mut patch = serde_json::Map::new();
                        patch.insert(key.to_string(), serde_json::Value::String(grant.id.clone()));
                        manager
                            .configurations
                            .change(&manifest.id, revision, Some(patch), None)?;
                        Ok(())
                    })();
                    if result.is_err() {
                        manager.files.revoke(&manifest.id, Some(&grant.id))?;
                    }
                    result?;
                }
            }
        }
    } else {
        let _operation = manager.operations.lock().await;
        validate()?;
        match operation.as_str() {
            "get" => {}
            "open-data" => {
                if !manifest.permissions.iter().any(|p| p == "files:data") {
                    return Err("插件未申请专属目录权限".into());
                }
                let files = manager.files.clone();
                let id = id.clone();
                let app = manager.app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let path = files.data_directory(&id)?;
                    app.opener()
                        .open_path(path.to_string_lossy(), None::<&str>)
                        .map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())??;
            }
            "update" | "reset" => {
                if !manifest.permissions.iter().any(|p| p == "config") {
                    return Err("插件未申请配置权限".into());
                }
                let revision = args
                    .get("revision")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("缺少配置修订号")?;
                let patch = if operation == "update" {
                    Some(
                        args.get("patch")
                            .and_then(serde_json::Value::as_object)
                            .ok_or("配置修改必须是对象")?
                            .clone(),
                    )
                } else {
                    None
                };
                let keys = args
                    .get("keys")
                    .map(|v| {
                        serde_json::from_value(v.clone())
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "plugins.settings.reset.parse_keys",
                                    error,
                                );
                            })
                            .map_err(|_| "配置键列表无效")
                    })
                    .transpose()?;
                manager.configurations.change(&id, revision, patch, keys)?;
            }
            "revoke" => {
                let root = args
                    .get("root")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("缺少目录标识")?;
                let files = manager.files.clone();
                let id = id.clone();
                let root = root.to_string();
                tauri::async_runtime::spawn_blocking(move || files.revoke(&id, Some(&root)))
                    .await
                    .map_err(|e| e.to_string())??;
            }
            _ => return Err("未知插件设置操作".into()),
        }
    }
    validate()?;
    Ok(
        serde_json::json!({"definition":manager.configurations.definition(&id).inspect_err(|error| { crate::infrastructure::diagnostics::warn("plugins.settings.read_definition", error); }).ok(),"snapshot":if manifest.configuration.is_some(){Some(manager.configurations.snapshot(&id)?)}else{None},"grants":manager.files.grants(&id)?}),
    )
}
