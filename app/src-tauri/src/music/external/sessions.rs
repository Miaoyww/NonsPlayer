//! Bounded persistent Components for login only; ordinary reads keep fresh Stores.
use super::*;
use crate::music::business::{AccountOutcome, AccountPresentation, AccountRequest};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct Sessions {
    slots: Mutex<HashMap<String, Arc<Slot>>>,
}
struct Slot {
    runtime: tokio::sync::Mutex<Runtime>,
    host: Arc<host::Context>,
    template: RequestContext,
    expires: Instant,
    closed: AtomicBool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}
impl Slot {
    fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.host.active.store(false, Ordering::Release);
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        self.close();
    }
}
impl Sessions {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.slots.lock().unwrap().is_empty()
    }
    pub fn cancel(&self, key: &str) {
        if let Ok(mut slots) = (self.slots.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("adapter.login_cancel_lock", error);
        }) {
            if let Some(slot) = slots.remove(key) {
                slot.close();
            }
        }
    }
    pub fn revoke(&self) {
        if let Ok(mut slots) = (self.slots.lock()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("adapter.login_revoke_lock", error);
        }) {
            for slot in slots.values() {
                slot.close();
            }
            slots.clear();
        }
    }
    fn get(&self, key: &str) -> MusicResult<Arc<Slot>> {
        self.slots
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("adapter.login_lookup_lock", error);
            })
            .map_err(|_| ErrorCode::Internal)?
            .get(key)
            .cloned()
            .ok_or_else(|| ErrorCode::StaleContext.into())
    }
    fn insert(self: &Arc<Self>, key: String, slot: Slot) -> MusicResult<()> {
        let slot = Arc::new(slot);
        {
            let mut slots = self.slots.lock().map_err(|error| {
                log::error!(target: "nons::adapter", "runtime_failure reason={error}");
                ErrorCode::Internal
            })?;
            if slots.len() >= 4 || slots.contains_key(&key) {
                return Err(ErrorCode::RateLimited.into());
            }
            slots.insert(key.clone(), slot.clone());
        }
        let weak = Arc::downgrade(&slot);
        let registry = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut tick = Instant::now();
            loop {
                let Some(slot) = weak.upgrade() else {
                    break;
                };
                if slot.closed.load(Ordering::Acquire) || Instant::now() >= slot.expires {
                    break;
                }
                let mut context = slot.template.clone();
                context.deadline = (Instant::now() + Duration::from_secs(15)).min(slot.expires);
                if let Err(error) = slot.host.renew(context).and_then(|_| slot.host.check()) {
                    log::info!(target: "nons::adapter", "operation=login.renew reason={error}");
                    break;
                }
                let event = match slot.host.next_event(100).await {
                    Ok(event) => event,
                    Err(error) => {
                        crate::infrastructure::diagnostics::warn("login.next_event", error);
                        break;
                    }
                };
                if event.is_some() || tick.elapsed() >= Duration::from_secs(1) {
                    use base64::Engine as _;
                    let value = match event {
                        Some(runtime::host_io::Event::Binary(message)) => {
                            serde_json::json!({"kind":"binary","socket":message.socket,"data":base64::engine::general_purpose::STANDARD.encode(message.data)})
                        }
                        Some(runtime::host_io::Event::Closed(socket)) => {
                            serde_json::json!({"kind":"closed","socket":socket})
                        }
                        None => serde_json::json!({"kind":"tick"}),
                    };
                    tick = Instant::now();
                    let mut runtime = slot.runtime.lock().await;
                    let arguments = value.to_string();
                    tokio::select! {
                        result = runtime.call("runtime:event", &arguments) => {
                            match result.inspect_err(|error| { crate::infrastructure::diagnostics::warn("adapter.login_event_call", error); }).ok().and_then(|reply| serde_json::from_str::<Reply<serde_json::Value>>(&reply).inspect_err(|error| { crate::infrastructure::diagnostics::warn("adapter.login_event_reply_decode", error); }).ok()) {
                                Some(Reply::Ok(_)) => {},
                                Some(Reply::Error(error)) => {
                                    crate::infrastructure::diagnostics::warn("login.runtime_event", error);
                                    break;
                                },
                                _ => break,
                            }
                        },
                        _ = async { while !slot.closed.load(Ordering::Acquire) { tokio::time::sleep(Duration::from_millis(20)).await; } } => break,
                    }
                }
            }
            if let Some(slot) = weak.upgrade() {
                slot.host.release().await;
            }
            if let Some(registry) = registry.upgrade() {
                registry.cancel(&key);
            }
        });
        Ok(())
    }
}
impl ExternalAdapter {
    pub(super) async fn account_request(
        &self,
        request: &AccountRequest,
        context: &RequestContext,
    ) -> MusicResult<AccountOutcome> {
        let services = self.services.clone().ok_or(ErrorCode::Unsupported)?;
        match request {
            AccountRequest::BeginLogin { .. } => {
                let expires = Instant::now() + Duration::from_secs(180);
                let permit = self
                    .reads
                    .clone()
                    .try_acquire_owned()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("adapter.login_admission", error);
                    })
                    .map_err(|_| ErrorCode::RateLimited)?;
                let host = host::Context::new(services, context.clone(), true);
                let mut runtime = Runtime::from_component_with_budget(
                    self.engine.clone(),
                    &self.component,
                    host.clone(),
                    runtime::Budget::ADAPTER,
                )
                .await
                .map_err(|error| {
                    log::error!(target: "nons::adapter", "runtime_failure reason={error}");
                    ErrorCode::Internal
                })?;
                // Each generic phase gets one invocation budget. The UI receives no
                // challenge until creation finishes; both phases share the same deadline.
                let preparation = runtime
                    .call("music.prepare-login", &Self::arguments(request, context)?)
                    .await
                    .map_err(|error| {
                        log::error!(target: "nons::adapter", "login_preparation_failure reason={error}");
                        ErrorCode::Internal
                    })?;
                match serde_json::from_str::<Reply<()>>(&preparation)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "adapter.login_prepare_reply_decode",
                            error,
                        );
                    })
                    .map_err(|_| ErrorCode::InvalidData)?
                {
                    Reply::Ok(()) => {}
                    Reply::Error(error) => return Err(error),
                }
                let reply = runtime
                    .call("music.account", &Self::arguments(request, context)?)
                    .await
                    .map_err(|error| {
                        log::error!(target: "nons::adapter", "runtime_failure reason={error}");
                        ErrorCode::Internal
                    })?;
                let presentation = match serde_json::from_str::<Reply<AccountPresentation>>(&reply)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "adapter.login_start_reply_decode",
                            error,
                        );
                    })
                    .map_err(|_| ErrorCode::InvalidData)?
                {
                    Reply::Ok(value) => value,
                    Reply::Error(error) => return Err(error),
                };
                let AccountPresentation::Challenge { key, .. } = &presentation else {
                    return Err(ErrorCode::InvalidData.into());
                };
                if key.is_empty() || key.len() > 256 {
                    return Err(ErrorCode::InvalidData.into());
                }
                self.sessions.insert(
                    key.clone(),
                    Slot {
                        runtime: tokio::sync::Mutex::new(runtime),
                        host,
                        template: context.clone(),
                        expires,
                        closed: AtomicBool::new(false),
                        _permit: permit,
                    },
                )?;
                Ok(AccountOutcome {
                    presentation,
                    credential: None,
                })
            }
            AccountRequest::PollLogin { key } => {
                let slot = self.sessions.get(key)?;
                if slot.closed.load(Ordering::Acquire) || Instant::now() >= slot.expires {
                    self.sessions.cancel(key);
                    return Err(ErrorCode::StaleContext.into());
                }
                slot.host
                    .renew(context.clone())
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "adapter.login_context_renew",
                            error,
                        );
                    })
                    .map_err(|_| ErrorCode::StaleContext)?;
                let mut runtime = slot.runtime.lock().await;
                let reply = runtime
                    .call("music.account", &Self::arguments(request, context)?)
                    .await
                    .map_err(|error| {
                        log::error!(target: "nons::adapter", "runtime_failure reason={error}");
                        ErrorCode::Internal
                    })?;
                let presentation = match serde_json::from_str::<Reply<AccountPresentation>>(&reply)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "adapter.login_poll_reply_decode",
                            error,
                        );
                    })
                    .map_err(|_| ErrorCode::InvalidData)?
                {
                    Reply::Ok(value) => value,
                    Reply::Error(error) => return Err(error),
                };
                let credential = if matches!(
                    presentation,
                    AccountPresentation::Progress { code: 803, .. }
                ) {
                    let pending = slot.host.pending.lock().map_err(|error| {
                        log::error!(target: "nons::adapter", "runtime_failure reason={error}");
                        ErrorCode::Internal
                    })?;
                    let (record, credential) = pending.as_ref().ok_or(ErrorCode::InvalidData)?;
                    // Retain the challenge copy until AccountManager confirms persistence.
                    Some((
                        record.clone(),
                        crate::music::account::OpaqueCredential::new(credential.expose().to_vec())?,
                    ))
                } else {
                    None
                };
                if matches!(
                    presentation,
                    AccountPresentation::Progress { code: 800, .. }
                ) {
                    self.sessions.cancel(key);
                }
                Ok(AccountOutcome {
                    presentation,
                    credential,
                })
            }
            AccountRequest::Profile => {
                let presentation = self
                    .invoke(
                        "music.account",
                        &Self::arguments(request, context)?,
                        false,
                        Some(context),
                    )
                    .await?;
                Ok(AccountOutcome {
                    presentation,
                    credential: None,
                })
            }
            AccountRequest::Logout => Err(ErrorCode::Unsupported.into()),
        }
    }
}
