use crate::model::AppResult;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

pub const API_VERSION: &str = "2.0.0";
pub const PERMISSIONS: &[&str] = &[
    "clipboard:read",
    "http:request",
    "http:transfer",
    "secrets",
    "account:credentials",
    "storage",
    "music:metadata",
    "music:library",
    "music:write",
    "player:read",
    "player:control",
    "ui",
    "config",
    "files:data",
    "files:selected",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub version: String,
    pub backend: Option<String>,
    pub frontend: Option<String>,
    pub configuration: Option<String>,
    pub permissions: Vec<String>,
    #[serde(default, rename = "httpHosts")]
    pub http_hosts: Vec<String>,
    #[serde(default, rename = "fileRoots")]
    pub file_roots: Vec<String>,
    pub engines: Engines,
    #[serde(default)]
    pub contributes: Contributions,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Engines {
    pub app: String,
    pub plugin_api: String,
    pub ui_api: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contributions {
    #[serde(default)]
    pub views: Vec<View>,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub navigation: Vec<Navigation>,
    #[serde(default)]
    pub commands: Vec<serde_json::Value>,
    #[serde(default)]
    pub menus: Vec<serde_json::Value>,
    #[serde(default)]
    pub context_menus: Vec<ContextMenu>,
    #[serde(default)]
    pub settings: Vec<serde_json::Value>,
    #[serde(default)]
    pub shortcuts: Vec<serde_json::Value>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub slot: String,
    pub export: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub id: String,
    pub path: String,
    pub export: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Navigation {
    #[serde(default)]
    pub category: Option<String>,
    pub id: String,
    pub label: String,
    pub page: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextMenu {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub target: String,
    pub source: String,
    pub export: String,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.as_bytes()[0].is_ascii_lowercase()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !matches!(
            id,
            "con"
                | "prn"
                | "aux"
                | "nul"
                | "com1"
                | "com2"
                | "com3"
                | "com4"
                | "com5"
                | "com6"
                | "com7"
                | "com8"
                | "com9"
                | "lpt1"
                | "lpt2"
                | "lpt3"
                | "lpt4"
                | "lpt5"
                | "lpt6"
                | "lpt7"
                | "lpt8"
                | "lpt9"
        )
}
pub fn safe_relative(path: &str) -> AppResult<PathBuf> {
    if path.is_empty()
        || path.len() > 512
        || path.contains(['\\', ':', '\0', '%', '?', '#'])
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
        || Path::new(path)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err("插件资源路径无效".into());
    }
    Ok(PathBuf::from(path))
}
pub fn checked_file(root: &Path, relative: &str) -> AppResult<PathBuf> {
    if is_link(&std::fs::symlink_metadata(root).map_err(|e| e.to_string())?) {
        return Err("插件根目录不允许链接".into());
    }
    let relative = safe_relative(relative)?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        let meta = std::fs::symlink_metadata(&current)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.manifest.resolve_asset.symlink_metadata",
                    error,
                );
            })
            .map_err(|_| "插件文件不存在")?;
        if is_link(&meta) {
            return Err("插件目录不允许链接".into());
        }
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let file = current.canonicalize().map_err(|e| e.to_string())?;
    if !file.starts_with(root) || !file.is_file() {
        return Err("插件资源越界".into());
    }
    Ok(file)
}
pub fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.is_symlink() || meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.is_symlink()
    }
}
impl Manifest {
    pub fn authorization(&self) -> String {
        let mut permissions = self.permissions.clone();
        let mut hosts = self.http_hosts.clone();
        permissions.sort();
        hosts.sort();
        let mut authorization =
            serde_json::json!({"version":self.version,"permissions":permissions,"httpHosts":hosts});
        if !self.file_roots.is_empty() {
            authorization["fileRoots"] = serde_json::json!(self.file_roots);
        }
        authorization.to_string()
    }
    pub fn read(root: &Path) -> AppResult<Self> {
        Self::read_configured(root).map(|(manifest, _)| manifest)
    }
    pub fn read_configured(
        root: &Path,
    ) -> AppResult<(Self, Option<std::sync::Arc<super::configuration::Compiled>>)> {
        let path = checked_file(root, "manifest.json")?;
        if path.metadata().map_err(|e| e.to_string())?.len() > 64 * 1024 {
            return Err("Manifest 过大".into());
        }
        let manifest: Self =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        manifest.validate()?;
        let configuration = super::configuration::Compiled::read(root, &manifest)?;
        for entry in [manifest.backend.as_ref(), manifest.frontend.as_ref()]
            .into_iter()
            .flatten()
        {
            let file = checked_file(root, entry)?;
            let limit = if entry.ends_with(".wasm") {
                16 * 1024 * 1024
            } else {
                8 * 1024 * 1024
            };
            if file.metadata().map_err(|e| e.to_string())?.len() > limit {
                return Err("插件入口过大".into());
            }
        }
        Ok((manifest, configuration))
    }
    pub fn validate(&self) -> AppResult<()> {
        if !self.file_roots.is_empty()
            && (self.file_roots != ["*"] || !self.permissions.iter().any(|p| p == "files:selected"))
        {
            return Err("fileRoots 当前仅支持 [\"*\"]，且须声明 files:selected".into());
        }
        if !valid_id(&self.id) || self.name.trim().is_empty() || self.name.len() > 128 {
            return Err("插件标识或名称无效".into());
        }
        if self.description.as_ref().is_some_and(|description| {
            description.trim().is_empty() || description.chars().count() > 1024
        }) {
            return Err("插件描述必须是 1 至 1024 个字符的非空文本".into());
        }
        if let Some(repository) = &self.repository {
            let url = url::Url::parse(repository)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.manifest.validate.repository_url",
                        error,
                    );
                })
                .map_err(|_| "插件代码仓库链接无效")?;
            if !(repository.starts_with("https://") || repository.starts_with("http://"))
                || repository.chars().count() > 2048
                || repository
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control())
                || !matches!(url.scheme(), "https" | "http")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("插件代码仓库必须是最多 2048 个字符且不含凭证的 HTTP(S) 链接".into());
            }
        }
        Version::parse(&self.version)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "plugins.manifest.validate.version",
                    error,
                );
            })
            .map_err(|_| "插件版本必须是 SemVer")?;
        for (range, version) in [
            (&self.engines.app, env!("CARGO_PKG_VERSION")),
            (&self.engines.plugin_api, API_VERSION),
            (&self.engines.ui_api, API_VERSION),
        ] {
            if !VersionReq::parse(range)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "plugins.manifest.validate.engine_range",
                        error,
                    );
                })
                .map_err(|_| "版本范围无效")?
                .matches(&Version::parse(version).map_err(|e| e.to_string())?)
            {
                return Err("插件 API 或应用版本不兼容".into());
            }
        }
        if self.backend.is_none() && self.frontend.is_none() {
            return Err("插件缺少入口".into());
        }
        for (entry, extension) in [(&self.backend, ".wasm"), (&self.frontend, ".mjs")] {
            if let Some(entry) = entry {
                safe_relative(entry)?;
                if !entry.ends_with(extension) {
                    return Err("插件入口格式无效".into());
                }
            }
        }
        let mut permissions = HashSet::new();
        if self
            .permissions
            .iter()
            .any(|p| !PERMISSIONS.contains(&p.as_str()) || !permissions.insert(p))
        {
            return Err("未知或重复的插件权限".into());
        }
        let c = &self.contributes;
        let mut hosts = HashSet::new();
        if self.http_hosts.len() > 32
            || self
                .http_hosts
                .iter()
                .any(|host| (host != "*" && !super::http::valid_host(host)) || !hosts.insert(host))
            || (self
                .permissions
                .iter()
                .any(|p| p == "http:request" || p == "http:transfer")
                == self.http_hosts.is_empty())
        {
            return Err("HTTP 权限需要 1 至 32 个不重复的精确域名或 *".into());
        }
        if let Some(path) = &self.configuration {
            safe_relative(path)?;
            if !path.ends_with(".json") {
                return Err("配置声明必须是 JSON 文件".into());
            }
        }
        if (!c.views.is_empty()
            || !c.pages.is_empty()
            || !c.navigation.is_empty()
            || !c.context_menus.is_empty())
            && (self.frontend.is_none() || !self.permissions.iter().any(|p| p == "ui"))
        {
            return Err("UI 扩展缺少 frontend 或 ui 权限".into());
        }
        let mut ids = HashSet::new();
        for (id, export) in c
            .views
            .iter()
            .map(|v| (&v.id, &v.export))
            .chain(c.pages.iter().map(|p| (&p.id, &p.export)))
            .chain(c.context_menus.iter().map(|m| (&m.id, &m.export)))
        {
            if !valid_id(id)
                || !ids.insert(id)
                || export.is_empty()
                || export.len() > 128
                || !export
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err("扩展标识或导出名称无效".into());
            }
        }
        if c.context_menus.len() > 16
            || c.context_menus.iter().any(|m| {
                m.target != "song"
                    || !valid_id(&m.source)
                    || m.label.trim().is_empty()
                    || m.label.len() > 128
                    || m.icon.as_deref().is_some_and(|icon| !valid_id(icon))
            })
        {
            return Err("歌曲菜单贡献无效".into());
        }
        if c.views.iter().any(|v| v.slot != "main.overlay") {
            return Err("未知 UI Slot".into());
        }
        let mut paths = HashSet::new();
        for page in &c.pages {
            if !page.path.starts_with('/')
                || page.path.contains(['?', '#', '\\', '%', ':'])
                || page.path.split('/').any(|p| p == ".." || p == ".")
                || !paths.insert(&page.path)
            {
                return Err("插件页面路径无效".into());
            }
        }
        for nav in &c.navigation {
            if nav.category.as_deref().is_some_and(|c| c != "tool")
                || !valid_id(&nav.id)
                || !ids.insert(&nav.id)
                || nav.label.trim().is_empty()
                || nav.label.len() > 128
                || !c.pages.iter().any(|p| p.id == nav.page)
            {
                return Err("导航扩展无效".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_tool_and_rejects_unsupported_file_scope() {
        let mut manifest: Manifest = serde_json::from_str(include_str!(
            "../../../../plugins/ncm-converter/manifest.json"
        ))
        .unwrap();
        manifest.validate().unwrap();
        manifest.file_roots = vec!["C:/".into()];
        assert!(manifest.validate().is_err());
        manifest.file_roots = vec!["*".into()];
        manifest.permissions.retain(|p| p != "files:selected");
        assert!(manifest.validate().is_err());
    }
    #[test]
    fn validates_download_plugin_and_rejects_unknown_menu_targets() {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../plugins/netease-download/manifest.json"
        ))
        .unwrap();
        serde_json::from_value::<Manifest>(value.clone())
            .unwrap()
            .validate()
            .unwrap();
        for (key, invalid) in [
            ("target", "album"),
            ("source", "../any"),
            ("export", "../run"),
            ("icon", "../icon"),
        ] {
            let mut candidate = value.clone();
            candidate["contributes"]["contextMenus"][0][key] = serde_json::json!(invalid);
            assert!(serde_json::from_value::<Manifest>(candidate)
                .unwrap()
                .validate()
                .is_err());
        }
    }
    #[test]
    fn validates_optional_metadata_and_preserves_authorization() {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../plugins/netease-island/manifest.json"
        ))
        .unwrap();
        let valid: Manifest = serde_json::from_value(value.clone()).unwrap();
        assert!(valid.validate().is_ok());
        let roundtrip = serde_json::to_value(&valid).unwrap();
        assert_eq!(roundtrip["description"], value["description"]);
        assert_eq!(roundtrip["repository"], value["repository"]);
        let mut legacy = value.clone();
        legacy.as_object_mut().unwrap().remove("description");
        legacy.as_object_mut().unwrap().remove("repository");
        let legacy: Manifest = serde_json::from_value(legacy).unwrap();
        assert!(legacy.validate().is_ok());
        assert_eq!(valid.authorization(), legacy.authorization());
        for (field, invalid) in [
            ("description", "".to_string()),
            ("description", "  ".to_string()),
            ("description", "文".repeat(1025)),
            ("repository", "".to_string()),
            ("repository", "javascript:alert(1)".to_string()),
            ("repository", "file:///tmp/repo".to_string()),
            ("repository", "git@github.com:example/repo.git".to_string()),
            (
                "repository",
                "https://user:secret@example.org/repo".to_string(),
            ),
            ("repository", "https://example.org/\nrepo".to_string()),
            (
                "repository",
                format!("https://example.org/{}", "a".repeat(2048)),
            ),
        ] {
            let mut candidate = value.clone();
            candidate[field] = serde_json::json!(invalid);
            assert!(
                serde_json::from_value::<Manifest>(candidate)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{field}: {invalid}"
            );
        }
        let mut candidate = valid;
        candidate.description = Some("文".repeat(1024));
        candidate.repository = Some("http://example.org/repo".into());
        assert!(candidate.validate().is_ok());
    }
    #[test]
    fn rejects_paths_and_reserved_ids() {
        for path in ["../a", "/a", "a\\b", "C:/a", "a/%2e", "a//b", "a./b"] {
            assert!(safe_relative(path).is_err(), "{path}");
        }
        assert!(safe_relative("assets/cover.png").is_ok());
        for id in ["../bad", "con", "A", "com1", ""] {
            assert!(!valid_id(id));
        }
        assert!(valid_id("netease-island"));
    }
    #[test]
    fn manifest_checks_compatibility_permissions_and_contribution_references() {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../plugins/netease-island/manifest.json"
        ))
        .unwrap();
        let valid: Manifest = serde_json::from_value(value.clone()).unwrap();
        assert!(valid.validate().is_ok());
        for hosts in [
            serde_json::json!([]),
            serde_json::json!(["*.example.org"]),
            serde_json::json!(["*", "*"]),
            serde_json::json!(["example.org", "example.org"]),
        ] {
            let mut candidate = value.clone();
            candidate["httpHosts"] = hosts;
            assert!(serde_json::from_value::<Manifest>(candidate)
                .unwrap()
                .validate()
                .is_err());
        }
        let mut different_scope = valid.clone();
        different_scope.http_hosts = vec!["*".into()];
        assert!(different_scope.validate().is_ok());
        assert_ne!(valid.authorization(), different_scope.authorization());
        different_scope.http_hosts.push("example.org".into());
        assert_ne!(valid.authorization(), different_scope.authorization());
        for (pointer, invalid) in [
            ("/id", serde_json::json!("../bad")),
            ("/version", serde_json::json!("latest")),
            ("/engines/pluginApi", serde_json::json!("^1.0.0")),
            ("/permissions", serde_json::json!(["http:any"])),
            ("/contributes/views/0/slot", serde_json::json!("unknown")),
            ("/frontend", serde_json::Value::Null),
        ] {
            let mut candidate = value.clone();
            *candidate.pointer_mut(pointer).unwrap() = invalid;
            assert!(
                serde_json::from_value::<Manifest>(candidate)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{pointer}"
            );
        }
        let mut unknown = value;
        unknown["network"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Manifest>(unknown).is_err());
    }
}
