//! Runtime installation and lifecycle for external music adapter Components.
//!
//! Installed packages are copied into application data before they are loaded.
//! The package itself never runs from the user selected path. A permission
//! fingerprint is persisted only after the user has confirmed the preview.

use super::{
    external::{host::Permissions, ExternalAdapter, ResourceComponent},
    identity::SourceId,
    provider::ResourceProviderDescriptor,
    service::MusicService,
};
use crate::{infrastructure::http::Http, model::AppResult, storage::Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

const MAX_FILES: usize = 1024;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MANIFEST_BYTES: usize = 16 * 1024;
const AUTH_PREFIX: &str = "musicAdapterAuthorizationV1:";
const ENABLED_PREFIX: &str = "musicAdapterEnabledV1:";
const PROVIDER_ENABLED_PREFIX: &str = "resourceProviderEnabledV1:";

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Package {
    pub(crate) version: Option<String>,
    pub(crate) descriptor: Option<super::adapter::SourceDescriptor>,
    pub(crate) resource_provider: Option<ResourceProviderDescriptor>,
    pub(crate) permissions: Permissions,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionView {
    pub http_hosts: Vec<String>,
    pub websocket_hosts: Vec<String>,
    pub system: bool,
    pub state: bool,
    pub credentials: bool,
}
impl From<&Permissions> for PermissionView {
    fn from(value: &Permissions) -> Self {
        Self {
            http_hosts: value.http_hosts.clone(),
            websocket_hosts: value.websocket_hosts.clone(),
            system: value.system,
            state: value.state,
            credentials: value.credentials,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterPackageInfo {
    pub id: SourceId,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub kind: String,
    pub permissions: PermissionView,
    pub installed_version: Option<String>,
    pub permission_changes: Vec<String>,
    pub authorized: bool,
    pub loaded: bool,
    pub enabled: bool,
    pub load_error: Option<String>,
}

#[derive(Clone)]
struct PackageWithPath {
    path: PathBuf,
    package: Package,
}

pub(crate) struct AdapterInstaller {
    root: PathBuf,
    music: Arc<MusicService>,
    store: Arc<Store>,
    http: Arc<Http>,
    operations: tokio::sync::Mutex<()>,
}

impl AdapterInstaller {
    pub(crate) fn new(
        root: PathBuf,
        music: Arc<MusicService>,
        store: Arc<Store>,
        http: Arc<Http>,
    ) -> AppResult<Arc<Self>> {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            root,
            music,
            store,
            http,
            operations: tokio::sync::Mutex::new(()),
        }))
    }

    pub(crate) async fn startup(&self) {
        let _guard = self.operations.lock().await;
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) => {
                crate::infrastructure::diagnostics::warn("adapter.install_startup_read", error);
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                || entry.file_name().to_string_lossy().starts_with('.')
            {
                continue;
            }
            let info = match read_package(&path) {
                Ok(info) => info,
                Err(error) => {
                    crate::infrastructure::diagnostics::warn(
                        "adapter.install_startup_package",
                        error,
                    );
                    continue;
                }
            };
            if let Some(source) = source_of(&info.package) {
                if source == super::bundled::default_source() {
                    continue;
                }
                let begin = if info.package.descriptor.is_some() {
                    source_descriptor(&info.package)
                        .ok_or_else(|| "适配器描述缺失".to_owned())
                        .and_then(|descriptor| {
                            self.music
                                .adapters
                                .begin_loading(descriptor)
                                .map_err(|error| error.to_string())
                        })
                } else {
                    info.package
                        .resource_provider
                        .clone()
                        .ok_or_else(|| "音源描述缺失".to_owned())
                        .and_then(|descriptor| {
                            self.music
                                .adapters
                                .begin_resource_loading(descriptor)
                                .map_err(|error| error.to_string())
                        })
                };
                if let Err(error) = begin {
                    crate::infrastructure::diagnostics::warn(
                        "adapter.install_startup_begin",
                        error,
                    );
                    continue;
                }
                let authorized = self.is_authorized(&source, &info.package).unwrap_or(false);
                if !authorized {
                    self.loading_failed(&info.package, &source, "适配器尚未获得安装授权");
                    continue;
                }
                if let Err(error) = self.load_package(&info.path, &info.package, false).await {
                    self.loading_failed(&info.package, &source, &error);
                }
            }
        }
    }

    fn loading_failed(&self, package: &Package, source: &SourceId, error: &str) {
        if package.resource_provider.is_some() {
            self.music.adapters.resource_loading_failed(source, error);
        } else {
            self.music.adapters.loading_failed(source, error);
        }
    }

    pub(crate) fn preview(&self, source: &Path) -> AppResult<AdapterPackageInfo> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let directory = temp.path().join("package");
        install_source(source, &directory)?;
        let info = read_package(&directory)?;
        let installed_version = source_of(&info.package)
            .and_then(|id| read_package(&self.root.join(id.as_str())).ok())
            .and_then(|installed| installed.package.version);
        self.info(&info, installed_version)
    }

    pub(crate) async fn install(
        &self,
        source: PathBuf,
        authorized: bool,
    ) -> AppResult<AdapterPackageInfo> {
        let _guard = self.operations.lock().await;
        if !authorized {
            return Err("安装适配器前必须确认权限".into());
        }
        let staging = self.root.join(format!(".staging-{}", uuid::Uuid::new_v4()));
        let result = async {
            install_source(&source, &staging)?;
            let package = read_package(&staging)?;
            let id = source_of(&package.package).ok_or("适配器描述缺失")?;
            if id == super::bundled::default_source() {
                return Err("不能覆盖或安装内置网易云适配器".into());
            }
            let destination = self.root.join(id.as_str());
            let previous = if destination.exists() {
                Some(read_package(&destination)?)
            } else {
                None
            };
            let new_version =
                semver::Version::parse(package.package.version.as_deref().unwrap_or("0.0.0"))
                    .map_err(|_| "适配器版本无效")?;
            if let Some(previous) = &previous {
                let old_version =
                    semver::Version::parse(previous.package.version.as_deref().unwrap_or("0.0.0"))
                        .map_err(|_| "已安装适配器版本无效")?;
                if new_version <= old_version {
                    return Err("只能安装更高版本的适配器".into());
                }
                if std::mem::discriminant(&previous.package.descriptor)
                    != std::mem::discriminant(&package.package.descriptor)
                {
                    return Err("适配器类型不能在升级时改变".into());
                }
            }
            let old_auth = self.store.setting(&auth_key(&id))?;
            self.store.set_setting(
                &auth_key(&id),
                &permission_fingerprint(&package.package.permissions)?,
            )?;
            let backup =
                self.root
                    .join(format!(".backup-{}-{}", id.as_str(), uuid::Uuid::new_v4()));
            if destination.exists() {
                move_directory(&destination, &backup)
                    .map_err(|error| format!("已安装适配器备份失败：{error}"))?;
            }
            if let Err(error) = move_directory(&staging, &destination) {
                if destination.exists() {
                    let _ = fs::remove_dir_all(&destination);
                }
                if backup.exists() {
                    let _ = move_directory(&backup, &destination);
                }
                restore_setting(&self.store, &auth_key(&id), old_auth.as_deref())?;
                return Err(format!("适配器包移动失败：{error}"));
            }
            let replacing = previous.as_ref().is_some_and(|_| self.is_loaded(&id));
            if let Err(error) = self
                .load_package(&destination, &package.package, replacing)
                .await
            {
                let _ = fs::remove_dir_all(&destination);
                if backup.exists() {
                    let _ = move_directory(&backup, &destination);
                }
                restore_setting(&self.store, &auth_key(&id), old_auth.as_deref())?;
                return Err(format!("适配器 Component 加载失败：{error}"));
            }
            if backup.exists() {
                let _ = fs::remove_dir_all(&backup);
            }
            Ok(self.info(
                &PackageWithPath {
                    path: destination,
                    package: package.package,
                },
                previous.and_then(|item| item.package.version),
            )?)
        }
        .await;
        if result.is_err() && staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub(crate) async fn uninstall(&self, id: SourceId, remove_data: bool) -> AppResult<()> {
        let _guard = self.operations.lock().await;
        if id == super::bundled::default_source() {
            return Err("不能卸载内置网易云适配器".into());
        }
        let destination = self.root.join(id.as_str());
        let package = read_package(&destination)?;
        let trash = self
            .root
            .join(format!(".remove-{}-{}", id.as_str(), uuid::Uuid::new_v4()));
        move_directory(&destination, &trash)?;
        let was_loaded = self.is_loaded(&id);
        let result = if package.package.descriptor.is_some() {
            self.music.adapters.remove(&id).map_err(|e| e.to_string())
        } else {
            self.music
                .adapters
                .remove_resource_provider(&id)
                .map_err(|e| e.to_string())
        };
        if let Err(error) = result {
            if was_loaded {
                let _ = move_directory(&trash, &destination);
                return Err(error);
            }
        }
        if let Err(error) = fs::remove_dir_all(&trash) {
            let _ = move_directory(&trash, &destination);
            return Err(error.to_string());
        }
        if remove_data {
            if let Some(accounts) = &self.music.accounts {
                for record in accounts.records(&id).map_err(|e| e.to_string())? {
                    accounts
                        .remove(&record.reference)
                        .map_err(|e| e.to_string())?;
                }
            }
            self.store
                .remove_setting(&format!("musicAdapterStateV1:{}", id.as_str()))?;
        }
        self.store.remove_setting(&auth_key(&id))?;
        self.store
            .remove_setting(&format!("{ENABLED_PREFIX}{}", id.as_str()))?;
        self.store
            .remove_setting(&format!("{PROVIDER_ENABLED_PREFIX}{}", id.as_str()))?;
        Ok(())
    }

    pub(crate) fn list(&self) -> AppResult<Vec<AdapterPackageInfo>> {
        let mut result = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir()
                || entry.file_name().to_string_lossy().starts_with('.')
            {
                continue;
            }
            if let Ok(info) = read_package(&entry.path()) {
                result.push(self.info(
                    &info,
                    Some(info.package.version.clone().unwrap_or_default()),
                )?);
            }
        }
        result.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(result)
    }

    fn info(
        &self,
        info: &PackageWithPath,
        installed_version: Option<String>,
    ) -> AppResult<AdapterPackageInfo> {
        let id = source_of(&info.package).ok_or("适配器描述缺失")?;
        let descriptor = source_descriptor(&info.package);
        let version = info.package.version.clone().ok_or("适配器版本缺失")?;
        let (loaded, enabled, load_error) = if info.package.descriptor.is_some() {
            self.music
                .adapters
                .statuses()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|s| s.descriptor.source == id)
                .map(|s| {
                    (
                        !s.loading && s.load_error.is_none(),
                        s.enabled,
                        s.load_error,
                    )
                })
                .unwrap_or((false, false, None))
        } else {
            self.music
                .adapters
                .resource_provider_statuses()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|s| s.descriptor.id == id)
                .map(|s| {
                    (
                        !s.loading && s.load_error.is_none(),
                        s.enabled,
                        s.load_error,
                    )
                })
                .unwrap_or((false, false, None))
        };
        let permission_changes = if self.root.join(id.as_str()) != info.path
            && self.root.join(id.as_str()).is_dir()
        {
            read_package(&self.root.join(id.as_str()))
                .ok()
                .map(|old| permission_changes(&old.package.permissions, &info.package.permissions))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(AdapterPackageInfo {
            id,
            name: descriptor
                .as_ref()
                .map(|d| d.display_name.clone())
                .unwrap_or_else(|| {
                    info.package
                        .resource_provider
                        .as_ref()
                        .map(|d| d.display_name.clone())
                        .unwrap_or_default()
                }),
            description: descriptor.and_then(|d| d.description).or_else(|| {
                info.package
                    .resource_provider
                    .as_ref()
                    .and_then(|d| d.description.clone())
            }),
            version,
            kind: if info.package.descriptor.is_some() {
                "adapter"
            } else {
                "provider"
            }
            .into(),
            permissions: (&info.package.permissions).into(),
            installed_version,
            permission_changes,
            authorized: self.is_authorized(&source_of(&info.package).unwrap(), &info.package)?,
            loaded,
            enabled,
            load_error,
        })
    }

    fn is_loaded(&self, id: &SourceId) -> bool {
        self.music
            .adapters
            .statuses()
            .map(|v| {
                v.into_iter()
                    .any(|s| s.descriptor.source == *id && !s.loading && s.load_error.is_none())
            })
            .unwrap_or(false)
            || self
                .music
                .adapters
                .resource_provider_statuses()
                .map(|v| {
                    v.into_iter()
                        .any(|s| s.descriptor.id == *id && !s.loading && s.load_error.is_none())
                })
                .unwrap_or(false)
    }

    fn is_authorized(&self, id: &SourceId, package: &Package) -> AppResult<bool> {
        Ok(self.store.setting(&auth_key(id))?.as_deref()
            == Some(permission_fingerprint(&package.permissions)?.as_str()))
    }

    async fn load_package(
        &self,
        directory: &Path,
        package: &Package,
        replacing: bool,
    ) -> AppResult<()> {
        let accounts = self.music.accounts.clone().ok_or("账号服务未就绪")?;
        if let Some(descriptor) = &package.resource_provider {
            let host_identity = super::adapter::SourceDescriptor {
                source: descriptor.id.clone(),
                display_name: descriptor.display_name.clone(),
                description: descriptor.description.clone(),
                platforms: Default::default(),
                contract_version: descriptor.contract_version,
                login_methods: vec![],
                capabilities: Default::default(),
                cover_hosts: descriptor.cover_hosts.clone(),
                share_link_templates: Default::default(),
            };
            let services = Arc::new(super::external::host::Services::new(
                accounts,
                Arc::downgrade(&self.music.adapters),
                self.store.clone(),
                self.http.clone(),
                package.permissions.clone(),
                host_identity,
            )?);
            let adapter =
                ResourceComponent::load(&component_path(directory)?, descriptor.clone(), services)
                    .await
                    .map_err(|e| e.to_string())?;
            let enabled = self
                .store
                .setting(&format!(
                    "{PROVIDER_ENABLED_PREFIX}{}",
                    descriptor.id.as_str()
                ))?
                .as_deref()
                != Some("false");
            if replacing {
                self.music
                    .adapters
                    .replace_resource_provider_with_enabled(Arc::new(adapter), enabled)
                    .map_err(|e| e.to_string())?;
            } else {
                self.music
                    .adapters
                    .register_resource_provider_with_enabled(Arc::new(adapter), enabled)
                    .map_err(|e| e.to_string())?;
            }
        } else if let Some(descriptor) = &package.descriptor {
            let services = Arc::new(super::external::host::Services::new(
                accounts.clone(),
                Arc::downgrade(&self.music.adapters),
                self.store.clone(),
                self.http.clone(),
                package.permissions.clone(),
                descriptor.clone(),
            )?);
            let adapter = ExternalAdapter::load_with_host(&component_path(directory)?, services)
                .await
                .map_err(|e| e.to_string())?;
            let source = descriptor.source.clone();
            let session_accounts = accounts.clone();
            let session_source = source.clone();
            let session = Arc::new(move || session_accounts.session(&session_source));
            if replacing {
                self.music
                    .adapters
                    .replace(Arc::new(adapter), session)
                    .map_err(|e| e.to_string())?;
            } else {
                self.music
                    .adapters
                    .register(Arc::new(adapter), session)
                    .map_err(|e| e.to_string())?;
            }
            if self
                .store
                .setting(&format!("{ENABLED_PREFIX}{}", source.as_str()))?
                .as_deref()
                == Some("false")
            {
                self.music
                    .adapters
                    .set_enabled(&source, false)
                    .map_err(|e| e.to_string())?;
            }
        } else {
            return Err("适配器描述缺失".into());
        }
        Ok(())
    }
}

fn source_of(package: &Package) -> Option<SourceId> {
    package
        .descriptor
        .as_ref()
        .map(|d| d.source.clone())
        .or_else(|| package.resource_provider.as_ref().map(|d| d.id.clone()))
}
fn source_descriptor(package: &Package) -> Option<super::adapter::SourceDescriptor> {
    package.descriptor.clone().or_else(|| {
        package
            .resource_provider
            .as_ref()
            .map(|d| super::adapter::SourceDescriptor {
                source: d.id.clone(),
                display_name: d.display_name.clone(),
                description: d.description.clone(),
                platforms: Default::default(),
                contract_version: d.contract_version,
                login_methods: vec![],
                capabilities: Default::default(),
                cover_hosts: d.cover_hosts.clone(),
                share_link_templates: Default::default(),
            })
    })
}
fn auth_key(id: &SourceId) -> String {
    format!("{AUTH_PREFIX}{}", id.as_str())
}
fn permission_fingerprint(permissions: &Permissions) -> AppResult<String> {
    let bytes = serde_json::to_vec(permissions).map_err(|e| e.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
fn permission_changes(old: &Permissions, new: &Permissions) -> Vec<String> {
    let mut changes = Vec::new();
    if old.http_hosts != new.http_hosts {
        changes.push("网络请求".to_owned());
    }
    if old.websocket_hosts != new.websocket_hosts {
        changes.push("长连接".to_owned());
    }
    if old.credentials != new.credentials {
        changes.push("账号凭据".to_owned());
    }
    if old.state != new.state {
        changes.push("专属状态".to_owned());
    }
    if old.system != new.system {
        changes.push("系统能力".to_owned());
    }
    changes
}
fn restore_setting(store: &Store, key: &str, value: Option<&str>) -> AppResult<()> {
    match value {
        Some(value) => store.set_setting(key, value),
        None => store.remove_setting(key),
    }
}

fn read_package(directory: &Path) -> AppResult<PackageWithPath> {
    let meta =
        fs::symlink_metadata(directory).map_err(|error| format!("适配器包目录不可读：{error}"))?;
    if !meta.is_dir() || is_link(&meta) {
        return Err("适配器包目录无效".into());
    }
    let manifest = directory.join("manifest.json");
    let mut bytes = Vec::new();
    fs::File::open(&manifest)
        .map_err(|error| format!("适配器包缺少 manifest.json：{error}"))?
        .take((MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MANIFEST_BYTES {
        return Err("适配器描述过大".into());
    }
    let package: Package = serde_json::from_slice(&bytes).map_err(|_| "适配器描述无效")?;
    validate_permissions(&package.permissions)?;
    let version = package.version.as_deref().ok_or("适配器版本缺失")?;
    semver::Version::parse(version).map_err(|_| "适配器版本无效")?;
    match (&package.descriptor, &package.resource_provider) {
        (Some(d), None) => d.validate().map_err(|e| e.to_string())?,
        (None, Some(d)) => {
            d.validate().map_err(|e| e.to_string())?;
            if package.permissions.credentials
                || package.permissions.state
                || !package.permissions.websocket_hosts.is_empty()
            {
                return Err("音源适配器不能申请账号、状态或长连接权限".into());
            }
        }
        _ => return Err("适配器必须声明一种描述".into()),
    }
    let component = component_path(directory)?;
    let component_meta = fs::symlink_metadata(&component)
        .map_err(|error| format!("适配器 Component 不可读：{error}"))?;
    if !component_meta.is_file()
        || is_link(&component_meta)
        || component_meta.len() > crate::wasm_runtime::Budget::ADAPTER.component_bytes as u64
    {
        return Err("适配器 Component 无效或过大".into());
    }
    Ok(PackageWithPath {
        path: directory.to_path_buf(),
        package,
    })
}
fn component_path(directory: &Path) -> AppResult<PathBuf> {
    let direct = directory.join("adapter.component.wasm");
    if direct.exists() {
        return Ok(direct);
    }
    let dist = directory.join("dist/adapter.component.wasm");
    if dist.exists() {
        return Ok(dist);
    }
    Err("适配器包缺少 adapter.component.wasm（应位于包根或 dist/）".into())
}
fn validate_permissions(permissions: &Permissions) -> AppResult<()> {
    for hosts in [&permissions.http_hosts, &permissions.websocket_hosts] {
        if hosts.len() > 32
            || hosts
                .iter()
                .any(|host| !nons_music_adapter_sdk::cover::valid_host_rule(host) || host == "*")
        {
            return Err("适配器网络授权范围无效".into());
        }
    }
    Ok(())
}
fn is_link(meta: &fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}
fn safe_relative(value: &str) -> AppResult<PathBuf> {
    let normalized = value.replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') || normalized.contains(':') {
        return Err("包内路径无效".into());
    }
    let mut output = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => output.push(part),
            _ => return Err("包内路径无效".into()),
        }
    }
    if output.as_os_str().is_empty() {
        Err("包内路径无效".into())
    } else {
        Ok(output)
    }
}
fn install_source(source: &Path, target: &Path) -> AppResult<()> {
    fs::create_dir(target).map_err(|e| e.to_string())?;
    let result = (|| {
        let meta =
            fs::symlink_metadata(source).map_err(|error| format!("安装来源不可读：{error}"))?;
        if is_link(&meta) {
            return Err("安装来源不允许链接".into());
        }
        if meta.is_dir() {
            copy_directory(source, target)
        } else {
            extract_zip(source, target)
        }
    })()
    .and_then(|_| normalize_package_root(target));
    if result.is_err() {
        let _ = fs::remove_dir_all(target);
    }
    result
}
fn normalize_package_root(target: &Path) -> AppResult<()> {
    if target.join("manifest.json").is_file() {
        return Ok(());
    }
    let entries = fs::read_dir(target)
        .map_err(|error| format!("适配器包根目录不可读：{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("适配器包根目录项不可读：{error}"))?;
    if entries.len() != 1 || !entries[0].file_type().map_err(|e| e.to_string())?.is_dir() {
        return Err("适配器包根目录缺少 manifest.json".into());
    }
    let nested = entries[0].path();
    if is_link(&fs::symlink_metadata(&nested).map_err(|e| e.to_string())?) {
        return Err("适配器包根目录不允许链接".into());
    }
    copy_directory(&nested, target)?;
    fs::remove_dir_all(&nested).map_err(|e| e.to_string())?;
    Ok(())
}
fn move_directory(source: &Path, target: &Path) -> AppResult<()> {
    if target.exists() {
        return Err("目标适配器目录已存在".into());
    }
    fs::create_dir_all(target).map_err(|e| e.to_string())?;
    if let Err(error) = copy_directory(source, target) {
        let _ = fs::remove_dir_all(target);
        return Err(error);
    }
    if let Err(error) = fs::remove_dir_all(source) {
        let _ = fs::remove_dir_all(target);
        return Err(error.to_string());
    }
    Ok(())
}
fn copy_directory(source: &Path, target: &Path) -> AppResult<()> {
    let mut files = 0usize;
    let mut bytes = 0u64;
    for item in walkdir::WalkDir::new(source)
        .follow_links(false)
        .max_open(8)
    {
        let item = item.map_err(|e| e.to_string())?;
        if item.path() == source {
            continue;
        }
        let meta = fs::symlink_metadata(item.path()).map_err(|e| e.to_string())?;
        if is_link(&meta) {
            return Err("适配器包不允许链接".into());
        }
        files += 1;
        bytes = bytes.checked_add(meta.len()).ok_or("适配器包过大")?;
        if files > MAX_FILES || bytes > MAX_BYTES {
            return Err("适配器包超过文件数或容量上限".into());
        }
        let relative = item
            .path()
            .strip_prefix(source)
            .map_err(|e| e.to_string())?;
        let destination = target.join(safe_relative(&relative.to_string_lossy())?);
        if meta.is_dir() {
            fs::create_dir_all(destination).map_err(|e| e.to_string())?;
        } else if meta.is_file() {
            fs::create_dir_all(destination.parent().ok_or("文件路径无效")?)
                .map_err(|e| e.to_string())?;
            let mut input = fs::File::open(item.path())
                .map_err(|e| e.to_string())?
                .take(meta.len() + 1);
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|e| e.to_string())?;
            if std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())? != meta.len() {
                return Err("安装时源文件发生变化".into());
            }
        } else {
            return Err("适配器包包含非常规文件".into());
        }
    }
    Ok(())
}
fn extract_zip(source: &Path, target: &Path) -> AppResult<()> {
    let file = fs::File::open(source).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("压缩包过大".into());
    }
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    if zip.len() > MAX_FILES {
        return Err("压缩包文件数过多".into());
    }
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("压缩包不允许链接".into());
        }
        let name = entry.name().trim_end_matches('/');
        let destination = target.join(safe_relative(name)?);
        total = total.checked_add(entry.size()).ok_or("压缩包过大")?;
        if total > MAX_BYTES {
            return Err("压缩包解压容量过大".into());
        }
        if entry.is_dir() {
            fs::create_dir_all(destination).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(destination.parent().ok_or("文件路径无效")?)
                .map_err(|e| e.to_string())?;
            let expected = entry.size();
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|e| e.to_string())?;
            let actual = std::io::copy(&mut (&mut entry).take(expected + 1), &mut output)
                .map_err(|e| e.to_string())?;
            if actual != expected {
                return Err("压缩包大小无效".into());
            }
            output.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn adapter_package_preview(
    path: String,
    installer: tauri::State<'_, Arc<AdapterInstaller>>,
) -> AppResult<AdapterPackageInfo> {
    let installer = installer.inner().clone();
    tauri::async_runtime::spawn_blocking(move || installer.preview(Path::new(&path)))
        .await
        .map_err(|error| error.to_string())?
}
#[tauri::command]
pub async fn adapter_install(
    path: String,
    authorized: bool,
    installer: tauri::State<'_, Arc<AdapterInstaller>>,
    app: tauri::AppHandle,
) -> AppResult<AdapterPackageInfo> {
    let result = installer.install(path.into(), authorized).await?;
    use tauri::Emitter;
    let _ = app.emit("adapters-changed", ());
    Ok(result)
}
#[tauri::command]
pub async fn adapter_uninstall(
    source: SourceId,
    remove_data: bool,
    installer: tauri::State<'_, Arc<AdapterInstaller>>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    installer.uninstall(source, remove_data).await?;
    use tauri::Emitter;
    let _ = app.emit("adapters-changed", ());
    Ok(())
}
#[tauri::command]
pub async fn adapter_installed_list(
    installer: tauri::State<'_, Arc<AdapterInstaller>>,
) -> AppResult<Vec<AdapterPackageInfo>> {
    let installer = installer.inner().clone();
    tauri::async_runtime::spawn_blocking(move || installer.list())
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn package_paths_reject_traversal_and_absolute_names() {
        for path in ["../escape", "/absolute", "C:/absolute", "a/../b"] {
            assert!(safe_relative(path).is_err(), "accepted unsafe path {path}");
        }
        assert_eq!(
            safe_relative("assets/icon.png").unwrap(),
            PathBuf::from("assets/icon.png")
        );
    }

    #[test]
    fn zip_import_rejects_traversal_and_removes_staging() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("adapter.zip");
        let mut zip = ZipWriter::new(fs::File::create(&archive).unwrap());
        zip.start_file("../escape", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"bad").unwrap();
        zip.finish().unwrap();
        let staging = root.path().join("staging");
        assert!(install_source(&archive, &staging).is_err());
        assert!(!staging.exists());
        assert!(!root.path().join("escape").exists());
    }

    #[test]
    fn permission_changes_identify_each_changed_capability() {
        let old = Permissions {
            http_hosts: vec!["old.example".into()],
            websocket_hosts: vec![],
            system: false,
            state: false,
            credentials: false,
        };
        let new = Permissions {
            http_hosts: vec!["new.example".into()],
            websocket_hosts: vec!["socket.example".into()],
            system: true,
            state: true,
            credentials: true,
        };
        assert_eq!(
            permission_changes(&old, &new),
            vec!["网络请求", "长连接", "账号凭据", "专属状态", "系统能力"]
        );
    }

    #[test]
    fn package_component_accepts_the_published_dist_layout() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("adapter");
        fs::create_dir_all(package.join("dist")).unwrap();
        fs::write(
            package.join("manifest.json"),
            serde_json::json!({
                "version": "1.0.0",
                "resourceProvider": {
                    "id": "fixture",
                    "displayName": "Fixture",
                    "contractVersion": 1,
                    "platforms": ["netease"],
                    "searchPlatforms": [],
                    "coverHosts": []
                },
                "permissions": {
                    "httpHosts": [],
                    "websocketHosts": [],
                    "system": false,
                    "state": false,
                    "credentials": false
                }
            })
            .to_string(),
        )
        .unwrap();
        fs::write(package.join("dist/adapter.component.wasm"), b"component").unwrap();
        assert!(read_package(&package).is_ok());
    }

    #[test]
    fn zip_with_one_top_level_folder_is_flattened_before_validation() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("adapter.zip");
        let mut zip = ZipWriter::new(fs::File::create(&archive).unwrap());
        for (name, bytes) in [
            ("qqmusic/manifest.json", br#"{}"#.as_slice()),
            ("qqmusic/adapter.component.wasm", b"component".as_slice()),
        ] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        let staging = root.path().join("staging");
        install_source(&archive, &staging).unwrap();
        assert!(staging.join("manifest.json").is_file());
        assert!(staging.join("adapter.component.wasm").is_file());
        assert!(!staging.join("qqmusic").exists());
    }

    #[tokio::test]
    async fn provider_startup_failure_finishes_loading_for_authorization_and_component_errors() {
        use crate::music::{manager::AdapterManager, service::MusicService};
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("state.sqlite3")).unwrap());
        let adapters = Arc::new(AdapterManager::default());
        let installer = AdapterInstaller::new(
            directory.path().join("adapters"),
            Arc::new(MusicService::new(adapters.clone())),
            store.clone(),
            Arc::new(Http::new().unwrap()),
        )
        .unwrap();
        let package = installer.root.join("fixture");
        fs::create_dir(&package).unwrap();
        fs::write(package.join("manifest.json"), serde_json::json!({
            "version": "1.0.0",
            "resourceProvider": { "id": "fixture", "displayName": "Fixture", "contractVersion": 1,
                "platforms": ["netease"], "searchPlatforms": [], "coverHosts": [] },
            "permissions": { "httpHosts": [], "websocketHosts": [], "system": false,
                "state": false, "credentials": false }
        }).to_string()).unwrap();
        fs::write(package.join("adapter.component.wasm"), b"invalid component").unwrap();
        installer.startup().await;
        let status = adapters
            .resource_provider_statuses()
            .unwrap()
            .pop()
            .unwrap();
        assert!(!status.loading && !status.enabled);
        assert!(status.load_error.unwrap().contains("授权"));
        let parsed = read_package(&package).unwrap();
        let id = source_of(&parsed.package).unwrap();
        store
            .set_setting(
                &auth_key(&id),
                &permission_fingerprint(&parsed.package.permissions).unwrap(),
            )
            .unwrap();
        installer.startup().await;
        let status = adapters
            .resource_provider_statuses()
            .unwrap()
            .pop()
            .unwrap();
        assert!(!status.loading && !status.enabled);
        assert!(status.load_error.is_some());
    }

    #[tokio::test]
    #[ignore = "Requires NONS_REAL_ADAPTER_ZIP; installs into isolated temporary storage"]
    async fn real_downloaded_package_can_install_and_restore() {
        use crate::music::{
            accounts::{AccountManager, CredentialStore},
            MusicResult,
        };

        #[derive(Default)]
        struct Secrets(std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>);
        impl CredentialStore for Secrets {
            fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
                Ok(self.0.lock().unwrap().get(key).cloned())
            }
            fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()> {
                self.0.lock().unwrap().insert(key.into(), bytes.to_vec());
                Ok(())
            }
            fn delete(&self, key: &str) -> MusicResult<()> {
                self.0.lock().unwrap().remove(key);
                Ok(())
            }
        }

        let source = PathBuf::from(
            std::env::var_os("NONS_REAL_ADAPTER_ZIP").expect("set NONS_REAL_ADAPTER_ZIP"),
        );
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("accounts.sqlite3")).unwrap());
        let secrets = Arc::new(Secrets::default());
        let http = Arc::new(Http::new().unwrap());
        let create = || {
            let accounts = Arc::new(AccountManager::new(store.clone(), secrets.clone()).unwrap());
            let (music, _) = crate::music::bundled::service(accounts).unwrap();
            AdapterInstaller::new(
                directory.path().join("adapters"),
                Arc::new(music),
                store.clone(),
                http.clone(),
            )
            .unwrap()
        };
        let installer = create();
        let preview = installer.preview(&source).unwrap();
        assert!(!preview.loaded && !preview.authorized);
        assert!(installer.install(source.clone(), false).await.is_err());
        assert!(installer.list().unwrap().is_empty());
        let installed = installer.install(source.clone(), true).await.unwrap();
        assert_eq!(installed.id, preview.id);
        assert_eq!(installed.version, preview.version);
        assert!(installed.loaded && installed.authorized);
        assert!(component_path(&installer.root.join(installed.id.as_str()))
            .unwrap()
            .is_file());
        assert!(installer
            .install(source, true)
            .await
            .err()
            .unwrap()
            .contains("更高版本"));

        let broken = directory.path().join("broken-upgrade");
        fs::create_dir(&broken).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(
                installer
                    .root
                    .join(installed.id.as_str())
                    .join("manifest.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut version = semver::Version::parse(&installed.version).unwrap();
        version.major += 1;
        manifest["version"] = serde_json::Value::String(version.to_string());
        fs::write(
            broken.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(broken.join("adapter.component.wasm"), b"invalid component").unwrap();
        assert!(installer
            .install(broken, true)
            .await
            .err()
            .unwrap()
            .contains("Component"));
        let retained = installer.list().unwrap().pop().unwrap();
        assert_eq!(retained.version, installed.version);
        assert!(retained.loaded && retained.authorized);
        drop(installer);

        let restored = create();
        restored.startup().await;
        let packages = restored.list().unwrap();
        assert_eq!(packages.len(), 1);
        assert!(packages[0].loaded && packages[0].authorized);
        assert_eq!(packages[0].id, installed.id);
        restored
            .uninstall(installed.id.clone(), false)
            .await
            .unwrap();
        assert!(restored.list().unwrap().is_empty());
        assert!(!restored.is_loaded(&installed.id));
        assert!(store.setting(&auth_key(&installed.id)).unwrap().is_none());
        eprintln!(
            "real adapter lifecycle passed: {} v{}",
            installed.id.as_str(),
            installed.version
        );
    }
}
