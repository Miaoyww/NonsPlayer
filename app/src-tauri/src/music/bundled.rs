//! Bundled native package composition. Platform protocols are owned by adapters/netease.
use super::{
    account::*, accounts::AccountManager, adapter::*, identity::*, manager::AdapterManager,
    service::MusicService, ErrorCode, MusicResult,
};
use std::sync::{Arc, Weak};

struct HostAccounts {
    accounts: Arc<AccountManager>,
    adapters: Weak<AdapterManager>,
    source: SourceId,
}
impl ProviderAccounts for HostAccounts {
    fn session(&self) -> SessionContext {
        self.accounts.session(&self.source)
    }
    fn credential(&self) -> MusicResult<Option<OpaqueCredential>> {
        let adapters = self
            .adapters
            .upgrade()
            .ok_or(ErrorCode::SourceUnavailable)?;
        let expected = adapters.stamp(&self.source)?;
        let result = self.accounts.provider_credential(&self.source)?;
        if adapters.stamp(&self.source)? != expected {
            return Err(ErrorCode::StaleContext.into());
        }
        Ok(result)
    }
    fn read(&self, context: &RequestContext) -> MusicResult<Option<OpaqueCredential>> {
        if context.session.source != self.source {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let adapters = self
            .adapters
            .upgrade()
            .ok_or(ErrorCode::SourceUnavailable)?;
        let (session, generation) = adapters.stamp(&self.source)?;
        context.check(&session, generation)?;
        let result = if context.session.account.is_some() {
            Some(self.accounts.scoped_access(context)?.read(context)?)
        } else {
            self.accounts.provider_credential(&self.source)?
        };
        let (session, generation) = adapters.stamp(&self.source)?;
        context.check(&session, generation)?;
        Ok(result)
    }
    fn invalidate<'a>(&'a self, expected: SessionContext) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            if expected.source != self.source {
                return Err(ErrorCode::PermissionDenied.into());
            }
            self.accounts
                .blocking(move |accounts| {
                    accounts.invalidate(&expected);
                    Ok(())
                })
                .await
        })
    }
}

pub(crate) fn service(
    accounts: Arc<AccountManager>,
) -> crate::model::AppResult<(MusicService, Arc<nons_adapter_netease::platform::Netease>)> {
    let adapters = Arc::new(AdapterManager::default());
    accounts.bind(&adapters);
    let bridge = Arc::new(HostAccounts {
        accounts: accounts.clone(),
        adapters: Arc::downgrade(&adapters),
        source: SourceId::try_from("netease".to_owned()).unwrap(),
    });
    let client = Arc::new(nons_adapter_netease::platform::Netease::with_accounts(
        bridge,
    ));
    let session_client = client.clone();
    adapters
        .register(
            Arc::new(nons_adapter_netease::NeteaseAdapter::new(client.clone())),
            Arc::new(move || nons_adapter_netease::session(&session_client)),
        )
        .map_err(|e| e.to_string())?;
    let mut music = MusicService::new(adapters);
    music.accounts = Some(accounts);
    Ok((music, client))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::accounts::CredentialStore;
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct Secrets(std::sync::Mutex<BTreeMap<String, Vec<u8>>>);
    impl CredentialStore for Secrets {
        fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()> {
            self.0.lock().unwrap().insert(key.into(), bytes.into());
            Ok(())
        }
        fn delete(&self, key: &str) -> MusicResult<()> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }
    #[test]
    fn native_package_bridge_revokes_credentials_and_preserves_accounts() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            crate::storage::Store::open(&directory.path().join("accounts.sqlite3")).unwrap(),
        );
        let accounts = Arc::new(AccountManager::new(store, Arc::new(Secrets::default())).unwrap());
        let source = SourceId::try_from("netease".to_owned()).unwrap();
        let record = AccountRecord {
            reference: AccountRef {
                source: source.clone(),
                id: OpaqueId::try_from("42".to_owned()).unwrap(),
            },
            display_name: "Listener".into(),
            avatar: Default::default(),
        };
        accounts
            .accept(
                &accounts.session(&source),
                record.clone(),
                OpaqueCredential::new(b"test-cookie".to_vec()).unwrap(),
                false,
            )
            .unwrap();
        let (service, client) = service(accounts.clone()).unwrap();
        let bridge = HostAccounts {
            accounts: accounts.clone(),
            adapters: Arc::downgrade(&service.adapters),
            source: source.clone(),
        };
        let (session, generation) = service.adapters.stamp(&source).unwrap();
        let mut context = RequestContext {
            session,
            adapter_generation: generation,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(10),
            cancellation: Cancellation::default(),
        };
        assert_eq!(
            bridge.read(&context).unwrap().unwrap().expose(),
            b"test-cookie"
        );
        context.session.source = SourceId::try_from("other".to_owned()).unwrap();
        assert_eq!(
            bridge.read(&context).err().unwrap().code,
            ErrorCode::PermissionDenied
        );
        context.session.source = source.clone();
        service.adapters.reload(&source).unwrap();
        assert_eq!(
            bridge.read(&context).err().unwrap().code,
            ErrorCode::StaleContext
        );
        assert_eq!(
            client.account_credentials().unwrap().as_deref(),
            Some("test-cookie")
        );
        service.adapters.set_enabled(&source, false).unwrap();
        assert!(client.account_credentials().is_err());
        assert_eq!(
            bridge.read(&context).err().unwrap().code,
            ErrorCode::SourceUnavailable
        );
        assert_eq!(accounts.records(&source).unwrap(), vec![record]);
        service.adapters.set_enabled(&source, true).unwrap();
        assert_eq!(
            client.account_credentials().unwrap().as_deref(),
            Some("test-cookie")
        );
    }
}
