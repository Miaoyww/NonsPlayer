use crate::{
    infrastructure::http::Http, model::AppResult, music::adapter_install::AdapterInstaller,
    plugins::PluginManager, storage::Store,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, State};
use tokio::io::AsyncWriteExt;
use url::Url;

const CDN: &str = "https://nons-plugn.miaoyww.top/index.json";
const GITHUB: &str = "https://raw.githubusercontent.com/NonsPlayer/plugins/meta/index.json";
const SOURCE_KEY: &str = "extensionStoreSource";
const INDEX_BYTES: usize = 2 * 1024 * 1024;
const PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 1024;
const MAX_PENDING: usize = 4;
const TOKEN_TTL: Duration = Duration::from_secs(10 * 60);
const INDEX_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const UNFINISHED: &str = "作者尚未完成，暂不可安装";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Source {
    #[default]
    Cdn,
    Github,
}
impl Source {
    fn url(self) -> &'static str {
        match self {
            Self::Cdn => CDN,
            Self::Github => GITHUB,
        }
    }
    fn setting(self) -> &'static str {
        match self {
            Self::Cdn => "cdn",
            Self::Github => "github",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Plugin,
    Adapter,
}
impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Adapter => "adapter",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Artifact {
    url: String,
    size: Option<u64>,
    sha256: String,
}
impl Artifact {
    fn issue(&self) -> Option<&'static str> {
        if self.size.is_none() {
            Some("缺大小暂不可安装")
        } else if self.size == Some(0) || self.size.is_some_and(|size| size > PACKAGE_BYTES) {
            Some("安装包大小超过允许上限")
        } else if release_url(&self.url, false).is_err() {
            Some("安装包链接无效，暂不可安装")
        } else if self.sha256.len() != 64 || !self.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            Some("安装包 SHA256 无效，暂不可安装")
        } else {
            None
        }
    }
    fn validate(&self) -> AppResult<()> {
        if let Some(issue) = self.issue() {
            return Err(issue.into());
        }
        Ok(())
    }
    fn size(&self) -> AppResult<u64> {
        self.size.ok_or_else(|| "缺大小暂不可安装".into())
    }
}

#[derive(Clone, Debug)]
struct Entry {
    kind: Kind,
    fields: Map<String, Value>,
    artifact: Option<Artifact>,
    artifact_error: Option<&'static str>,
}
impl Entry {
    fn text(&self, key: &str) -> &str {
        self.fields.get(key).and_then(Value::as_str).unwrap_or("")
    }
    fn flag(&self, key: &str) -> bool {
        self.fields.get(key).and_then(Value::as_bool) == Some(true)
    }
    fn compatible(&self) -> bool {
        if self.kind == Kind::Plugin {
            let engines = &self.fields["engines"];
            [
                ("app", env!("CARGO_PKG_VERSION")),
                ("pluginApi", crate::plugins::manifest::API_VERSION),
                ("uiApi", crate::plugins::manifest::API_VERSION),
            ]
            .iter()
            .all(|(key, current)| {
                engines[*key]
                    .as_str()
                    .and_then(|range| VersionReq::parse(range).ok())
                    .zip(Version::parse(current).ok())
                    .is_some_and(|(range, current)| range.matches(&current))
            })
        } else {
            self.text("distribution") == "source"
                || self.fields["contractVersion"].as_u64()
                    == Some(crate::music::CONTRACT_VERSION as u64)
        }
    }
    fn reason(&self, bundled: bool) -> Option<&'static str> {
        if self.flag("blocked") {
            Some("此扩展已被禁止安装")
        } else if self.flag("unavailable") {
            Some("此扩展暂不可用")
        } else if !self.compatible() {
            Some("与当前应用版本或接口不兼容")
        } else if self.text("distribution") == "source" {
            Some(if bundled {
                "官方已随包安装"
            } else {
                UNFINISHED
            })
        } else if cfg!(target_os = "android") {
            Some("请在桌面应用中安装扩展")
        } else if bundled && self.kind == Kind::Adapter {
            Some("随包适配器暂不支持在运行时替换")
        } else if self.fields.get("installable").and_then(Value::as_bool) == Some(false) {
            Some("此扩展暂不可安装")
        } else if self.artifact_error.is_some() {
            self.artifact_error
        } else if let Some(artifact) = &self.artifact {
            artifact.issue()
        } else {
            Some("此扩展尚无可安装产物")
        }
    }
    fn review(
        &self,
        installed: Option<&str>,
        bundled: bool,
        version: &str,
    ) -> AppResult<&Artifact> {
        if self.text("version") != version {
            return Err("商店版本已变化，请刷新后重试".into());
        }
        if let Some(reason) = self.reason(bundled) {
            return Err(reason.into());
        }
        higher_version(version, installed)?;
        self.artifact.as_ref().ok_or_else(|| "缺少安装包".into())
    }
    fn decorated(&self, installed: Option<&str>, bundled: bool) -> Value {
        let mut fields = self.fields.clone();
        for key in ["unavailableReason", "installedVersion"] {
            fields.remove(key);
        }
        fields.insert("compatible".into(), self.compatible().into());
        fields.insert("bundled".into(), bundled.into());
        fields.insert(
            "updateAvailable".into(),
            (installed.is_some()
                && self.reason(bundled).is_none()
                && higher_version(self.text("version"), installed).is_ok())
            .into(),
        );
        if let Some(reason) = self.reason(bundled) {
            fields.insert("unavailableReason".into(), reason.into());
        }
        if let Some(version) = installed {
            fields.insert("installedVersion".into(), version.into());
        }
        Value::Object(fields)
    }
}

struct Catalog {
    generated_at: String,
    entries: Vec<Entry>,
}
fn text<'a>(fields: &'a Map<String, Value>, key: &str, limit: usize) -> AppResult<&'a str> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty() && s.len() <= limit && !s.chars().any(char::is_control))
        .ok_or_else(|| format!("商店字段无效：{key}"))
}
fn strings(value: &Value) -> AppResult<()> {
    let values = value.as_array().ok_or("商店字段必须是文本列表")?;
    if values.len() > 128
        || values.iter().any(|v| {
            v.as_str()
                .is_none_or(|s| s.is_empty() || s.len() > 2048 || s.chars().any(char::is_control))
        })
    {
        return Err("商店文本列表无效".into());
    }
    Ok(())
}
fn parse_catalog(bytes: &[u8]) -> AppResult<Catalog> {
    if bytes.len() > INDEX_BYTES {
        return Err("商店索引超过大小上限".into());
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        crate::infrastructure::diagnostics::warn("extension_store.index_json", error);
        "商店索引格式无效".to_owned()
    })?;
    let root = value.as_object().ok_or("商店索引无效")?;
    if root.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("不支持此商店索引版本".into());
    }
    let generated_at = text(root, "generated_at", 128)?.to_owned();
    let mut entries = Vec::new();
    let mut identities = HashSet::new();
    for (field, kind) in [("plugins", Kind::Plugin), ("adapters", Kind::Adapter)] {
        let values = root
            .get(field)
            .and_then(Value::as_array)
            .ok_or("商店条目列表无效")?;
        if entries.len() + values.len() > MAX_ENTRIES {
            return Err("商店条目数量超过上限".into());
        }
        for value in values {
            let fields = value.as_object().ok_or("商店条目无效")?.clone();
            let id = text(&fields, "id", 128)?;
            if !crate::plugins::manifest::valid_id(id)
                || !identities.insert((kind.as_str(), id.to_owned()))
                || text(&fields, "kind", 16)? != kind.as_str()
            {
                return Err("商店条目标识无效或重复".into());
            }
            text(&fields, "name", 512)?;
            Version::parse(text(&fields, "version", 128)?).map_err(|_| "商店版本号无效")?;
            for flag in ["blocked", "deprecated", "unavailable"] {
                if fields.get(flag).and_then(Value::as_bool).is_none() {
                    return Err(format!("商店状态无效：{flag}"));
                }
            }
            if fields.get("installable").is_some_and(|v| !v.is_boolean()) {
                return Err("商店安装状态无效".into());
            }
            for key in [
                "platforms",
                "searchPlatforms",
                "coverHosts",
                "httpHosts",
                "fileRoots",
                "labels",
            ] {
                if let Some(value) = fields.get(key) {
                    strings(value)?;
                }
            }
            if kind == Kind::Plugin {
                let engines = fields
                    .get("engines")
                    .and_then(Value::as_object)
                    .ok_or("插件缺少版本兼容声明")?;
                for key in ["app", "pluginApi", "uiApi"] {
                    VersionReq::parse(text(engines, key, 256)?)
                        .map_err(|_| "插件版本兼容范围无效")?;
                }
                strings(fields.get("permissions").ok_or("插件缺少权限声明")?)?;
            } else {
                match fields.get("distribution").and_then(Value::as_str) {
                    Some("source") => (),
                    Some("component") => {
                        if fields
                            .get("contractVersion")
                            .and_then(Value::as_u64)
                            .is_none()
                        {
                            return Err("适配器缺少契约版本".into());
                        }
                        let permissions = fields
                            .get("permissions")
                            .and_then(Value::as_object)
                            .ok_or("适配器权限声明无效")?;
                        for key in ["httpHosts", "websocketHosts"] {
                            strings(permissions.get(key).ok_or("适配器缺少网络域名声明")?)?;
                        }
                        for key in ["system", "state", "credentials"] {
                            if permissions.get(key).and_then(Value::as_bool).is_none() {
                                return Err("适配器权限字段无效".into());
                            }
                        }
                    }
                    _ => return Err("不支持此适配器分发类型".into()),
                }
            }
            let artifact = fields
                .get("artifact")
                .filter(|v| !v.is_null())
                .map(|v| {
                    serde_json::from_value::<Artifact>(v.clone())
                        .map_err(|_| "安装包声明无效".to_owned())
                })
                .transpose();
            let (artifact, artifact_error) = match artifact {
                Ok(artifact) => (artifact, None),
                Err(_) => (None, Some("安装包声明无效，暂不可安装")),
            };
            entries.push(Entry {
                kind,
                fields,
                artifact,
                artifact_error,
            });
        }
    }
    Ok(Catalog {
        generated_at,
        entries,
    })
}

fn higher_version(version: &str, installed: Option<&str>) -> AppResult<()> {
    let new = Version::parse(version).map_err(|_| "待安装版本号无效")?;
    if let Some(installed) = installed {
        let old = Version::parse(installed).map_err(|_| "已安装版本无效")?;
        if new.cmp_precedence(&old) != std::cmp::Ordering::Greater {
            return Err("只能安装更高版本".into());
        }
    }
    Ok(())
}

// Redirects are manual: every hop is checked before it reaches the shared client.
fn secure_url(raw: &str) -> AppResult<Url> {
    let url = Url::parse(raw).map_err(|_| "商店链接无效")?;
    if raw.len() > 8192
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || raw.contains(['\\', '\r', '\n'])
    {
        return Err("商店链接不符合安全要求".into());
    }
    Ok(url)
}
fn release_url(raw: &str, redirected: bool) -> AppResult<Url> {
    let url = secure_url(raw)?;
    let valid = match url.host_str() {
        Some("github.com") => {
            let parts: Vec<_> = url.path().split('/').collect();
            parts.len() == 7
                && parts[3] == "releases"
                && parts[4] == "download"
                && parts
                    .iter()
                    .skip(1)
                    .all(|p| !p.is_empty() && *p != "." && *p != "..")
                && url.query().is_none()
        }
        Some(
            "release-assets.githubusercontent.com"
            | "objects.githubusercontent.com"
            | "github-releases.githubusercontent.com",
        ) => redirected,
        _ => false,
    };
    if !valid {
        return Err("安装包及其跳转仅允许使用 GitHub Release 资产链接".into());
    }
    Ok(url)
}

#[derive(Default)]
struct Inventory {
    installed: BTreeMap<(Kind, String), String>,
    bundled: BTreeMap<String, String>,
}
impl Inventory {
    fn installed(&self, entry: &Entry) -> Option<&str> {
        self.installed
            .get(&(entry.kind, entry.text("id").to_owned()))
            .map(String::as_str)
    }
    fn bundled(&self, entry: &Entry) -> bool {
        entry.kind == Kind::Adapter && self.bundled.contains_key(entry.text("id"))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Index {
    source: Source,
    url: &'static str,
    generated_at: String,
    plugins: Vec<Value>,
    adapters: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Review {
    token: String,
    kind: Kind,
    id: String,
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_version: Option<String>,
    permissions: Vec<String>,
    warnings: Vec<String>,
}
struct Pending {
    created: Instant,
    source: Source,
    artifact: Artifact,
    file: tempfile::TempPath,
    review: Review,
}
#[derive(Default)]
struct Tickets(BTreeMap<String, Pending>);
impl Tickets {
    fn prune(&mut self) {
        self.0.retain(|_, p| p.created.elapsed() < TOKEN_TTL);
    }
    fn take(&mut self, token: &str) -> AppResult<Pending> {
        self.prune();
        self.0
            .remove(token)
            .ok_or_else(|| "安装确认已过期或已取消，请重新准备".into())
    }
}

pub(crate) struct ExtensionStore {
    store: Arc<Store>,
    plugins: Arc<PluginManager>,
    adapters: Arc<AdapterInstaller>,
    http: Arc<Http>,
    bundled_directory: PathBuf,
    tickets: Mutex<Tickets>,
    prepare_slots: tokio::sync::Semaphore,
    requests: tokio::sync::Semaphore,
    install: tokio::sync::Mutex<()>,
}
impl ExtensionStore {
    pub(crate) fn new(
        store: Arc<Store>,
        plugins: Arc<PluginManager>,
        adapters: Arc<AdapterInstaller>,
        bundled_directory: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            http: plugins.shared_http(),
            store,
            plugins,
            adapters,
            bundled_directory,
            tickets: Mutex::new(Tickets::default()),
            prepare_slots: tokio::sync::Semaphore::new(1),
            requests: tokio::sync::Semaphore::new(4),
            install: tokio::sync::Mutex::new(()),
        })
    }
    pub(crate) fn cleanup(&self) {
        if let Ok(mut tickets) = self.tickets.lock() {
            tickets.0.clear();
        }
    }
    pub(crate) fn start_cleanup(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let Some(store) = weak.upgrade() else { break };
                if let Ok(mut tickets) = store.tickets.lock() {
                    tickets.prune();
                };
            }
        });
    }
    fn source(&self, source: Option<Source>) -> AppResult<Source> {
        select_source(&self.store, source)
    }
    async fn response(
        &self,
        raw: &str,
        artifact: bool,
        timeout: Duration,
    ) -> AppResult<reqwest::Response> {
        let mut url = secure_url(raw)?;
        for hop in 0..=5 {
            if artifact {
                release_url(url.as_str(), hop > 0)?;
            } else if ![CDN, GITHUB].contains(&url.as_str()) {
                return Err("商店索引跳转地址未获允许".into());
            }
            let response = self
                .http
                .0
                .get(url.clone())
                .timeout(timeout)
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .send()
                .await
                .map_err(|error| network_error(error, artifact))?;
            self.http.check_target(&response)?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .ok_or("下载跳转缺少目标地址")?
                    .to_str()
                    .map_err(|_| "下载跳转地址无效")?;
                url = url.join(location).map_err(|_| "下载跳转地址无效")?;
                continue;
            }
            if response.status() != reqwest::StatusCode::OK {
                return Err(http_error(response.status().as_u16(), artifact));
            }
            return Ok(response);
        }
        Err("下载跳转次数超过上限".into())
    }
    async fn catalog(&self, source: Source) -> AppResult<Catalog> {
        let _slot = self
            .requests
            .try_acquire()
            .map_err(|_| "商店请求繁忙，请稍后重试")?;
        tokio::time::timeout(INDEX_TIMEOUT, async {
            let mut response = self.response(source.url(), false, INDEX_TIMEOUT).await?;
            if response
                .content_length()
                .is_some_and(|n| n > INDEX_BYTES as u64)
            {
                return Err("商店索引超过大小上限".into());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|error| network_error(error, false))?
            {
                if chunk.len() > INDEX_BYTES.saturating_sub(bytes.len()) {
                    return Err("商店索引超过大小上限".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            parse_catalog(&bytes)
        })
        .await
        .map_err(|_| "读取商店索引超时，请重试".to_owned())?
    }
    async fn inventory(self: &Arc<Self>) -> AppResult<Inventory> {
        let this = self.clone();
        blocking(move || {
            let mut result = Inventory::default();
            for plugin in this.plugins.list()? {
                result
                    .installed
                    .insert((Kind::Plugin, plugin.manifest.id), plugin.manifest.version);
            }
            for adapter in this.adapters.list()? {
                result.installed.insert(
                    (Kind::Adapter, adapter.id.as_str().to_owned()),
                    adapter.version,
                );
            }
            // Cargo.lock identifies the native library actually linked into this host.
            let native_version = native_version(include_str!("../../Cargo.lock"))?;
            result
                .bundled
                .insert("netease".into(), native_version.into());
            match std::fs::read_dir(&this.bundled_directory) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry.map_err(|e| e.to_string())?;
                        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                            continue;
                        }
                        let path = entry.path();
                        let mut bytes = Vec::new();
                        std::fs::File::open(path.join("manifest.json"))
                            .map_err(|e| e.to_string())?
                            .take(16385)
                            .read_to_end(&mut bytes)
                            .map_err(|e| e.to_string())?;
                        if bytes.len() > 16384 {
                            return Err("随包适配器描述超过大小上限".into());
                        }
                        let package: crate::music::adapter_install::Package =
                            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                        let id = match (&package.descriptor, &package.resource_provider) {
                            (Some(d), None) => {
                                d.validate().map_err(|e| e.to_string())?;
                                d.source.as_str()
                            }
                            (None, Some(d)) => {
                                d.validate().map_err(|e| e.to_string())?;
                                d.id.as_str()
                            }
                            _ => return Err("随包适配器身份无效".into()),
                        };
                        if let Some(version) = package.version {
                            Version::parse(&version).map_err(|e| e.to_string())?;
                            result.bundled.insert(id.to_owned(), version);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.to_string()),
            }
            for (id, version) in &result.bundled {
                result
                    .installed
                    .insert((Kind::Adapter, id.clone()), version.clone());
            }
            Ok(result)
        })
        .await
    }
    async fn index(self: &Arc<Self>, source: Source) -> AppResult<Index> {
        let catalog = self.catalog(source).await?;
        let inventory = self.inventory().await?;
        let mut index = Index {
            source,
            url: source.url(),
            generated_at: catalog.generated_at,
            plugins: Vec::new(),
            adapters: Vec::new(),
        };
        for entry in catalog.entries {
            let value = entry.decorated(inventory.installed(&entry), inventory.bundled(&entry));
            match entry.kind {
                Kind::Plugin => index.plugins.push(value),
                Kind::Adapter => index.adapters.push(value),
            }
        }
        Ok(index)
    }
    async fn download(&self, artifact: &Artifact) -> AppResult<tempfile::TempPath> {
        artifact.validate()?;
        tokio::time::timeout(DOWNLOAD_TIMEOUT, async {
            let response = self.response(&artifact.url, true, DOWNLOAD_TIMEOUT).await?;
            download_response(response, artifact).await
        })
        .await
        .map_err(|_| "安装包下载超时，请重试".to_owned())?
    }
    async fn package_review(
        self: &Arc<Self>,
        kind: Kind,
        path: PathBuf,
        id: String,
        version: String,
    ) -> AppResult<Review> {
        let this = self.clone();
        blocking(move || {
            let (actual_id, actual_version, name, installed_version, permissions, mut warnings) =
                match kind {
                    Kind::Plugin => {
                        let manifest = this.plugins.preview(&path)?;
                        let installed = this
                            .plugins
                            .list()?
                            .into_iter()
                            .find(|p| p.manifest.id == manifest.id)
                            .map(|p| p.manifest.version);
                        let mut permissions = manifest.permissions.clone();
                        permissions.extend(manifest.http_hosts.iter().map(|h| format!("http:{h}")));
                        permissions
                            .extend(manifest.file_roots.iter().map(|h| format!("files:{h}")));
                        let warnings = plugin_review_warnings(&manifest);
                        (
                            manifest.id,
                            manifest.version,
                            manifest.name,
                            installed,
                            permissions,
                            warnings,
                        )
                    }
                    Kind::Adapter => {
                        let preview = this.adapters.preview(&path)?;
                        let mut permissions: Vec<_> = preview
                            .permissions
                            .http_hosts
                            .iter()
                            .map(|h| format!("http:{h}"))
                            .collect();
                        permissions.extend(
                            preview
                                .permissions
                                .websocket_hosts
                                .iter()
                                .map(|h| format!("websocket:{h}")),
                        );
                        for (enabled, name) in [
                            (preview.permissions.system, "system"),
                            (preview.permissions.state, "state"),
                            (preview.permissions.credentials, "credentials"),
                        ] {
                            if enabled {
                                permissions.push(name.into());
                            }
                        }
                        (
                            preview.id.as_str().to_owned(),
                            preview.version,
                            preview.name,
                            preview.installed_version,
                            permissions,
                            preview.permission_changes,
                        )
                    }
                };
            verify_identity(&id, &version, &actual_id, &actual_version)?;
            higher_version(&version, installed_version.as_deref())?;
            if permissions
                .iter()
                .any(|p| p == "credentials" || p == "account:credentials")
            {
                warnings.push("此扩展可读取账号凭据，请确认信任作者".into());
            }
            if permissions
                .iter()
                .any(|p| p == "http:*" || p == "process:spawn")
            {
                warnings.push("此扩展申请敏感系统或网络权限，请确认信任作者".into());
            }
            let permissions = permissions
                .iter()
                .map(|permission| permission_label(kind, permission))
                .collect();
            Ok(Review {
                token: String::new(),
                kind,
                id,
                version,
                name,
                installed_version,
                permissions,
                warnings,
            })
        })
        .await
    }
    async fn prepare(
        self: &Arc<Self>,
        source: Source,
        kind: Kind,
        id: String,
        version: String,
    ) -> AppResult<Review> {
        let _slot = self
            .prepare_slots
            .try_acquire()
            .map_err(|_| "正在准备其他安装包，请稍后重试")?;
        {
            let mut tickets = self.tickets.lock().map_err(|_| "安装确认状态不可用")?;
            tickets.prune();
            if tickets.0.len() >= MAX_PENDING {
                return Err("待确认安装包已达上限，请先关闭其他安装确认".into());
            }
        }
        let catalog = self.catalog(source).await?;
        let entry = find_entry(&catalog, kind, &id)?;
        let inventory = self.inventory().await?;
        let artifact = entry
            .review(
                inventory.installed(entry),
                inventory.bundled(entry),
                &version,
            )?
            .clone();
        let file = self.download(&artifact).await?;
        let mut review = self
            .package_review(kind, file.to_path_buf(), id, version)
            .await?;
        if entry.flag("deprecated") {
            review.warnings.push("此扩展已弃用，确认后仍可安装".into());
        }
        review.token = uuid::Uuid::new_v4().to_string();
        self.tickets
            .lock()
            .map_err(|_| "安装确认状态不可用")?
            .0
            .insert(
                review.token.clone(),
                Pending {
                    created: Instant::now(),
                    source,
                    artifact,
                    file,
                    review: review.clone(),
                },
            );
        Ok(review)
    }
    async fn install(
        self: &Arc<Self>,
        token: &str,
        confirmed: bool,
        app: &tauri::AppHandle,
    ) -> AppResult<()> {
        let _operation = self
            .install
            .try_lock()
            .map_err(|_| "正在安装其他扩展，请稍后重试")?;
        let pending = self
            .tickets
            .lock()
            .map_err(|_| "安装确认状态不可用")?
            .take(token)?;
        if !confirmed {
            return Err("安装前必须确认权限".into());
        }
        let catalog = self.catalog(pending.source).await?;
        let entry = find_entry(&catalog, pending.review.kind, &pending.review.id)?;
        let inventory = self.inventory().await?;
        let artifact = entry.review(
            inventory.installed(entry),
            inventory.bundled(entry),
            &pending.review.version,
        )?;
        unchanged_artifact(artifact, &pending.artifact)?;
        let path = pending.file.to_path_buf();
        let expected = artifact.clone();
        blocking(move || verify_file(&path, &expected)).await?;
        let mut review = self
            .package_review(
                entry.kind,
                pending.file.to_path_buf(),
                pending.review.id.clone(),
                pending.review.version.clone(),
            )
            .await?;
        if entry.flag("deprecated") {
            review.warnings.push("此扩展已弃用，确认后仍可安装".into());
        }
        review.token.clone_from(&pending.review.token);
        if review != pending.review {
            return Err("安装确认信息已变化，请重新准备并确认".into());
        }
        match entry.kind {
            Kind::Plugin => {
                self.plugins
                    .install_reviewed(pending.file.to_path_buf(), true)
                    .await?
            }
            Kind::Adapter => {
                self.adapters
                    .install(pending.file.to_path_buf(), true)
                    .await?;
                let _ = app.emit("adapters-changed", ()).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "extension_store.adapters_changed",
                        error,
                    );
                });
            }
        }
        let _ = app
            .emit("extension-store-changed", ())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("extension_store.changed", error);
            });
        Ok(())
    }
}

fn native_version(lock: &str) -> AppResult<&str> {
    lock.split("[[package]]")
        .find_map(|package| {
            let mut lines = package
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty());
            if lines.next()? != "name = \"nons-adapter-netease\"" {
                return None;
            }
            let version = lines
                .next()?
                .strip_prefix("version = \"")?
                .strip_suffix('"')?;
            Version::parse(version).ok()?;
            Some(version)
        })
        .ok_or_else(|| "随包原生适配器版本缺失".into())
}
fn select_source(store: &Store, source: Option<Source>) -> AppResult<Source> {
    if let Some(source) = source {
        // Persist intent before the first await, including failed network attempts.
        store.set_setting(SOURCE_KEY, source.setting())?;
        Ok(source)
    } else {
        match store.setting(SOURCE_KEY)?.as_deref() {
            Some("github") => Ok(Source::Github),
            Some("cdn") | None => Ok(Source::Cdn),
            _ => Err("已保存的商店来源无效".into()),
        }
    }
}
fn find_entry<'a>(catalog: &'a Catalog, kind: Kind, id: &str) -> AppResult<&'a Entry> {
    catalog
        .entries
        .iter()
        .find(|entry| entry.kind == kind && entry.text("id") == id)
        .ok_or_else(|| "此扩展已从商店移除".into())
}
fn unchanged_artifact(current: &Artifact, reviewed: &Artifact) -> AppResult<()> {
    if current != reviewed {
        return Err("商店安装包已变化，请重新准备并确认".into());
    }
    Ok(())
}
async fn download_response(
    mut response: reqwest::Response,
    artifact: &Artifact,
) -> AppResult<tempfile::TempPath> {
    artifact.validate()?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(http_error(response.status().as_u16(), true));
    }
    let expected = artifact.size()?;
    if response
        .content_length()
        .is_some_and(|size| size != expected)
    {
        return Err("安装包长度与目录声明不符".into());
    }
    let temp = blocking(|| {
        tempfile::NamedTempFile::new().map_err(|error| local_error("创建安装临时文件失败", error))
    })
    .await?;
    let (file, path) = temp.into_parts();
    let mut file = tokio::fs::File::from_std(file);
    let mut size = 0;
    let mut hash = Sha256::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| network_error(error, true))?
    {
        checked_size(&mut size, chunk.len(), expected)?;
        hash.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|error| local_error("写入安装临时包失败", error))?;
    }
    verify_digest(size, &hex::encode(hash.finalize()), artifact)?;
    file.flush()
        .await
        .map_err(|error| local_error("保存安装临时包失败", error))?;
    drop(file);
    Ok(path)
}
fn http_error(status: u16, artifact: bool) -> String {
    let resource = if artifact {
        "安装包"
    } else {
        "商店索引"
    };
    format!("{resource}暂不可用（HTTP {status}）")
}
fn permission_label(kind: Kind, permission: &str) -> String {
    if let Some(host) = permission.strip_prefix("http:") {
        if host != "request" && host != "transfer" {
            return if host == "*" {
                "敏感权限：请求任意网络域名".into()
            } else {
                format!("网络请求：{host}")
            };
        }
    }
    if let Some(host) = permission.strip_prefix("websocket:") {
        return format!("网络长连接：{host}");
    }
    if let Some(root) = permission.strip_prefix("files:") {
        if !matches!(root, "data" | "selected") {
            return format!("外部目录授权范围：{root}");
        }
    }
    match permission {
        "clipboard:read" => "读取剪贴板文本及监听变化",
        "http:request" => "请求已声明的网络域名",
        "http:transfer" => "将授权域名的资源传输到授权目录",
        "secrets" => "管理插件自己的系统凭据",
        "account:credentials" => "敏感权限：读取音乐平台账户凭证",
        "music:metadata" => "读取音乐来源的公开音乐信息",
        "music:library" => "读取当前账号的音乐库、收藏和听歌记录",
        "music:write" => "敏感权限：更改当前账号的收藏、歌单和推荐",
        "player:read" => "读取播放状态",
        "player:control" => "控制播放",
        "process:spawn" => "敏感权限：启动任意程序并管理其输入输出",
        "window:manage" => "创建和管理插件独立窗口（含置顶、透明和鼠标穿透）",
        "storage" => "保存独立插件数据",
        "ui" => "运行受信任的界面代码",
        "config" => "管理独立插件配置",
        "files:data" => "读写插件专属数据目录",
        "files:selected" => "申请访问用户选择的外部目录",
        "system" if kind == Kind::Adapter => "访问宿主系统能力",
        "state" if kind == Kind::Adapter => "保存适配器专属状态",
        "credentials" if kind == Kind::Adapter => "敏感权限：读取当前平台账号凭据",
        _ => return format!("扩展权限：{permission}"),
    }
    .into()
}
fn plugin_review_warnings(manifest: &crate::plugins::manifest::Manifest) -> Vec<String> {
    let mut warnings = Vec::new();
    if manifest.frontend.is_some()
        || manifest
            .permissions
            .iter()
            .any(|permission| permission == "ui")
    {
        warnings.push("前端与播放器共享界面环境，可执行代码；WASM权限不构成前端沙箱".into());
    }
    if manifest.file_roots.iter().any(|root| root == "*") {
        warnings.push(
            "敏感权限：外部目录授权范围为 *，获得授权后可读写、修改和删除其中任意文件".into(),
        );
    }
    warnings
}
fn require_desktop_installation() -> AppResult<()> {
    #[cfg(target_os = "android")]
    return Err("请在桌面应用中安装扩展".into());
    #[cfg(not(target_os = "android"))]
    Ok(())
}
fn network_error(error: reqwest::Error, artifact: bool) -> String {
    crate::infrastructure::diagnostics::warn("extension_store.network", &error);
    let resource = if artifact {
        "安装包"
    } else {
        "商店索引"
    };
    if error.is_timeout() {
        format!("{resource}请求超时，请重试")
    } else if error.is_connect() {
        format!("无法连接{resource}服务器，请检查网络后重试")
    } else {
        format!("{resource}下载失败，请重试")
    }
}
fn local_error(message: &str, error: std::io::Error) -> String {
    crate::infrastructure::diagnostics::warn("extension_store.file", error);
    message.to_owned()
}
fn checked_size(size: &mut u64, chunk: usize, expected: u64) -> AppResult<()> {
    *size = size.checked_add(chunk as u64).ok_or("安装包大小溢出")?;
    if *size > expected || *size > PACKAGE_BYTES {
        return Err("安装包超过声明大小".into());
    }
    Ok(())
}
fn verify_digest(size: u64, digest: &str, artifact: &Artifact) -> AppResult<()> {
    if size != artifact.size()? || !digest.eq_ignore_ascii_case(&artifact.sha256) {
        return Err("安装包大小或 SHA256 校验失败".into());
    }
    Ok(())
}
fn verify_file(path: &Path, artifact: &Artifact) -> AppResult<()> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| local_error("安装临时包不存在或不可读", error))?;
    if crate::plugins::manifest::is_link(&metadata)
        || !metadata.is_file()
        || metadata.len() != artifact.size()?
    {
        return Err("安装临时包已变化，请重新准备".into());
    }
    let mut file =
        std::fs::File::open(path).map_err(|error| local_error("安装临时包不可读", error))?;
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut bytes = [0; 65536];
    loop {
        let count = file
            .read(&mut bytes)
            .map_err(|error| local_error("读取安装临时包失败", error))?;
        if count == 0 {
            break;
        }
        checked_size(&mut size, count, artifact.size()?)?;
        hash.update(&bytes[..count]);
    }
    verify_digest(size, &hex::encode(hash.finalize()), artifact)
}
fn verify_identity(
    id: &str,
    version: &str,
    actual_id: &str,
    actual_version: &str,
) -> AppResult<()> {
    if id != actual_id || version != actual_version {
        return Err("安装包标识或版本与商店条目不符".into());
    }
    Ok(())
}
async fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| e.to_string())?
}

async fn finish_installation(
    task: impl std::future::Future<Output = AppResult<()>> + Send + 'static,
) -> AppResult<()> {
    // Dropping the IPC waiter must not cancel publication or release its temporary package.
    tauri::async_runtime::spawn(task).await.map_err(|error| {
        crate::infrastructure::diagnostics::warn("extension_store.install_task", error);
        "安装任务异常结束".to_owned()
    })?
}

#[tauri::command]
pub(crate) async fn extension_store_index(
    source: Option<Source>,
    refresh: Option<bool>,
    store: State<'_, Arc<ExtensionStore>>,
) -> AppResult<Index> {
    let source = store.source(source)?;
    let _ = refresh; // Indexes are never cached by the backend.
    store.inner().clone().index(source).await
}
#[tauri::command]
pub(crate) async fn extension_store_prepare(
    source: Source,
    kind: Kind,
    id: String,
    version: String,
    store: State<'_, Arc<ExtensionStore>>,
) -> AppResult<Review> {
    require_desktop_installation()?;
    store
        .inner()
        .clone()
        .prepare(source, kind, id, version)
        .await
}
#[tauri::command]
pub(crate) async fn extension_store_install(
    token: String,
    confirmed: bool,
    store: State<'_, Arc<ExtensionStore>>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    require_desktop_installation()?;
    let store = store.inner().clone();
    finish_installation(async move { store.install(&token, confirmed, &app).await }).await
}
#[tauri::command]
pub(crate) fn extension_store_cancel(
    token: String,
    store: State<'_, Arc<ExtensionStore>>,
) -> AppResult<()> {
    let mut tickets = store.tickets.lock().map_err(|_| "安装确认状态不可用")?;
    tickets.prune();
    tickets.0.remove(&token);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    #[test]
    fn plugin_review_always_explains_frontend_trust_and_wildcard_file_access() {
        let mut manifest: crate::plugins::manifest::Manifest = serde_json::from_value(json!({
            "id": "review-test", "name": "Review test", "version": "1.0.0",
            "frontend": "ui.mjs", "permissions": ["ui", "files:selected"], "fileRoots": ["*"],
            "engines": {"app": "^1.0.0", "pluginApi": "^2.0.0", "uiApi": "^2.0.0"}
        }))
        .unwrap();
        let warnings = plugin_review_warnings(&manifest);
        assert!(warnings.iter().any(
            |warning| warning == "前端与播放器共享界面环境，可执行代码；WASM权限不构成前端沙箱"
        ));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("删除") && warning.contains("任意文件")));
        manifest.frontend = None;
        assert!(plugin_review_warnings(&manifest)
            .iter()
            .any(|warning| warning.contains("前端沙箱")));
        manifest.permissions.clear();
        manifest.frontend = Some("ui.mjs".into());
        assert!(plugin_review_warnings(&manifest)
            .iter()
            .any(|warning| warning.contains("前端沙箱")));
        manifest.frontend = None;
        manifest.file_roots.clear();
        assert!(plugin_review_warnings(&manifest).is_empty());
    }

    #[tokio::test]
    async fn cancelled_ipc_waiter_keeps_installation_and_package_alive_until_completion() {
        let file = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let path = file.to_path_buf();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, proceed) = tokio::sync::oneshot::channel();
        let (completed, done) = tokio::sync::oneshot::channel();
        let waiter = tokio::spawn(finish_installation(async move {
            let _ = started.send(());
            proceed.await.map_err(|error| error.to_string())?;
            assert!(file.exists());
            drop(file);
            let _ = completed.send(());
            Ok(())
        }));
        tokio::time::timeout(Duration::from_secs(3), ready)
            .await
            .unwrap()
            .unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert!(path.exists());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), done)
            .await
            .unwrap()
            .unwrap();
        assert!(!path.exists());
        assert_eq!(
            finish_installation(async { Err("安装校验失败".into()) })
                .await
                .unwrap_err(),
            "安装校验失败"
        );
    }

    #[tokio::test]
    #[ignore = "Requires network"]
    async fn live_catalog_sources_parse_current_online_schema() {
        let http = Http::new().expect("NETWORK: 无法创建公共网络客户端");
        let mut failures = Vec::new();
        for source in [Source::Cdn, Source::Github] {
            let url = source.url();
            let result: AppResult<Vec<u8>> = match tokio::time::timeout(INDEX_TIMEOUT, async {
                let mut response = http
                    .0
                    .get(secure_url(url)?)
                    .timeout(INDEX_TIMEOUT)
                    .header(reqwest::header::ACCEPT_ENCODING, "identity")
                    .send()
                    .await
                    .map_err(|error| format!("NETWORK: {error:?}"))?;
                http.check_target(&response)
                    .map_err(|error| format!("NETWORK_POLICY: {error}"))?;
                if response.status() != reqwest::StatusCode::OK {
                    return Err(format!("HTTP: {}", response.status()));
                }
                if response
                    .content_length()
                    .is_some_and(|size| size > INDEX_BYTES as u64)
                {
                    return Err("SIZE_LIMIT: 商店索引超过 2MiB".into());
                }
                let mut bytes = Vec::new();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|error| format!("NETWORK: 读取响应失败：{error:?}"))?
                {
                    if chunk.len() > INDEX_BYTES.saturating_sub(bytes.len()) {
                        return Err("SIZE_LIMIT: 商店索引超过 2MiB".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Ok(bytes)
            })
            .await
            {
                Ok(result) => result,
                Err(_) => Err("NETWORK: 读取商店索引超过 20 秒期限".into()),
            };
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(error) => {
                    let failure = format!("{} {url}: {error}", source.setting());
                    eprintln!("{failure}");
                    failures.push(failure);
                    continue;
                }
            };
            let catalog = match parse_catalog(&bytes) {
                Ok(catalog) => catalog,
                Err(error) => {
                    let failure = format!("{} {url}: SCHEMA: {error}", source.setting());
                    eprintln!("{failure}");
                    failures.push(failure);
                    continue;
                }
            };
            let plugins = catalog
                .entries
                .iter()
                .filter(|entry| entry.kind == Kind::Plugin)
                .count();
            println!("source={} url={url} schema=1 generatedAt={} plugins={plugins} adapters={} bytes={} indexSha256={}",
                source.setting(), catalog.generated_at, catalog.entries.len() - plugins,
                bytes.len(), hex::encode(Sha256::digest(&bytes)));
            for entry in catalog.entries {
                let artifact = if entry.text("distribution") == "source" {
                    "sourceArtifact"
                } else {
                    "artifact"
                };
                println!(
                    "  {} {} version={} {artifact}.sha256={}",
                    entry.kind.as_str(),
                    entry.text("id"),
                    entry.text("version"),
                    entry
                        .fields
                        .get(artifact)
                        .and_then(|value| value.get("sha256"))
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>")
                );
            }
        }
        assert!(
            failures.is_empty(),
            "联网索引验证失败：\n{}",
            failures.join("\n")
        );
    }

    fn artifact(bytes: &[u8]) -> Artifact {
        Artifact {
            url: "https://github.com/NonsPlayer/example/releases/download/v1.2.0/example.zip"
                .into(),
            size: Some(bytes.len() as u64),
            sha256: hex::encode(Sha256::digest(bytes)),
        }
    }
    fn plugin() -> Value {
        let artifact = artifact(b"package");
        json!({
            "id": "example", "kind": "plugin", "name": "Example", "version": "1.2.0",
            "blocked": false, "unavailable": false, "deprecated": false,
            "engines": {"app": "^1.0.0", "pluginApi": "^2.0.0", "uiApi": "^2.0.0"},
            "permissions": ["ui"], "artifact": {
                "url": artifact.url, "size": artifact.size, "sha256": artifact.sha256
            }, "futureMetadata": {"preserved": true}
        })
    }
    fn adapter() -> Value {
        let mut value = plugin();
        value["kind"] = json!("adapter");
        value["distribution"] = json!("component");
        value["contractVersion"] = json!(crate::music::CONTRACT_VERSION);
        value["platforms"] = json!(["netease", "qqmusic"]);
        value["permissions"] = json!({"httpHosts": [], "websocketHosts": [], "system": true, "state": false, "credentials": false});
        value
    }
    fn catalog(plugin: Vec<Value>, adapters: Vec<Value>) -> AppResult<Catalog> {
        parse_catalog(&serde_json::to_vec(&json!({"schema_version": 1, "generated_at": "2026-10-30T12:00:00Z", "plugins": plugin, "adapters": adapters})).unwrap())
    }
    fn entry(value: Value) -> Entry {
        if value["kind"] == "plugin" {
            catalog(vec![value], vec![]).unwrap().entries.remove(0)
        } else {
            catalog(vec![], vec![value]).unwrap().entries.remove(0)
        }
    }

    #[test]
    fn catalog_preserves_metadata_and_uses_semver_and_music_contract() {
        let plugin = entry(plugin());
        assert!(plugin.compatible());
        let decorated = plugin.decorated(Some("1.1.0"), false);
        assert_eq!(decorated["futureMetadata"]["preserved"], true);
        assert_eq!(decorated["installedVersion"], "1.1.0");
        assert_eq!(decorated["updateAvailable"], true);
        let mut incompatible = plugin.fields.clone();
        incompatible.get_mut("engines").unwrap()["app"] = json!(">=999.0.0");
        let incompatible = entry(Value::Object(incompatible));
        assert!(!incompatible.compatible());
        assert!(incompatible.review(None, false, "1.2.0").is_err());
        let mut value = adapter();
        assert!(entry(value.clone()).compatible());
        value["contractVersion"] = json!(999);
        assert!(!entry(value).compatible());
    }

    #[test]
    fn catalog_rejects_invalid_schema_and_entries_without_panicking() {
        for value in [
            json!({}),
            json!({"schema_version": 2}),
            json!({"schema_version": 1, "generated_at": "now"}),
        ] {
            assert!(parse_catalog(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        assert!(parse_catalog(&vec![b' '; INDEX_BYTES + 1]).is_err());
        assert!(catalog(vec![plugin(), plugin()], vec![]).is_err());
        for (key, value) in [
            ("blocked", json!("false")),
            ("id", json!("../escape")),
            ("version", json!("latest")),
            ("kind", json!("adapter")),
            ("permissions", json!([42])),
        ] {
            let mut invalid = plugin();
            invalid[key] = value;
            assert!(catalog(vec![invalid], vec![]).is_err(), "accepted {key}");
        }
        let mut invalid = plugin();
        invalid["engines"]["pluginApi"] = json!("invalid");
        assert!(catalog(vec![invalid], vec![]).is_err());
        assert!(catalog(vec![plugin(); MAX_ENTRIES + 1], vec![]).is_err());
    }

    #[test]
    fn blocked_unavailable_source_and_missing_artifact_are_refused_but_deprecated_is_reviewable() {
        for flag in ["blocked", "unavailable"] {
            let mut value = plugin();
            value[flag] = json!(true);
            let entry = entry(value);
            assert!(entry.review(None, false, "1.2.0").is_err());
            assert_eq!(
                entry.decorated(Some("1.0.0"), false)["updateAvailable"],
                false
            );
        }
        let mut deprecated = plugin();
        deprecated["deprecated"] = json!(true);
        assert!(entry(deprecated).review(None, false, "1.2.0").is_ok());
        let mut missing = plugin();
        missing["artifact"] = Value::Null;
        assert!(entry(missing).review(None, false, "1.2.0").is_err());
        let mut source = adapter();
        source["distribution"] = json!("source");
        source["installable"] = json!(false);
        let source = entry(source);
        assert_eq!(source.reason(false), Some(UNFINISHED));
        assert_ne!(source.reason(true), Some(UNFINISHED));
        assert!(source.review(None, false, "1.2.0").is_err());
        assert!(source.review(Some("1.0.0"), true, "1.2.0").is_err());
        assert!(entry(adapter())
            .review(Some("1.0.0"), true, "1.2.0")
            .is_err());
    }

    #[test]
    fn versions_and_identity_require_exact_selection_and_strict_upgrade() {
        let entry = entry(plugin());
        for version in ["1.2.0", "2.0.0", "1.2.0+local"] {
            assert!(entry.review(Some(version), false, "1.2.0").is_err());
        }
        assert!(entry.review(None, false, "1.3.0").is_err());
        assert!(higher_version("1.2.0+new", Some("1.2.0+old")).is_err());
        assert!(higher_version("1.2.0", Some("1.2.0-rc.1")).is_ok());
        assert!(verify_identity("a", "1.0.0", "b", "1.0.0").is_err());
        assert!(verify_identity("a", "1.0.0", "a", "1.0.1").is_err());
        assert!(verify_identity("a", "1.0.0", "a", "1.0.0").is_ok());
    }

    #[test]
    fn confirmation_rechecks_catalog_removal_status_version_and_artifact() {
        let initial = catalog(vec![plugin()], vec![]).unwrap();
        let selected = find_entry(&initial, Kind::Plugin, "example").unwrap();
        let reviewed = selected.review(None, false, "1.2.0").unwrap();
        assert!(find_entry(&initial, Kind::Adapter, "example").is_err());
        let empty = catalog(vec![], vec![]).unwrap();
        assert!(find_entry(&empty, Kind::Plugin, "example").is_err());
        for flag in ["blocked", "unavailable"] {
            let mut value = plugin();
            value[flag] = json!(true);
            assert!(entry(value).review(None, false, "1.2.0").is_err());
        }
        let mut value = plugin();
        value["version"] = json!("1.3.0");
        assert!(entry(value).review(None, false, "1.2.0").is_err());
        assert!(unchanged_artifact(reviewed, reviewed).is_ok());
        for changed in [
            Artifact {
                size: Some(reviewed.size.unwrap() + 1),
                ..reviewed.clone()
            },
            Artifact {
                sha256: "0".repeat(64),
                ..reviewed.clone()
            },
            Artifact {
                url: "https://github.com/NonsPlayer/other/releases/download/v1.2.0/example.zip"
                    .into(),
                ..reviewed.clone()
            },
        ] {
            assert!(unchanged_artifact(&changed, reviewed).is_err());
        }
    }

    #[test]
    fn release_urls_restrict_initial_downloads_and_every_redirect() {
        let url = artifact(b"package").url;
        assert!(release_url(&url, false).is_ok());
        for host in [
            "release-assets.githubusercontent.com",
            "objects.githubusercontent.com",
            "github-releases.githubusercontent.com",
        ] {
            let url = format!("https://{host}/asset?signature=opaque");
            assert!(release_url(&url, true).is_ok());
            assert!(release_url(&url, false).is_err());
        }
        for url in [
            "http://github.com/a/b/releases/download/v1/a.zip",
            "https://github.com.evil.test/a/b/releases/download/v1/a.zip",
            "https://github.com@evil.test/a/b/releases/download/v1/a.zip",
            "https://evil.test@github.com/a/b/releases/download/v1/a.zip",
            "https://github.com:8443/a/b/releases/download/v1/a.zip",
            "https://github.com/a/b/archive/refs/heads/main.zip",
            "https://raw.githubusercontent.com/a/b/main/a.zip",
            "https://127.0.0.1/a.zip",
            "https://release-assets.githubusercontent.com.evil.test/asset",
            "file:///tmp/package.zip",
            "https://github.com/a/b/releases/download/v1/a.zip#fragment",
        ] {
            assert!(release_url(url, false).is_err(), "accepted initial {url}");
            assert!(release_url(url, true).is_err(), "accepted redirect {url}");
        }
    }

    #[test]
    fn unavailable_reasons_and_download_failures_are_localized() {
        assert_eq!(http_error(404, true), "安装包暂不可用（HTTP 404）");
        assert_eq!(http_error(503, false), "商店索引暂不可用（HTTP 503）");
        let mut source = adapter();
        source["distribution"] = json!("source");
        assert_eq!(entry(source).reason(true), Some("官方已随包安装"));
        for flag in ["blocked", "unavailable"] {
            let mut value = plugin();
            value[flag] = json!(true);
            let reason = entry(value).reason(false).unwrap();
            assert!(!reason.is_ascii());
        }
        assert_eq!(
            permission_label(Kind::Plugin, "files:selected"),
            "申请访问用户选择的外部目录"
        );
        assert_eq!(
            permission_label(Kind::Adapter, "credentials"),
            "敏感权限：读取当前平台账号凭据"
        );
        assert_eq!(
            permission_label(Kind::Adapter, "http:example.org"),
            "网络请求：example.org"
        );
    }

    #[test]
    fn missing_size_and_bad_artifacts_only_disable_their_own_entry() {
        for (id, artifact, reason) in [
            (
                "null-size",
                json!({"url": "https://github.com/a/b/releases/download/v1/a.zip", "size": null, "sha256": "0".repeat(64)}),
                "缺大小暂不可安装",
            ),
            (
                "bad-url",
                json!({"url": "https://evil.test/asset.zip", "size": 4, "sha256": "0".repeat(64)}),
                "安装包链接无效，暂不可安装",
            ),
            (
                "malformed",
                json!({"size": "wrong"}),
                "安装包声明无效，暂不可安装",
            ),
        ] {
            let mut bad = plugin();
            bad["id"] = json!(id);
            bad["artifact"] = artifact.clone();
            let catalog = catalog(vec![bad, plugin()], vec![]).unwrap();
            let bad = find_entry(&catalog, Kind::Plugin, id).unwrap();
            assert_eq!(bad.reason(false), Some(reason));
            assert!(bad.review(None, false, "1.2.0").is_err());
            assert_eq!(bad.decorated(None, false)["artifact"], artifact);
            assert!(find_entry(&catalog, Kind::Plugin, "example")
                .unwrap()
                .review(None, false, "1.2.0")
                .is_ok());
        }
    }

    async fn fixture_response(response: &'static [u8]) -> reqwest::Response {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = [0; 4096];
            assert!(socket.read(&mut request).unwrap() > 0);
            let _ = socket.write_all(response);
        });
        let http = Http::for_test(
            reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
        );
        let response = http
            .0
            .get(format!("http://{address}/asset.zip"))
            .send()
            .await
            .unwrap();
        http.check_target(&response).unwrap();
        server.join().unwrap();
        response
    }

    #[tokio::test]
    async fn http_download_stream_checks_hash_declared_limits_and_404() {
        let expected = artifact(b"package");
        let response = fixture_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\npackage",
        )
        .await;
        let package = download_response(response, &expected).await.unwrap();
        assert_eq!(std::fs::read(&package).unwrap(), b"package");
        verify_file(&package, &expected).unwrap();
        let path = package.to_path_buf();
        drop(package);
        assert!(!path.exists());

        let response = fixture_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nchanged",
        )
        .await;
        assert!(download_response(response, &expected)
            .await
            .unwrap_err()
            .contains("SHA256"));
        let response = fixture_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\npackage",
        )
        .await;
        assert!(download_response(response, &expected)
            .await
            .unwrap_err()
            .contains("长度"));
        let response = fixture_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8\r\npackage!\r\n0\r\n\r\n").await;
        assert!(download_response(response, &expected)
            .await
            .unwrap_err()
            .contains("超过声明大小"));
        let response = fixture_response(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert_eq!(
            download_response(response, &expected).await.unwrap_err(),
            "安装包暂不可用（HTTP 404）"
        );
        let oversized = Artifact {
            size: Some(PACKAGE_BYTES + 1),
            ..expected
        };
        let response =
            fixture_response(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
        assert!(download_response(response, &oversized)
            .await
            .unwrap_err()
            .contains("大小超过"));
    }

    #[test]
    fn streamed_and_retained_packages_check_digest_size_and_mutation() {
        let artifact = artifact(b"package");
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"package").unwrap();
        file.flush().unwrap();
        assert!(verify_file(file.path(), &artifact).is_ok());
        std::fs::write(file.path(), b"changed").unwrap();
        assert!(verify_file(file.path(), &artifact).is_err());
        std::fs::write(file.path(), b"short").unwrap();
        assert!(verify_file(file.path(), &artifact).is_err());
        let mut size = 0;
        checked_size(&mut size, 3, 7).unwrap();
        assert!(checked_size(&mut size, 5, 7).is_err());
        assert!(verify_digest(6, &artifact.sha256, &artifact).is_err());
        let mut bad = artifact.clone();
        bad.size = Some(PACKAGE_BYTES + 1);
        assert!(bad.validate().is_err());
        bad.size = Some(0);
        assert!(bad.validate().is_err());
        bad.size = Some(7);
        bad.sha256 = "not-a-digest".into();
        assert!(bad.validate().is_err());
        bad.sha256 = "g".repeat(64);
        assert!(bad.validate().is_err());
    }

    #[test]
    fn tickets_are_single_use_expire_and_delete_owned_temp_files() {
        let mut tickets = Tickets::default();
        let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let disk_path = path.to_path_buf();
        let review = Review {
            token: "one".into(),
            kind: Kind::Plugin,
            id: "example".into(),
            name: "Example".into(),
            version: "1.2.0".into(),
            installed_version: None,
            permissions: vec![],
            warnings: vec![],
        };
        tickets.0.insert(
            "one".into(),
            Pending {
                created: Instant::now(),
                source: Source::Cdn,
                artifact: artifact(b"package"),
                file: path,
                review,
            },
        );
        let pending = tickets.take("one").unwrap();
        assert!(tickets.take("one").is_err());
        assert!(disk_path.exists());
        drop(pending);
        assert!(!disk_path.exists());
        let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let disk_path = path.to_path_buf();
        tickets.0.insert(
            "expired".into(),
            Pending {
                created: Instant::now() - TOKEN_TTL,
                source: Source::Cdn,
                artifact: artifact(b"package"),
                file: path,
                review: Review {
                    token: "expired".into(),
                    kind: Kind::Plugin,
                    id: "a".into(),
                    name: "A".into(),
                    version: "1.0.0".into(),
                    installed_version: None,
                    permissions: vec![],
                    warnings: vec![],
                },
            },
        );
        assert!(tickets.take("expired").is_err());
        assert!(!disk_path.exists());
    }

    #[test]
    fn explicit_source_is_persisted_before_requests_and_implicit_read_preserves_it() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(&directory.path().join("store.sqlite3")).unwrap();
        assert_eq!(select_source(&store, None).unwrap(), Source::Cdn);
        assert!(store.setting(SOURCE_KEY).unwrap().is_none());
        let slow_source = select_source(&store, Some(Source::Github)).unwrap();
        assert_eq!(
            store.setting(SOURCE_KEY).unwrap().as_deref(),
            Some("github")
        );
        let fast_source = select_source(&store, Some(Source::Cdn)).unwrap();
        assert_eq!(fast_source, Source::Cdn);
        assert_eq!(slow_source, Source::Github);
        assert_eq!(select_source(&store, None).unwrap(), Source::Cdn);
        assert_eq!(store.setting(SOURCE_KEY).unwrap().as_deref(), Some("cdn"));
    }

    #[test]
    fn native_bundled_version_is_read_from_the_locked_dependency() {
        assert_eq!(native_version("[[package]]\nname = \"unrelated\"\nversion = \"9.0.0\"\n[[package]]\nname = \"nons-adapter-netease\"\nversion = \"1.3.0\"\n").unwrap(), "1.3.0");
        assert!(native_version("").is_err());
        assert!(native_version(include_str!("../../Cargo.lock")).is_ok());
    }
}
