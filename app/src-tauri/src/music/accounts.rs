//! Host-owned account records and opaque system credentials. No platform protocol lives here.
use super::{account::*, adapter::RequestContext, identity::*, ErrorCode, MusicResult};
use crate::infrastructure::credentials as keyring;
pub trait AccountRecordStore: Send + Sync {
    fn load(&self) -> MusicResult<Option<String>>;
    fn save(&self, json: &str) -> MusicResult<()>;
}
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock, Weak};

pub trait CredentialStore: Send + Sync {
    fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>>;
    fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()>;
    fn delete(&self, key: &str) -> MusicResult<()>;
}
pub struct SystemCredentials;
impl CredentialStore for SystemCredentials {
    fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
        match keyring::Entry::new("NonsPlayer", key)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("credentials.read_entry", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .get_secret()
        {
            Ok(bytes) => Ok(Some(bytes)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(ErrorCode::Internal.into()),
        }
    }
    fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()> {
        keyring::Entry::new("NonsPlayer", key)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("credentials.write_entry", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .set_secret(bytes)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("credentials.write_secret", error);
            })
            .map_err(|_| ErrorCode::Internal.into())
    }
    fn delete(&self, key: &str) -> MusicResult<()> {
        match keyring::Entry::new("NonsPlayer", key)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("credentials.delete_entry", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .delete_credential()
        {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(ErrorCode::Internal.into()),
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedAccounts {
    records: Vec<AccountRecord>,
    selected: Vec<AccountRef>,
    pending_deletions: Vec<AccountRef>,
}
struct State {
    saved: SavedAccounts,
    generations: std::collections::BTreeMap<SourceId, u64>,
}
type AccountChanged = Arc<dyn Fn(&SourceId) + Send + Sync>;
pub struct AccountManager {
    state: Mutex<State>,
    changed: Mutex<Option<AccountChanged>>,
    published: RwLock<std::collections::BTreeMap<SourceId, SessionContext>>,
    store: Arc<dyn AccountRecordStore>,
    credentials: Arc<dyn CredentialStore>,
    io: Arc<tokio::sync::Semaphore>,
    adapters: Mutex<Weak<super::manager::AdapterManager>>,
}
fn key(reference: &AccountRef) -> String {
    use sha2::{Digest, Sha256};
    let identity = serde_json::to_vec(reference).expect("account serializes");
    format!("music-account-v1-{:x}", Sha256::digest(identity))
}
impl AccountManager {
    pub fn new(
        store: Arc<dyn AccountRecordStore>,
        credentials: Arc<dyn CredentialStore>,
    ) -> MusicResult<Self> {
        let mut saved: SavedAccounts = match store.load()? {
            None => SavedAccounts::default(),
            Some(value) if value.len() > 2 * 1024 * 1024 => {
                crate::infrastructure::diagnostics::warn(
                    "accounts.restore_size",
                    format!("bytes={}", value.len()),
                );
                SavedAccounts::default()
            }
            Some(value) => serde_json::from_str(&value)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("accounts.restore_decode", error);
                })
                .unwrap_or_default(),
        };
        if saved.records.len() > 100
            || saved.selected.len() > 100
            || saved.pending_deletions.len() > 100
            || saved
                .records
                .iter()
                .any(|a| a.display_name.len() > 1024 || a.avatar.len() > 8192)
            || saved.records.iter().enumerate().any(|(i, a)| {
                saved.records[..i]
                    .iter()
                    .any(|b| a.reference == b.reference)
            })
            || saved
                .selected
                .iter()
                .enumerate()
                .any(|(i, a)| saved.selected[..i].iter().any(|b| a.source == b.source))
            || saved
                .pending_deletions
                .iter()
                .any(|r| saved.records.iter().any(|a| &a.reference == r))
            || saved
                .selected
                .iter()
                .any(|r| !saved.records.iter().any(|a| &a.reference == r))
        {
            crate::infrastructure::diagnostics::warn(
                "accounts.restore_shape",
                "saved account records are invalid",
            );
            saved = SavedAccounts::default();
        }
        let published = saved
            .selected
            .iter()
            .map(|r| {
                (
                    r.source.clone(),
                    SessionContext {
                        source: r.source.clone(),
                        account: Some(r.id.clone()),
                        generation: 0,
                    },
                )
            })
            .collect();
        let manager = Self {
            published: RwLock::new(published),
            changed: Default::default(),
            state: Mutex::new(State {
                saved,
                generations: Default::default(),
            }),
            store,
            credentials,
            io: Arc::new(tokio::sync::Semaphore::new(4)),
            adapters: Default::default(),
        };
        // Tombstones prevent crash recovery from restoring an account whose deletion failed.
        let _ = (manager.retry_cleanup()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("manager.retry_cleanup", error);
        });
        Ok(manager)
    }
    pub(crate) async fn blocking<T: Send + 'static>(
        self: &Arc<Self>,
        operation: impl FnOnce(&Self) -> MusicResult<T> + Send + 'static,
    ) -> MusicResult<T> {
        let permit = self
            .io
            .clone()
            .try_acquire_owned()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.io_admission", error);
            })
            .map_err(|_| ErrorCode::RateLimited)?;
        let manager = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation(&manager)
        })
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("accounts.io_task_join", error);
        })
        .map_err(|_| ErrorCode::Internal)?
    }
    pub(crate) fn on_change(&self, callback: AccountChanged) {
        *self.changed.lock().expect("account notification lock") = Some(callback);
    }
    pub fn bind(&self, adapters: &Arc<super::manager::AdapterManager>) {
        *self.adapters.lock().expect("account adapter lock") = Arc::downgrade(adapters);
    }
    fn persist(&self, saved: &SavedAccounts) -> MusicResult<()> {
        self.store.save(
            &serde_json::to_string(saved)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("accounts.persist_encode", error);
                })
                .map_err(|_| ErrorCode::Internal)?,
        )
    }
    fn session_locked(state: &State, source: &SourceId) -> SessionContext {
        SessionContext {
            source: source.clone(),
            account: state
                .saved
                .selected
                .iter()
                .find(|r| &r.source == source)
                .map(|r| r.id.clone()),
            generation: *state.generations.get(source).unwrap_or(&0),
        }
    }
    pub fn session(&self, source: &SourceId) -> SessionContext {
        // Publication lock is never held across storage IO; playback does not wait for keyring.
        self.published
            .read()
            .expect("session publication lock")
            .get(source)
            .cloned()
            .unwrap_or(SessionContext {
                source: source.clone(),
                account: None,
                generation: 0,
            })
    }
    fn advance(&self, state: &mut State, source: &SourceId) {
        *state.generations.entry(source.clone()).or_default() += 1;
        self.published
            .write()
            .expect("session publication lock")
            .insert(source.clone(), Self::session_locked(state, source));
        if let Some(callback) = self
            .changed
            .lock()
            .expect("account notification lock")
            .as_ref()
        {
            callback(source);
        }
    }
    pub fn records(&self, source: &SourceId) -> MusicResult<Vec<AccountRecord>> {
        Ok(self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.records_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .saved
            .records
            .iter()
            .filter(|a| &a.reference.source == source)
            .cloned()
            .collect())
    }
    pub fn current_record(&self, source: &SourceId) -> MusicResult<Option<AccountRecord>> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.current_record_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        Ok(state
            .saved
            .records
            .iter()
            .find(|record| {
                &record.reference.source == source
                    && state.saved.selected.contains(&record.reference)
            })
            .cloned())
    }
    pub fn select(&self, reference: &AccountRef) -> MusicResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.select_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if !state
            .saved
            .records
            .iter()
            .any(|a| &a.reference == reference)
        {
            return Err(ErrorCode::NotFound.into());
        }
        if self.credentials.read(&key(reference))?.is_none() {
            return Err(ErrorCode::Unauthenticated.into());
        }
        let mut saved = state.saved.clone();
        saved.selected.retain(|r| r.source != reference.source);
        saved.selected.push(reference.clone());
        self.persist(&saved)?;
        state.saved = saved;
        self.advance(&mut state, &reference.source);
        Ok(())
    }
    /// The adapter confirms identity; CAS prevents a late login replacing a new session.
    pub fn accept(
        &self,
        expected: &SessionContext,
        record: AccountRecord,
        credential: OpaqueCredential,
    ) -> MusicResult<()> {
        self.accept_guarded(expected, record, credential, None)
    }
    pub(crate) fn accept_context(
        &self,
        context: &RequestContext,
        record: AccountRecord,
        credential: OpaqueCredential,
    ) -> MusicResult<()> {
        self.accept_guarded(&context.session, record, credential, Some(context))
    }
    fn check_context(&self, context: &RequestContext) -> MusicResult<()> {
        let adapters = self
            .adapters
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.context_adapter_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .upgrade()
            .ok_or(ErrorCode::SourceUnavailable)?;
        let (session, generation) = adapters.stamp(&context.session.source)?;
        context.check(&session, generation)
    }
    fn accept_guarded(
        &self,
        expected: &SessionContext,
        record: AccountRecord,
        credential: OpaqueCredential,
        context: Option<&RequestContext>,
    ) -> MusicResult<()> {
        if let Some(context) = context {
            self.check_context(context)?;
        }
        if record.reference.source != expected.source
            || record.display_name.len() > 1024
            || record.avatar.len() > 8192
        {
            return Err(ErrorCode::InvalidData.into());
        }
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.accept_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if Self::session_locked(&state, &expected.source) != *expected {
            return Err(ErrorCode::StaleContext.into());
        }
        if state.saved.pending_deletions.contains(&record.reference) {
            return Err(ErrorCode::Internal.into());
        }
        let mut saved = state.saved.clone();
        saved.records.retain(|a| a.reference != record.reference);
        if saved.records.len() >= 100 {
            return Err(ErrorCode::RateLimited.into());
        }
        let target = key(&record.reference);
        let previous = self.credentials.read(&target)?;
        self.credentials.write(&target, credential.expose())?;
        let verified = self.credentials.read(&target)?.ok_or(ErrorCode::Internal)?;
        if verified != credential.expose() {
            return Err(ErrorCode::Internal.into());
        }
        OpaqueCredential::new(verified)?;
        saved.selected.retain(|r| r.source != expected.source);
        saved.selected.push(record.reference.clone());
        saved.records.push(record);
        if let Some(context) = context {
            if let Err(error) = self.check_context(context) {
                if let Some(previous) = &previous {
                    self.credentials.write(&target, previous)?;
                }
                return Err(error);
            }
        }
        if let Err(error) = self.persist(&saved) {
            // A failed metadata commit must not replace an already selected account's secret.
            if let Some(previous) = previous {
                self.credentials.write(&target, &previous)?;
            }
            return Err(error);
        }
        state.saved = saved;
        self.advance(&mut state, &expected.source);
        Ok(())
    }
    pub fn invalidate(&self, expected: &SessionContext) -> MusicResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.invalidate_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if Self::session_locked(&state, &expected.source) != *expected {
            return Err(ErrorCode::StaleContext.into());
        }
        let mut saved = state.saved.clone();
        saved.selected.retain(|r| r.source != expected.source);
        self.persist(&saved)?;
        state.saved = saved;
        self.advance(&mut state, &expected.source);
        Ok(())
    }
    /// Trusted provider bridge reads only the selected account.
    pub(crate) fn provider_credential(
        &self,
        source: &SourceId,
    ) -> MusicResult<Option<OpaqueCredential>> {
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "accounts.provider_credential_lock",
                    error,
                );
            })
            .map_err(|_| ErrorCode::Internal)?;
        let bytes =
            if let Some(reference) = state.saved.selected.iter().find(|r| &r.source == source) {
                self.credentials.read(&key(reference))?
            } else {
                None
            };
        bytes.map(OpaqueCredential::new).transpose()
    }
    pub fn remove(&self, reference: &AccountRef) -> MusicResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.remove_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if !state
            .saved
            .records
            .iter()
            .any(|a| &a.reference == reference)
            && !state.saved.pending_deletions.contains(reference)
        {
            return Err(ErrorCode::NotFound.into());
        }
        let mut saved = state.saved.clone();
        saved.records.retain(|a| &a.reference != reference);
        saved.selected.retain(|r| r != reference);
        if !saved.pending_deletions.contains(reference) {
            if saved.pending_deletions.len() >= 100 {
                return Err(ErrorCode::RateLimited.into());
            }
            saved.pending_deletions.push(reference.clone());
        }
        self.persist(&saved)?;
        state.saved = saved;
        self.advance(&mut state, &reference.source);
        drop(state);
        self.retry_cleanup()
    }
    pub(crate) fn clear_expected(&self, expected: &SessionContext) -> MusicResult<()> {
        let source = &expected.source;
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.clear_expected_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if Self::session_locked(&state, source) != *expected {
            return Err(ErrorCode::StaleContext.into());
        }
        let mut saved = state.saved.clone();
        saved.selected.retain(|r| &r.source != source);
        self.persist(&saved)?;
        state.saved = saved;
        self.advance(&mut state, source);
        Ok(())
    }
    fn retry_cleanup(&self) -> MusicResult<()> {
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.cleanup_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        let mut saved = state.saved.clone();
        let mut failed = false;
        saved.pending_deletions.retain(|r| {
            let error = self.credentials.delete(&key(r)).is_err();
            failed |= error;
            error
        });
        self.persist(&saved)?;
        state.saved = saved;
        if failed {
            Err(ErrorCode::Internal.into())
        } else {
            Ok(())
        }
    }
    fn authorize(&self, context: &RequestContext) -> MusicResult<AccountRef> {
        let adapters = self
            .adapters
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.authorize_adapter_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .upgrade()
            .ok_or(ErrorCode::SourceUnavailable)?;
        let (session, generation) = adapters.stamp(&context.session.source)?;
        context.check(&session, generation)?;
        Ok(AccountRef {
            source: session.source,
            id: session.account.ok_or(ErrorCode::Unauthenticated)?,
        })
    }
}
impl AccountManager {
    fn read_bound(&self, context: &RequestContext) -> MusicResult<OpaqueCredential> {
        let reference = self.authorize(context)?;
        let state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.read_bound_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if Self::session_locked(&state, &reference.source) != context.session {
            return Err(ErrorCode::StaleContext.into());
        }
        let credential = OpaqueCredential::new(
            self.credentials
                .read(&key(&reference))?
                .ok_or(ErrorCode::Unauthenticated)?,
        )?;
        drop(state);
        self.authorize(context)?;
        Ok(credential)
    }
    fn replace_bound(
        &self,
        context: &RequestContext,
        credential: OpaqueCredential,
    ) -> MusicResult<()> {
        let reference = self.authorize(context)?;
        let mut state = self
            .state
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("accounts.replace_bound_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?;
        if Self::session_locked(&state, &reference.source) != context.session {
            return Err(ErrorCode::StaleContext.into());
        }
        let target = key(&reference);
        let previous = self
            .credentials
            .read(&target)?
            .ok_or(ErrorCode::Unauthenticated)?;
        self.credentials.write(&target, credential.expose())?;
        let verified = self.credentials.read(&target);
        let valid = match verified {
            Ok(Some(bytes)) if bytes == credential.expose() => self.authorize(context).map(|_| ()),
            Ok(_) => Err(ErrorCode::Internal.into()),
            Err(error) => Err(error),
        };
        if let Err(error) = valid {
            self.credentials.write(&target, &previous)?;
            return Err(error);
        }
        self.advance(&mut state, &reference.source);
        Ok(())
    }
}

pub struct ScopedAccountAccess<'a> {
    manager: &'a AccountManager,
    binding: RequestContext,
}
impl AccountManager {
    pub(crate) fn scoped_access(
        &self,
        context: &RequestContext,
    ) -> MusicResult<ScopedAccountAccess<'_>> {
        self.authorize(context)?;
        Ok(ScopedAccountAccess {
            manager: self,
            binding: context.clone(),
        })
    }
}
impl ScopedAccountAccess<'_> {
    fn check(&self, context: &RequestContext) -> MusicResult<()> {
        if context.session != self.binding.session
            || context.adapter_generation != self.binding.adapter_generation
        {
            return Err(ErrorCode::PermissionDenied.into());
        }
        self.binding
            .check(&context.session, context.adapter_generation)?;
        Ok(())
    }
}
impl AccountAccess for ScopedAccountAccess<'_> {
    fn read(&self, context: &RequestContext) -> MusicResult<OpaqueCredential> {
        self.check(context)?;
        self.manager.read_bound(context)
    }
    fn replace(&self, context: &RequestContext, credential: OpaqueCredential) -> MusicResult<()> {
        self.check(context)?;
        self.manager.replace_bound(context, credential)
    }
}
#[cfg(test)]
mod tests;
