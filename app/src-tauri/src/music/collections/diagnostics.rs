//! Playlist query diagnostics; cursor contents, account scopes and payloads stay private.
use super::*;
use std::time::Instant;

static NEXT: AtomicU64 = AtomicU64::new(1);

pub(super) struct Query<'a> {
    id: u64,
    entity: &'a identity::EntityRef,
    route: &'static str,
    started: Instant,
    limit: u16,
    cursor: bool,
    refresh: bool,
    current: Mutex<(&'static str, &'static str, String)>,
    finished: AtomicBool,
}

impl<'a> Query<'a> {
    pub(super) fn new(
        entity: &'a identity::EntityRef,
        page: &PageRequest,
        refresh: bool,
        route: &'static str,
    ) -> Self {
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            entity,
            route,
            started: Instant::now(),
            limit: page.limit,
            cursor: page.cursor.is_some(),
            refresh,
            current: Mutex::new(("request_validate", "page_request_invalid", String::new())),
            finished: AtomicBool::new(false),
        }
    }

    pub(super) fn stage(&self, stage: &'static str, reason: &'static str) {
        if let Ok(mut current) = self.current.lock() {
            current.0 = stage;
            current.1 = reason;
            current.2.clear();
        }
    }

    pub(super) fn details(&self, details: std::fmt::Arguments<'_>) {
        if let Ok(mut current) = self.current.lock() {
            current.2 = details.to_string();
        }
    }

    fn log(&self, code: &str, success: bool, level: log::Level) {
        let current = self
            .current
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        log::log!(target: "nons::music", level,
            "operation=tracks route={} request=collection-{} source={} entity_kind={:?} entity_id={} limit={} cursor_present={} refresh={} elapsed_ms={} stage={} code={} reason={} {}",
            self.route, self.id, crate::infrastructure::diagnostics::field(self.entity.source.as_str()), self.entity.kind,
            crate::infrastructure::diagnostics::field(self.entity.id.as_str()), self.limit, self.cursor, self.refresh,
            self.started.elapsed().as_millis(), if success { "complete" } else { current.0 }, code,
            if success { "completed" } else { current.1 }, current.2);
    }

    pub(super) fn finish<T>(&self, result: &MusicResult<T>) {
        let (code, level) = match result {
            Ok(_) => ("ok".to_owned(), log::Level::Info),
            Err(error) => (
                format!("{:?}", error.code),
                if matches!(error.code, ErrorCode::StaleContext | ErrorCode::Cancelled) {
                    log::Level::Info
                } else {
                    log::Level::Error
                },
            ),
        };
        self.log(&code, result.is_ok(), level);
        self.finished.store(true, Ordering::Relaxed);
    }
}

impl Drop for Query<'_> {
    fn drop(&mut self) {
        if !self.finished.load(Ordering::Relaxed) {
            self.log("Cancelled", false, log::Level::Info);
        }
    }
}
