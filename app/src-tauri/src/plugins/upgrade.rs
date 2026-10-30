use super::{
    database::AuthorizationState, install, manifest::Manifest, Descriptor, PluginManager, Record,
};
use crate::model::AppResult;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

// Covers import, export validation and activate(), not future page renders.
const FRONTEND_START_TIMEOUT: Duration = Duration::from_secs(15);

async fn wait_for_start(
    mut lifecycle: tokio::sync::watch::Receiver<()>,
    timeout: Duration,
    mut status: impl FnMut() -> AppResult<bool>,
) -> AppResult<()> {
    tokio::time::timeout(timeout, async {
        loop {
            if status()? {
                return Ok(());
            }
            lifecycle
                .changed()
                .await
                .map_err(|error| error.to_string())?;
        }
    })
    .await
    .map_err(|_| "插件前端启动确认超时".to_string())?
}

struct Package {
    workspace: tempfile::TempDir,
    manifest: Manifest,
}

impl Package {
    fn prepare(root: &Path, source: &Path) -> AppResult<Self> {
        let workspace = tempfile::Builder::new()
            .prefix(".install-")
            .tempdir_in(root)
            .map_err(|error| error.to_string())?;
        let manifest = install::install(source, &workspace.path().join("package"))?;
        Ok(Self {
            workspace,
            manifest,
        })
    }

    fn publish(&self, destination: &Path, replacing: bool) -> AppResult<()> {
        if replacing {
            std::fs::rename(destination, self.workspace.path().join("previous"))
                .map_err(|error| error.to_string())?;
        }
        std::fs::rename(self.workspace.path().join("package"), destination)
            .map_err(|error| error.to_string())
    }

    fn rollback(&self, destination: &Path) -> AppResult<()> {
        let previous = self.workspace.path().join("previous");
        // A failed initial rename leaves the original package in place.
        if previous.exists() {
            if destination.exists() {
                std::fs::rename(destination, self.workspace.path().join("rejected"))
                    .map_err(|error| error.to_string())?;
            }
            std::fs::rename(previous, destination).map_err(|error| error.to_string())?;
        } else if !self.workspace.path().join("package").exists() && destination.exists() {
            std::fs::rename(destination, self.workspace.path().join("rejected"))
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn check_upgrade(old: &Manifest, new: &Manifest, confirmed: bool) -> AppResult<()> {
    let old_version = semver::Version::parse(&old.version).map_err(|error| error.to_string())?;
    let new_version = semver::Version::parse(&new.version).map_err(|error| error.to_string())?;
    if !new_version.cmp_precedence(&old_version).is_gt() {
        return Err("插件升级只接受更高版本".into());
    }
    // Frontend trust and authorization are version-bound, even without new permissions.
    if !confirmed {
        return Err("升级需要确认：请先预览新版本、权限范围和前端信任说明，再确认安装".into());
    }
    Ok(())
}

fn check_reviewed(actual: &Manifest, expected: Option<&Manifest>) -> AppResult<()> {
    if let Some(expected) = expected {
        if actual.id != expected.id
            || actual.version != expected.version
            || actual.authorization() != expected.authorization()
        {
            return Err("插件包已在预览后改变，请重新预览并确认版本、权限和信任说明".into());
        }
    }
    Ok(())
}

impl PluginManager {
    pub(crate) fn preview(&self, source: &Path) -> AppResult<Manifest> {
        Ok(Package::prepare(&self.root, source)?.manifest.clone())
    }

    pub(crate) async fn install_reviewed(
        self: &Arc<Self>,
        source: PathBuf,
        confirmed: bool,
    ) -> AppResult<()> {
        self.install_reviewed_expected(source, confirmed, None)
            .await
    }

    pub(crate) async fn install_reviewed_expected(
        self: &Arc<Self>,
        source: PathBuf,
        confirmed: bool,
        expected_manifest: Option<Manifest>,
    ) -> AppResult<()> {
        let manager = self.clone();
        // Dropping an IPC future must not cancel a transaction after stopping the old instance.
        tauri::async_runtime::spawn(async move {
            let _operation = manager.operations.lock().await;
            manager
                .install_package(source, confirmed, expected_manifest)
                .await
        })
        .await
        .map_err(|error| error.to_string())?
    }

    fn register_package(&self, manifest: Manifest, enabled: bool) -> AppResult<()> {
        let id = manifest.id.clone();
        let (_, configuration) = Manifest::read_configured(&self.root.join(&id))?;
        self.configurations.register(&id, configuration)?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst);
        let context = self.context(&manifest, generation);
        self.lock()?.insert(
            id,
            Record {
                descriptor: Descriptor {
                    manifest,
                    enabled,
                    loaded: false,
                    generation,
                    error: None,
                },
                context,
                runtime: None,
                ready: false,
            },
        );
        Ok(())
    }

    async fn await_started(self: &Arc<Self>, id: &str) -> AppResult<()> {
        let generation = self
            .lock()?
            .get(id)
            .ok_or("插件未安装")?
            .descriptor
            .generation;
        wait_for_start(self.lifecycle.subscribe(), FRONTEND_START_TIMEOUT, || {
            let records = self.lock()?;
            let record = records.get(id).ok_or("插件未安装")?;
            if self.stopped.load(Ordering::SeqCst) {
                return Err("应用正在退出，插件启动未完成".into());
            }
            if record.descriptor.generation != generation || !record.descriptor.loaded {
                return Err(record
                    .descriptor
                    .error
                    .clone()
                    .unwrap_or_else(|| "插件启动失败".into()));
            }
            Ok(record.ready)
        })
        .await
    }

    async fn install_package(
        self: &Arc<Self>,
        source: PathBuf,
        confirmed: bool,
        expected_manifest: Option<Manifest>,
    ) -> AppResult<()> {
        let root = self.root.clone();
        let package =
            tauri::async_runtime::spawn_blocking(move || Package::prepare(&root, &source))
                .await
                .map_err(|error| error.to_string())??;
        check_reviewed(&package.manifest, expected_manifest.as_ref())?;
        let id = package.manifest.id.clone();
        let destination = self.root.join(&id);
        let previous = self
            .lock()?
            .get(&id)
            .map(|record| record.descriptor.clone());
        if let Some(previous) = &previous {
            check_upgrade(&previous.manifest, &package.manifest, confirmed)?;
        } else if destination.exists() {
            return Err("插件目录已存在但未登记，请先检查插件目录并重新发现".into());
        }
        let authorization = self.database.authorization_state(&id)?;
        if previous.is_some() {
            self.unload(&id).await?;
        }
        let result: AppResult<()> = async {
            package.publish(&destination, previous.is_some())?;
            let enabled = previous.as_ref().is_some_and(|old| old.enabled);
            self.database.restore_authorization(
                &id,
                &AuthorizationState {
                    enabled: Some(enabled),
                    scope: previous.as_ref().map(|_| package.manifest.authorization()),
                },
            )?;
            self.register_package(package.manifest.clone(), enabled)?;
            if previous.as_ref().is_some_and(|old| old.loaded) {
                self.load(&id).await?;
                self.sync_watcher()?;
                self.notify();
                self.await_started(&id).await?;
            }
            Ok(())
        }
        .await;
        if let Err(error) = result {
            let recovery = self
                .restore_package(&package, &destination, previous.as_ref(), &authorization)
                .await;
            self.notify();
            if let Err(recovery) = recovery {
                // Preserve the old package if filesystem/DB/runtime recovery itself fails.
                let retained = package.workspace.keep();
                return Err(format!(
                    "安装失败：{error}；恢复失败：{recovery}；安装恢复文件保留于 {}",
                    retained.display()
                ));
            }
            return Err(if previous.is_some() {
                format!("安装失败：{error}；已恢复原插件包、授权和加载状态")
            } else {
                format!("安装失败：{error}；已撤回安装并保留原有数据")
            });
        }
        self.notify();
        Ok(())
    }

    async fn restore_package(
        self: &Arc<Self>,
        package: &Package,
        destination: &Path,
        previous: Option<&Descriptor>,
        authorization: &AuthorizationState,
    ) -> AppResult<()> {
        if self.lock()?.contains_key(&package.manifest.id) {
            self.unload(&package.manifest.id).await?;
        }
        package.rollback(destination)?;
        self.database
            .restore_authorization(&package.manifest.id, authorization)?;
        if let Some(previous) = previous {
            self.register_package(previous.manifest.clone(), previous.enabled)?;
            if previous.loaded && !self.stopped.load(Ordering::SeqCst) {
                self.load(&previous.manifest.id).await?;
                self.sync_watcher()?;
                self.notify();
                self.await_started(&previous.manifest.id).await?;
            }
            if self.stopped.load(Ordering::SeqCst) && previous.loaded {
                return Err("原插件包及授权已恢复，应用退出期间未恢复运行实例".into());
            }
        } else {
            self.lock()?.remove(&package.manifest.id);
            self.configurations.register(&package.manifest.id, None)?;
        }
        self.sync_watcher()
    }
}

#[cfg(test)]
mod tests {
    use super::super::database::Database;
    use super::*;
    use std::fs;

    fn source(root: &Path, version: &str) -> PathBuf {
        let source = root.join(format!("source-{version}"));
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("manifest.json"),
            serde_json::json!({
                "id": "upgrade-test", "name": "Upgrade test", "version": version,
                "frontend": "ui.mjs", "permissions": ["ui", "storage"],
                "engines": {"app": "^1.0.0", "pluginApi": "^2.0.0", "uiApi": "^2.0.0"}
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            source.join("ui.mjs"),
            format!("export const version = '{version}';"),
        )
        .unwrap();
        source
    }

    #[test]
    fn upgrades_require_higher_precedence_and_explicit_version_trust() {
        let temp = tempfile::tempdir().unwrap();
        let old = Package::prepare(temp.path(), &source(temp.path(), "1.0.0")).unwrap();
        for version in ["0.9.0", "1.0.0", "1.0.0+rebuilt", "1.0.0-rc.1"] {
            let new = Package::prepare(temp.path(), &source(temp.path(), version)).unwrap();
            assert!(check_upgrade(&old.manifest, &new.manifest, true).is_err());
        }
        let mut new = Package::prepare(temp.path(), &source(temp.path(), "1.0.1")).unwrap();
        assert!(check_upgrade(&old.manifest, &new.manifest, false)
            .unwrap_err()
            .contains("确认"));
        assert!(check_upgrade(&old.manifest, &new.manifest, true).is_ok());
        new.manifest.permissions.push("account:credentials".into());
        new.manifest.http_hosts.push("*".into());
        new.manifest.file_roots.push("*".into());
        assert!(check_upgrade(&old.manifest, &new.manifest, false).is_err());
        assert!(check_upgrade(&old.manifest, &new.manifest, true).is_ok());
    }

    #[test]
    fn reviewed_manifest_rejects_swapped_identity_version_and_authorization() {
        let temp = tempfile::tempdir().unwrap();
        let package = Package::prepare(temp.path(), &source(temp.path(), "1.1.0")).unwrap();
        let expected = package.manifest;
        check_reviewed(&expected, Some(&expected)).unwrap();
        for change in ["id", "version", "permissions", "hosts", "roots", "windows"] {
            let mut actual = expected.clone();
            match change {
                "id" => actual.id = "swapped".into(),
                "version" => actual.version = "1.2.0".into(),
                "permissions" => actual.permissions.push("account:credentials".into()),
                "hosts" => actual.http_hosts.push("*".into()),
                "roots" => actual.file_roots.push("*".into()),
                "windows" => {
                    actual
                        .contributes
                        .windows
                        .push(super::super::manifest::PluginWindow {
                            id: "new-window".into(),
                            export: "View".into(),
                        })
                }
                _ => unreachable!(),
            }
            assert!(
                check_reviewed(&actual, Some(&expected)).is_err(),
                "{change}"
            );
        }
    }

    #[test]
    fn corrupt_package_and_failed_publication_keep_old_package() {
        let temp = tempfile::tempdir().unwrap();
        let old = source(temp.path(), "1.0.0");
        let destination = temp.path().join("upgrade-test");
        Package::prepare(temp.path(), &old)
            .unwrap()
            .publish(&destination, false)
            .unwrap();
        let bad = source(temp.path(), "2.0.0");
        fs::remove_file(bad.join("ui.mjs")).unwrap();
        assert!(Package::prepare(temp.path(), &bad).is_err());
        assert_eq!(Manifest::read(&destination).unwrap().version, "1.0.0");

        let candidate = Package::prepare(temp.path(), &source(temp.path(), "1.1.0")).unwrap();
        // Simulate a publication failure after the old package was renamed.
        fs::rename(
            candidate.workspace.path().join("package"),
            candidate.workspace.path().join("unavailable"),
        )
        .unwrap();
        assert!(candidate.publish(&destination, true).is_err());
        candidate.rollback(&destination).unwrap();
        assert_eq!(Manifest::read(&destination).unwrap().version, "1.0.0");
    }

    #[test]
    fn replacement_and_rollback_preserve_data_grants_credentials_and_authorization() {
        let temp = tempfile::tempdir().unwrap();
        let old = Package::prepare(temp.path(), &source(temp.path(), "1.0.0")).unwrap();
        let destination = temp.path().join("upgrade-test");
        old.publish(&destination, false).unwrap();
        let id = &old.manifest.id;
        let db = Database::open(&temp.path().join("plugins.sqlite3")).unwrap();
        db.authorize(id, &old.manifest.authorization()).unwrap();
        db.enable(id, false).unwrap();
        db.set(id, "task", "{\"progress\":7}").unwrap();
        db.config_change(id, 0, |values| {
            values.insert("directory".into(), serde_json::json!("selected"));
            Ok(())
        })
        .unwrap();
        db.grant(id, "selected", "C:/music", true).unwrap();
        db.secret_key(id, "credential", true).unwrap();
        let data = temp.path().join("plugin-data");
        fs::create_dir(&data).unwrap();
        fs::write(data.join("output"), b"user data").unwrap();
        let config = db.config_read(id).unwrap();
        let grants = db.grants(id).unwrap();
        let authorization = db.authorization_state(id).unwrap();
        let new = Package::prepare(temp.path(), &source(temp.path(), "1.1.0")).unwrap();
        new.publish(&destination, true).unwrap();
        db.restore_authorization(
            id,
            &AuthorizationState {
                enabled: Some(false),
                scope: Some(new.manifest.authorization()),
            },
        )
        .unwrap();
        assert!(!db.enabled(id, &new.manifest.authorization()).unwrap());
        assert_eq!(Manifest::read(&destination).unwrap().version, "1.1.0");
        new.rollback(&destination).unwrap();
        db.restore_authorization(id, &authorization).unwrap();
        assert_eq!(db.authorization_state(id).unwrap(), authorization);
        assert_eq!(Manifest::read(&destination).unwrap().version, "1.0.0");
        assert_eq!(
            db.get(id, "task").unwrap().as_deref(),
            Some("{\"progress\":7}")
        );
        assert_eq!(db.config_read(id).unwrap(), config);
        assert_eq!(db.grants(id).unwrap(), grants);
        assert_eq!(db.secret_keys(id).unwrap(), vec!["credential"]);
        assert_eq!(fs::read(data.join("output")).unwrap(), b"user data");
    }

    #[tokio::test]
    async fn startup_wait_observes_ready_and_fault_including_early_notifications() {
        let (sender, receiver) = tokio::sync::watch::channel(());
        let state = Arc::new(std::sync::Mutex::new(Ok(false)));
        let ready_state = state.clone();
        let waiter = wait_for_start(receiver, Duration::from_secs(1), || {
            state.lock().unwrap().clone()
        });
        let signal = async {
            tokio::task::yield_now().await;
            *ready_state.lock().unwrap() = Ok(true);
            sender.send_replace(());
        };
        let (result, ()) = tokio::join!(waiter, signal);
        result.unwrap();
        let receiver = sender.subscribe();
        sender.send_replace(());
        assert_eq!(
            wait_for_start(receiver, Duration::from_secs(1), || {
                Err("frontend fault".into())
            })
            .await
            .unwrap_err(),
            "frontend fault"
        );
        wait_for_start(sender.subscribe(), Duration::from_secs(1), || Ok(true))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn startup_wait_does_not_commit_without_frontend_acknowledgement() {
        let (_sender, receiver) = tokio::sync::watch::channel(());
        assert!(
            wait_for_start(receiver, Duration::from_millis(10), || Ok(false))
                .await
                .unwrap_err()
                .contains("超时")
        );
    }
}
