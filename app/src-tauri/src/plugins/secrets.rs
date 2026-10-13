use crate::model::AppResult;
#[cfg(target_os = "android")]
use crate::platform::android::credentials as keyring;
use serde_json::Value;

// Only this plugin's explicitly named secrets; never access the player's account.
static OPERATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub fn call(
    plugin: &str,
    operation: &str,
    args: &Value,
    database: &super::database::Database,
    active: &std::sync::atomic::AtomicBool,
) -> AppResult<Value> {
    let _guard = OPERATIONS.lock().map_err(|_| "凭据库不可用")?;
    if !active.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("插件已停用".into());
    }
    let key = args
        .get("key")
        .and_then(Value::as_str)
        .ok_or("缺少凭据键")?;
    if !super::manifest::valid_id(key) {
        return Err("凭据键无效".into());
    }
    let entry = keyring::Entry::new("NonsPlayer.plugins", &format!("{plugin}:{key}"))
        .map_err(|_| "系统凭据库不可用")?;
    match operation {
        "secrets.get" => match entry.get_password() {
            Ok(value) => Ok(Value::String(value)),
            Err(keyring::Error::NoEntry) => Ok(Value::Null),
            Err(_) => Err("读取系统凭据失败".into()),
        },
        "secrets.set" => {
            let value = args
                .get("value")
                .and_then(Value::as_str)
                .ok_or("凭据必须是文本")?;
            if value.len() > 16384 {
                return Err("凭据超过 16KiB".into());
            }
            let keys = database.secret_keys(plugin)?;
            if keys.len() >= 16 && !keys.iter().any(|k| k == key) {
                return Err("每插件最多保存 16 个凭据".into());
            }
            database.secret_key(plugin, key, true)?;
            entry.set_password(value).map_err(|_| "保存系统凭据失败")?;
            Ok(Value::Null)
        }
        "secrets.delete" => {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => (),
                Err(_) => return Err("删除系统凭据失败".into()),
            }
            database.secret_key(plugin, key, false)?;
            Ok(Value::Null)
        }
        _ => Err("未知凭据操作".into()),
    }
}

pub fn clear(plugin: &str, database: &super::database::Database) -> AppResult<()> {
    let _guard = OPERATIONS.lock().map_err(|_| "凭据库不可用")?;
    for key in database.secret_keys(plugin)? {
        let entry = keyring::Entry::new("NonsPlayer.plugins", &format!("{plugin}:{key}"))
            .map_err(|_| "凭据库不可用")?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => (),
            Err(_) => return Err("清理插件凭据失败".into()),
        }
        database.secret_key(plugin, &key, false)?;
    }
    Ok(())
}
