use super::{
    database::Database,
    manifest::{checked_file, Manifest},
};
use crate::model::AppResult;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub version: u32,
    pub page: Page,
    pub fields: Vec<Field>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum Page {
    Generated,
    Custom {
        #[serde(rename = "pageId")]
        page_id: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub title: String,
    pub description: String,
    pub schema: Value,
    pub default: Value,
    #[serde(default = "live")]
    pub apply: String,
    pub group: Option<String>,
    pub editor: Option<Editor>,
}
fn live() -> String {
    "live".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Editor {
    pub kind: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub unit: Option<String>,
    #[serde(default)]
    pub options: Vec<Choice>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub label: String,
    pub value: Value,
}
pub struct Compiled {
    pub definition: Definition,
    validators: BTreeMap<String, jsonschema::Validator>,
}
impl Compiled {
    pub fn read(root: &Path, manifest: &Manifest) -> AppResult<Option<Arc<Self>>> {
        let Some(path) = &manifest.configuration else {
            return Ok(None);
        };
        let file = checked_file(root, path)?;
        if file.metadata().map_err(|e| e.to_string())?.len() > 65536 {
            return Err("配置声明超过 64KiB".into());
        }
        let definition: Definition =
            serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
                .map_err(|e| format!("配置声明无效：{e}"))?;
        if !definition.fields.is_empty() && !manifest.permissions.iter().any(|p| p == "config") {
            return Err("统一配置字段需要 config 权限".into());
        }
        if let Page::Custom { page_id } = &definition.page {
            if !manifest.contributes.pages.iter().any(|p| &p.id == page_id) {
                return Err("自定义配置页未注册".into());
            }
        }
        Ok(Some(Arc::new(Self::compile(definition)?)))
    }
    pub fn compile(definition: Definition) -> AppResult<Self> {
        if definition.version != 1 || definition.fields.len() > 128 {
            return Err("不支持的配置声明版本或配置项过多".into());
        }
        let mut keys = HashSet::new();
        let mut validators = BTreeMap::new();
        let mut defaults = Map::new();
        for field in &definition.fields {
            if !field.schema.is_object() {
                return Err("配置校验规则必须是 JSON Schema 对象".into());
            }
            if field.key.is_empty()
                || field.key.len() > 128
                || !keys.insert(&field.key)
                || field.title.trim().is_empty()
                || field.title.len() > 128
                || field.description.len() > 2048
                || !matches!(field.apply.as_str(), "live" | "reload")
            {
                return Err("配置键、标题或生效方式无效".into());
            }
            check_refs(&field.schema)?;
            let validator = jsonschema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .build(&field.schema)
                .map_err(|e| format!("{} 校验规则无效：{e}", field.key))?;
            if !validator.is_valid(&field.default) {
                return Err(format!("{} 默认值不符合校验规则", field.key));
            }
            if let Some(editor) = &field.editor {
                let kind = editor.kind.as_str();
                let expected = match kind {
                    "text"
                    | "textarea"
                    | "color"
                    | "file"
                    | "directory"
                    | "authorizedDirectory" => "string",
                    "number" | "slider" => "number",
                    "switch" => "boolean",
                    "select" => "enum",
                    "multiselect" => "array",
                    _ => return Err("未知配置编辑控件".into()),
                };
                let valid = match expected {
                    "string" => field.default.is_string(),
                    "number" => field.default.is_number(),
                    "boolean" => field.default.is_boolean(),
                    "array" => field.default.is_array(),
                    _ => true,
                };
                if !valid {
                    return Err(format!("{} 控件与默认值类型不一致", field.key));
                }
                if editor.step.is_some_and(|v| !v.is_finite() || v <= 0.0) {
                    return Err("控件步长必须大于零".into());
                }
                if kind == "slider"
                    && !matches!((editor.min, editor.max), (Some(a),Some(b)) if a.is_finite() && b.is_finite() && a < b)
                {
                    return Err("slider 必须声明有效范围".into());
                }
                if kind == "slider"
                    && (field
                        .default
                        .as_f64()
                        .is_some_and(|v| v < editor.min.unwrap() || v > editor.max.unwrap())
                        || !validator.is_valid(&serde_json::json!(editor.min.unwrap()))
                        || !validator.is_valid(&serde_json::json!(editor.max.unwrap())))
                {
                    return Err("slider 范围与配置规则或默认值不一致".into());
                }
                if field.schema.get("type").and_then(Value::as_str) == Some("integer")
                    && editor.step.is_some_and(|step| step.fract() != 0.0)
                {
                    return Err("整数配置的步长必须是整数".into());
                }
                if matches!(kind, "select" | "multiselect")
                    && (editor.options.is_empty() || editor.options.len() > 128)
                {
                    return Err("选择控件必须声明选项".into());
                }
                for option in &editor.options {
                    let candidate = if kind == "multiselect" {
                        Value::Array(vec![option.value.clone()])
                    } else {
                        option.value.clone()
                    };
                    let option_valid = if kind == "multiselect" {
                        let mut schema = field.schema.clone();
                        if let Some(object) = schema.as_object_mut() {
                            object.remove("minItems");
                            object.remove("maxItems");
                        }
                        jsonschema::options()
                            .with_draft(jsonschema::Draft::Draft202012)
                            .build(&schema)
                            .map_err(|e| e.to_string())?
                            .is_valid(&candidate)
                    } else {
                        validator.is_valid(&candidate)
                    };
                    if option.label.is_empty() || !option_valid {
                        return Err("控件选项不符合配置规则".into());
                    }
                }
                if matches!(kind, "select" | "multiselect") {
                    let values = if kind == "multiselect" {
                        field.default.as_array().unwrap().clone()
                    } else {
                        vec![field.default.clone()]
                    };
                    if values
                        .iter()
                        .any(|v| !editor.options.iter().any(|o| &o.value == v))
                    {
                        return Err("默认值必须在控件选项中".into());
                    }
                }
            }
            defaults.insert(field.key.clone(), field.default.clone());
            validators.insert(field.key.clone(), validator);
        }
        if serde_json::to_vec(&defaults)
            .map_err(|e| e.to_string())?
            .len()
            > 65536
        {
            return Err("配置默认值超过 64KiB".into());
        }
        Ok(Self {
            definition,
            validators,
        })
    }
    fn effective(
        &self,
        raw: &Map<String, Value>,
    ) -> (Map<String, Value>, BTreeMap<String, String>) {
        let mut values = Map::new();
        let mut diagnostics = BTreeMap::new();
        for field in &self.definition.fields {
            let value = match raw.get(&field.key) {
                Some(value) if self.validators[&field.key].is_valid(value) => value,
                Some(_) => {
                    diagnostics.insert(
                        field.key.clone(),
                        "已保存的值不符合当前规则，暂用默认值；原值仍保留，请修改或恢复默认。"
                            .into(),
                    );
                    &field.default
                }
                None => &field.default,
            };
            values.insert(field.key.clone(), value.clone());
        }
        (values, diagnostics)
    }
}
fn check_refs(value: &Value) -> AppResult<()> {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if matches!(key.as_str(), "$ref" | "$dynamicRef")
                    && value.as_str().is_some_and(|v| !v.starts_with('#'))
                {
                    return Err("配置规则仅允许内部引用".into());
                }
                if key == "$id" && value.is_string() {
                    return Err("配置规则不允许外部资源标识".into());
                }
                check_refs(value)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                check_refs(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub values: Map<String, Value>,
    pub revision: u64,
    pub diagnostics: BTreeMap<String, String>,
    pub pending_reload: bool,
}
struct Entry {
    compiled: Arc<Compiled>,
    baseline: Option<Map<String, Value>>,
}
pub struct Configurations {
    database: Arc<Database>,
    entries: Mutex<BTreeMap<String, Entry>>,
    pub changes: tokio::sync::broadcast::Sender<String>,
}
impl Configurations {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            entries: Default::default(),
            changes: tokio::sync::broadcast::channel(64).0,
        }
    }
    pub fn register(&self, id: &str, compiled: Option<Arc<Compiled>>) -> AppResult<()> {
        let mut entries = self
            .entries
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "配置管理器不可用")?;
        entries.remove(id);
        if let Some(compiled) = compiled {
            entries.insert(
                id.into(),
                Entry {
                    compiled,
                    baseline: None,
                },
            );
        }
        Ok(())
    }
    pub fn definition(&self, id: &str) -> AppResult<Definition> {
        Ok(self
            .entries
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "配置管理器不可用")?
            .get(id)
            .ok_or("插件未声明配置")?
            .compiled
            .definition
            .clone())
    }
    pub fn snapshot(&self, id: &str) -> AppResult<Snapshot> {
        let entries = self
            .entries
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "配置管理器不可用")?;
        let entry = entries.get(id).ok_or("插件未声明配置")?;
        let (revision, raw) = self.database.config_read(id)?;
        let (values, diagnostics) = entry.compiled.effective(&raw);
        let pending_reload = entry.baseline.as_ref().is_some_and(|old| {
            entry.compiled.definition.fields.iter().any(|field| {
                field.apply == "reload" && old.get(&field.key) != values.get(&field.key)
            })
        });
        Ok(Snapshot {
            values,
            revision,
            diagnostics,
            pending_reload,
        })
    }
    pub fn loaded(&self, id: &str) -> AppResult<()> {
        if self.definition(id).is_err() {
            return Ok(());
        }
        let values = self.snapshot(id)?.values;
        if let Some(entry) = self
            .entries
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "配置管理器不可用")?
            .get_mut(id)
        {
            entry.baseline = Some(values);
        }
        Ok(())
    }
    pub fn unloaded(&self, id: &str) {
        if let Ok(mut entries) = (self.entries.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
        }) {
            if let Some(entry) = entries.get_mut(id) {
                entry.baseline = None;
            }
        }
    }
    pub fn change(
        &self,
        id: &str,
        revision: u64,
        patch: Option<Map<String, Value>>,
        reset: Option<Vec<String>>,
    ) -> AppResult<Snapshot> {
        {
            let entries = self
                .entries
                .lock()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "配置管理器不可用")?;
            let compiled = &entries.get(id).ok_or("插件未声明配置")?.compiled;
            self.database.config_change(id, revision, |raw| {
                if let Some(patch) = patch {
                    for (key, value) in patch {
                        let validator = compiled
                            .validators
                            .get(&key)
                            .ok_or_else(|| format!("未知配置键：{key}"))?;
                        validator
                            .validate(&value)
                            .map_err(|error| format!("{key}：{error}"))?;
                        raw.insert(key, value);
                    }
                } else {
                    if reset.is_none() {
                        raw.clear();
                    }
                    let keys = reset.unwrap_or_else(|| {
                        compiled
                            .definition
                            .fields
                            .iter()
                            .map(|f| f.key.clone())
                            .collect()
                    });
                    for key in keys {
                        if !compiled.validators.contains_key(&key) {
                            return Err(format!("未知配置键：{key}"));
                        }
                        raw.remove(&key);
                    }
                }
                let (values, diagnostics) = compiled.effective(raw);
                let envelope = Snapshot {
                    values,
                    diagnostics,
                    revision: revision + 1,
                    pending_reload: false,
                };
                if serde_json::to_vec(&envelope)
                    .map_err(|e| e.to_string())?
                    .len()
                    > 65536
                {
                    return Err("配置超过 64KiB".into());
                }
                Ok(())
            })?;
        }
        let snapshot = self.snapshot(id)?;
        let _ = (self.changes.send(id.into())).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("self.changes.send", error);
        });
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_configuration_compiles_quality_and_authorized_directory_fields() {
        let definition: Definition = serde_json::from_str(include_str!(
            "../../../../plugins/netease-download/configuration.json"
        ))
        .unwrap();
        let compiled = Compiled::compile(definition).unwrap();
        let (values, diagnostics) = compiled.effective(&Map::new());
        assert!(diagnostics.is_empty());
        assert_eq!(values["quality"], "lossless");
        assert_eq!(values["directory"], "");
    }
    use serde_json::json;
    fn definition() -> Definition {
        serde_json::from_value(json!({"version":1,"page":{"mode":"generated"},"fields":[{"key":"duration","title":"时长","description":"秒","schema":{"type":"number","minimum":1,"maximum":30},"default":8,"apply":"reload","editor":{"kind":"slider","min":1,"max":30}},{"key":"rules","title":"规则","description":"自定义","schema":{"type":"object","required":["names"],"properties":{"names":{"type":"array","items":{"type":"string"}}}},"default":{"names":[]}}]})).unwrap()
    }
    #[test]
    fn atomic_revision_defaults_and_retained_invalid_values() {
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let configs = Configurations::new(db.clone());
        configs
            .register(
                "one",
                Some(Arc::new(Compiled::compile(definition()).unwrap())),
            )
            .unwrap();
        assert_eq!(configs.snapshot("one").unwrap().values["duration"], 8);
        configs.loaded("one").unwrap();
        assert!(configs
            .change(
                "one",
                0,
                Some(serde_json::from_value(json!({"duration":12,"rules":{}})).unwrap()),
                None
            )
            .is_err());
        assert_eq!(configs.snapshot("one").unwrap().revision, 0);
        assert!(configs.change("other", 0, Some(Map::new()), None).is_err());
        assert!(
            configs
                .change(
                    "one",
                    0,
                    Some(serde_json::from_value(json!({"duration":12})).unwrap()),
                    None
                )
                .unwrap()
                .pending_reload
        );
        assert!(configs.change("one", 0, Some(Map::new()), None).is_err());
        assert!(
            !configs
                .change("one", 1, None, Some(vec!["duration".into()]))
                .unwrap()
                .pending_reload
        );
        db.config_change("one", 2, |raw| {
            raw.insert("duration".into(), json!(99));
            Ok(())
        })
        .unwrap();
        assert_eq!(configs.snapshot("one").unwrap().values["duration"], 8);
        assert!(configs
            .snapshot("one")
            .unwrap()
            .diagnostics
            .contains_key("duration"));
        assert_eq!(db.config_read("one").unwrap().1["duration"], 99);
        configs.unloaded("one");
        configs
            .register(
                "one",
                Some(Arc::new(Compiled::compile(definition()).unwrap())),
            )
            .unwrap();
        assert!(configs
            .snapshot("one")
            .unwrap()
            .diagnostics
            .contains_key("duration"));
        db.remove("one").unwrap();
        assert_eq!(configs.snapshot("one").unwrap().revision, 0);
    }
    #[test]
    fn declaration_rejects_bad_defaults_editors_and_external_refs() {
        let mut d = definition();
        d.fields[0].default = json!(100);
        assert!(Compiled::compile(d).is_err());
        let mut d = definition();
        d.fields[0].schema = json!({"$ref":"file:///secret"});
        assert!(Compiled::compile(d).is_err());
        let mut d = definition();
        d.fields[0].editor.as_mut().unwrap().max = Some(0.0);
        assert!(Compiled::compile(d).is_err());
    }
    #[test]
    fn custom_file_page_is_independent_from_configuration_permission() {
        let tmp = tempfile::tempdir().unwrap();
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../../plugins/settings-fixture/manifest.json"
        ))
        .unwrap();
        value["permissions"] = json!(["ui", "files:data"]);
        let manifest: Manifest = serde_json::from_value(value).unwrap();
        let path = tmp.path().join("configuration.json");
        std::fs::write(
            &path,
            json!({"version":1,"page":{"mode":"custom","pageId":"settings"},"fields":[]})
                .to_string(),
        )
        .unwrap();
        assert!(Compiled::read(tmp.path(), &manifest).unwrap().is_some());
        std::fs::write(&path, serde_json::to_string(&definition()).unwrap()).unwrap();
        assert!(Compiled::read(tmp.path(), &manifest).is_err());
        std::fs::write(
            &path,
            json!({"version":1,"page":{"mode":"custom","pageId":"missing"},"fields":[]})
                .to_string(),
        )
        .unwrap();
        assert!(Compiled::read(tmp.path(), &manifest).is_err());
    }
    #[test]
    fn restore_all_clears_invalid_and_removed_keys() {
        let db = Arc::new(Database::open(Path::new(":memory:")).unwrap());
        let configs = Configurations::new(db.clone());
        configs
            .register(
                "one",
                Some(Arc::new(Compiled::compile(definition()).unwrap())),
            )
            .unwrap();
        db.config_change("one", 0, |raw| {
            raw.insert("duration".into(), json!(99));
            raw.insert("removed".into(), json!("legacy"));
            Ok(())
        })
        .unwrap();
        let restored = configs.change("one", 1, None, None).unwrap();
        assert_eq!(restored.values["duration"], 8);
        assert!(restored.diagnostics.is_empty());
        assert!(db.config_read("one").unwrap().1.is_empty());
    }
}
