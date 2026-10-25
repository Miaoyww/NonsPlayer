use super::*;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
#[derive(Default)]
struct Records {
    json: Mutex<Option<String>>,
    fail: AtomicBool,
}
impl AccountRecordStore for Records {
    fn load(&self) -> MusicResult<Option<String>> {
        Ok(self.json.lock().unwrap().clone())
    }
    fn save(&self, json: &str) -> MusicResult<()> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(ErrorCode::Internal.into());
        }
        *self.json.lock().unwrap() = Some(json.into());
        Ok(())
    }
}
#[derive(Default)]
struct Secrets {
    values: Mutex<BTreeMap<String, Vec<u8>>>,
    fail_write: AtomicBool,
    fail_delete: AtomicBool,
    fail_read: AtomicBool,
    after_write: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}
impl CredentialStore for Secrets {
    fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
        if self.fail_read.load(Ordering::SeqCst) {
            return Err(ErrorCode::Internal.into());
        }
        Ok(self.values.lock().unwrap().get(key).cloned())
    }
    fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()> {
        if self.fail_write.load(Ordering::SeqCst) {
            return Err(ErrorCode::Internal.into());
        }
        self.values.lock().unwrap().insert(key.into(), bytes.into());
        let callback = self.after_write.lock().unwrap().take();
        if let Some(callback) = callback {
            callback();
        }
        Ok(())
    }
    fn delete(&self, key: &str) -> MusicResult<()> {
        if self.fail_delete.load(Ordering::SeqCst) {
            return Err(ErrorCode::Internal.into());
        }
        self.values.lock().unwrap().remove(key);
        Ok(())
    }
}
fn source(value: &str) -> SourceId {
    SourceId::try_from(value.to_owned()).unwrap()
}
fn record(source_id: &str, id: &str) -> AccountRecord {
    AccountRecord {
        reference: AccountRef {
            source: source(source_id),
            id: OpaqueId::try_from(id.to_owned()).unwrap(),
        },
        display_name: "user".into(),
        avatar: Default::default(),
    }
}
fn secret(value: &[u8]) -> OpaqueCredential {
    OpaqueCredential::new(value.into()).unwrap()
}
fn setup() -> (AccountManager, Arc<Records>, Arc<Secrets>) {
    let records = Arc::new(Records::default());
    let secrets = Arc::new(Secrets::default());
    (
        AccountManager::new(records.clone(), secrets.clone()).unwrap(),
        records,
        secrets,
    )
}

#[test]
fn invalid_saved_json_falls_back_to_empty_runtime_state() {
    let records = Arc::new(Records::default());
    *records.json.lock().unwrap() = Some("{invalid".into());
    let manager = AccountManager::new(records, Arc::new(Secrets::default())).unwrap();
    assert!(manager.records(&source("test")).unwrap().is_empty());
    assert!(manager.session(&source("test")).account.is_none());
}
#[test]
fn failed_login_does_not_publish_account_and_can_retry_after_restart() {
    for failure in 0..3 {
        let (manager, records, secrets) = setup();
        let expected = manager.session(&source("test"));
        secrets.fail_write.store(failure == 0, Ordering::SeqCst);
        secrets.fail_read.store(failure == 1, Ordering::SeqCst);
        records.fail.store(failure == 2, Ordering::SeqCst);
        assert!(manager
            .accept(&expected, record("test", "123"), secret(b"credential"))
            .is_err());
        assert_eq!(manager.session(&source("test")), expected);
        assert!(manager.records(&source("test")).unwrap().is_empty());
        secrets.fail_write.store(false, Ordering::SeqCst);
        secrets.fail_read.store(false, Ordering::SeqCst);
        records.fail.store(false, Ordering::SeqCst);
        let restarted = AccountManager::new(records, secrets).unwrap();
        restarted
            .accept(
                &restarted.session(&source("test")),
                record("test", "123"),
                secret(b"credential"),
            )
            .unwrap();
        assert_eq!(restarted.records(&source("test")).unwrap().len(), 1);
    }
}
#[test]
fn late_login_and_cross_source_completion_cannot_replace_selected_account() {
    let (manager, _, _) = setup();
    let expected = manager.session(&source("test"));
    manager
        .accept(&expected, record("test", "A:id"), secret(b"a"))
        .unwrap();
    assert_eq!(
        manager
            .accept(&expected, record("test", "B:id"), secret(b"b"))
            .unwrap_err()
            .code,
        ErrorCode::StaleContext
    );
    let expected = manager.session(&source("test"));
    assert_eq!(
        manager
            .accept(&expected, record("other", "id"), secret(b"b"))
            .unwrap_err()
            .code,
        ErrorCode::InvalidData
    );
    manager
        .accept(&expected, record("test", "B:id"), secret(b"b"))
        .unwrap();
    manager.select(&record("test", "A:id").reference).unwrap();
    assert_eq!(
        manager
            .provider_credential(&source("test"))
            .unwrap()
            .unwrap()
            .expose(),
        b"a"
    );
    assert_eq!(manager.records(&source("test")).unwrap().len(), 2);
}
#[test]
fn deletion_tombstone_survives_cleanup_failure_and_restart() {
    let (manager, records, secrets) = setup();
    manager
        .accept(
            &manager.session(&source("test")),
            record("test", "a"),
            secret(b"secret"),
        )
        .unwrap();
    secrets.fail_delete.store(true, Ordering::SeqCst);
    assert!(manager.remove(&record("test", "a").reference).is_err());
    assert!(manager.session(&source("test")).account.is_none());
    assert!(manager
        .provider_credential(&source("test"))
        .unwrap()
        .is_none());
    let restarted = AccountManager::new(records.clone(), secrets.clone()).unwrap();
    assert!(restarted.records(&source("test")).unwrap().is_empty());
    assert!(restarted.session(&source("test")).account.is_none());
    secrets.fail_delete.store(false, Ordering::SeqCst);
    AccountManager::new(records, secrets.clone()).unwrap();
    assert!(secrets.values.lock().unwrap().is_empty());
}
struct Adapter(super::super::adapter::SourceDescriptor);
impl super::super::adapter::MusicAdapter for Adapter {
    fn descriptor(&self) -> &super::super::adapter::SourceDescriptor {
        &self.0
    }
    fn read_track<'a>(
        &'a self,
        _: &'a EntityRef,
        _: &'a RequestContext,
    ) -> super::super::adapter::AdapterFuture<'a, MusicTrack> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
    fn resolve_playback<'a>(
        &'a self,
        _: &'a super::super::resource::ResolveRequest,
        _: &'a RequestContext,
    ) -> super::super::adapter::AdapterFuture<'a, super::super::resource::PlaybackResource> {
        Box::pin(async { Err(ErrorCode::Unsupported.into()) })
    }
}
#[test]
fn scoped_access_rejects_cross_source_stale_accounts_and_disabled_instances() {
    let (manager, _, _) = setup();
    let manager = Arc::new(manager);
    let adapters = Arc::new(super::super::manager::AdapterManager::default());
    manager.bind(&adapters);
    for name in ["test", "other"] {
        let accounts = manager.clone();
        let source_id = source(name);
        let session_source = source_id.clone();
        adapters
            .register(
                Arc::new(Adapter(super::super::adapter::SourceDescriptor {
                    source: source_id,
                    display_name: name.into(),
                    description: None,
                    platforms: Default::default(),
                    contract_version: super::super::CONTRACT_VERSION,
                    login_methods: Vec::new(),
                    cover_hosts: Default::default(),
                    share_link_templates: Default::default(),
                    capabilities: Default::default(),
                })),
                Arc::new(move || accounts.session(&session_source)),
            )
            .unwrap();
        manager
            .accept(
                &manager.session(&source(name)),
                record(name, "A"),
                secret(name.as_bytes()),
            )
            .unwrap();
    }
    let context = |name| {
        let (session, generation) = adapters.stamp(&source(name)).unwrap();
        RequestContext {
            session,
            adapter_generation: generation,
            deadline: Instant::now() + Duration::from_secs(10),
            cancellation: Default::default(),
        }
    };
    let expected = context("test");
    let access = manager.scoped_access(&expected).unwrap();
    assert_eq!(access.read(&expected).unwrap().expose(), b"test");
    assert_eq!(
        access.read(&context("other")).unwrap_err().code,
        ErrorCode::PermissionDenied
    );
    access.replace(&expected, secret(b"refreshed")).unwrap();
    assert_eq!(
        access.read(&expected).unwrap_err().code,
        ErrorCode::StaleContext
    );
    let expected = context("test");
    let access = manager.scoped_access(&expected).unwrap();
    adapters.set_enabled(&source("test"), false).unwrap();
    assert_eq!(
        access.read(&expected).unwrap_err().code,
        ErrorCode::SourceUnavailable
    );
    assert_eq!(manager.records(&source("test")).unwrap().len(), 1);
    adapters.set_enabled(&source("test"), true).unwrap();
    assert_eq!(
        access.read(&expected).unwrap_err().code,
        ErrorCode::StaleContext
    );
}

#[test]
fn failed_relogin_metadata_commit_preserves_existing_credential() {
    let (manager, records, _) = setup();
    let expected = manager.session(&source("test"));
    manager
        .accept(&expected, record("test", "a"), secret(b"original"))
        .unwrap();
    let expected = manager.session(&source("test"));
    records.fail.store(true, Ordering::SeqCst);
    assert!(manager
        .accept(&expected, record("test", "a"), secret(b"replacement"))
        .is_err());
    assert_eq!(manager.session(&source("test")), expected);
    assert_eq!(
        manager
            .provider_credential(&source("test"))
            .unwrap()
            .unwrap()
            .expose(),
        b"original"
    );
}
#[tokio::test]
async fn logout_preserves_credentials_and_can_reselect_after_restart() {
    let (manager, records, secrets) = setup();
    let manager = Arc::new(manager);
    let adapters = Arc::new(super::super::manager::AdapterManager::default());
    manager.bind(&adapters);
    let provider = Adapter(super::super::adapter::SourceDescriptor {
        source: source("test"),
        display_name: "test".into(),
        description: None,
        platforms: Default::default(),
        contract_version: super::super::CONTRACT_VERSION,
        login_methods: Vec::new(),
        cover_hosts: Default::default(),
        share_link_templates: Default::default(),
        capabilities: [super::super::adapter::Capability::Account]
            .into_iter()
            .collect(),
    });
    let sessions = manager.clone();
    adapters
        .register(
            Arc::new(provider),
            Arc::new(move || sessions.session(&source("test"))),
        )
        .unwrap();
    manager
        .accept(
            &manager.session(&source("test")),
            record("test", "a"),
            secret(b"credential"),
        )
        .unwrap();
    let result = adapters
        .account(
            &source("test"),
            &super::super::business::AccountRequest::Logout,
            &manager,
        )
        .await
        .unwrap();
    let super::super::business::AccountPresentation::Logout(report) = result else {
        panic!("logout report");
    };
    assert!(report.local_cleared);
    assert!(report.local_error.is_none());
    assert!(report.remote_error.is_none());
    assert_eq!(
        manager.records(&source("test")).unwrap(),
        vec![record("test", "a")]
    );
    assert!(manager.current_record(&source("test")).unwrap().is_none());
    assert!(manager
        .provider_credential(&source("test"))
        .unwrap()
        .is_none());
    let restored = AccountManager::new(records, secrets).unwrap();
    assert!(restored.session(&source("test")).account.is_none());
    restored.select(&record("test", "a").reference).unwrap();
    assert_eq!(
        restored
            .provider_credential(&source("test"))
            .unwrap()
            .unwrap()
            .expose(),
        b"credential"
    );
}

#[test]
fn logout_is_scoped_atomic_and_rejects_stale_sessions() {
    let (manager, records, _) = setup();
    for provider in ["test", "other"] {
        manager
            .accept(
                &manager.session(&source(provider)),
                record(provider, "a"),
                secret(b"credential"),
            )
            .unwrap();
    }
    let before = manager.session(&source("test"));
    records.fail.store(true, Ordering::SeqCst);
    assert!(manager.clear_expected(&before).is_err());
    assert_eq!(manager.session(&source("test")), before);
    records.fail.store(false, Ordering::SeqCst);
    manager.clear_expected(&before).unwrap();
    assert!(manager.session(&source("test")).account.is_none());
    assert!(manager.session(&source("other")).account.is_some());
    manager.select(&record("test", "a").reference).unwrap();
    assert_eq!(
        manager.clear_expected(&before).unwrap_err().code,
        ErrorCode::StaleContext
    );
    assert!(manager.current_record(&source("test")).unwrap().is_some());
}

#[test]
fn refresh_after_instance_revocation_restores_previous_secret_without_publication() {
    let (manager, _, secrets) = setup();
    let manager = Arc::new(manager);
    let adapters = Arc::new(super::super::manager::AdapterManager::default());
    manager.bind(&adapters);
    let sessions = manager.clone();
    adapters
        .register(
            Arc::new(Adapter(super::super::adapter::SourceDescriptor {
                source: source("test"),
                display_name: "test".into(),
                description: None,
                platforms: Default::default(),
                contract_version: super::super::CONTRACT_VERSION,
                login_methods: Vec::new(),
                cover_hosts: Default::default(),
                share_link_templates: Default::default(),
                capabilities: Default::default(),
            })),
            Arc::new(move || sessions.session(&source("test"))),
        )
        .unwrap();
    manager
        .accept(
            &manager.session(&source("test")),
            record("test", "a"),
            secret(b"original"),
        )
        .unwrap();
    let (session, adapter_generation) = adapters.stamp(&source("test")).unwrap();
    let context = RequestContext {
        session: session.clone(),
        adapter_generation,
        deadline: Instant::now() + Duration::from_secs(10),
        cancellation: Default::default(),
    };
    let access = manager.scoped_access(&context).unwrap();
    let runtime = adapters.clone();
    *secrets.after_write.lock().unwrap() = Some(Box::new(move || {
        runtime.set_enabled(&source("test"), false).unwrap();
    }));
    assert_eq!(
        access.replace(&context, secret(b"late")).unwrap_err().code,
        ErrorCode::SourceUnavailable
    );
    assert_eq!(manager.session(&source("test")), session);
    assert_eq!(
        secrets
            .read(&key(&record("test", "a").reference))
            .unwrap()
            .unwrap(),
        b"original"
    );
}

#[test]
fn repeated_cleanup_failure_bounds_tombstones_and_keeps_remaining_account_recoverable() {
    let (manager, records, secrets) = setup();
    secrets.fail_delete.store(true, Ordering::SeqCst);
    for index in 0..101 {
        let record = record("test", &index.to_string());
        manager
            .accept(
                &manager.session(&source("test")),
                record.clone(),
                secret(b"credential"),
            )
            .unwrap();
        let error = manager.remove(&record.reference).unwrap_err();
        assert_eq!(
            error.code,
            if index < 100 {
                ErrorCode::Internal
            } else {
                ErrorCode::RateLimited
            }
        );
    }
    let restarted = AccountManager::new(records, secrets).unwrap();
    assert_eq!(restarted.records(&source("test")).unwrap().len(), 1);
    assert_eq!(
        restarted.session(&source("test")).account.unwrap().as_str(),
        "100"
    );
}

#[test]
fn invalidation_is_scoped_preserves_saved_accounts_and_reports_save_failure() {
    let (manager, records, secrets) = setup();
    let source = source("qqmusic");
    manager
        .accept(
            &manager.session(&source),
            record("qqmusic", "opaque-user"),
            secret(b"credential"),
        )
        .unwrap();
    let session = manager.session(&source);
    records.fail.store(true, Ordering::SeqCst);
    assert!(manager.invalidate(&session).is_err());
    assert_eq!(manager.session(&source), session);
    records.fail.store(false, Ordering::SeqCst);
    manager.invalidate(&session).unwrap();
    assert!(manager.session(&source).account.is_none());
    assert_eq!(manager.records(&source).unwrap().len(), 1);
    assert_eq!(
        manager.invalidate(&session).unwrap_err().code,
        ErrorCode::StaleContext
    );
    let restart = AccountManager::new(records, secrets).unwrap();
    assert!(restart.session(&source).account.is_none());
    assert_eq!(restart.records(&source).unwrap().len(), 1);
}
