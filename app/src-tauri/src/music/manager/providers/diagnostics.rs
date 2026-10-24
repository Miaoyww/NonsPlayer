//! Bounded resource resolution diagnostics without transport or account data.
use super::*;
use std::sync::atomic::AtomicBool;

static NEXT: AtomicU64 = AtomicU64::new(1);
pub(super) fn next_request_id() -> u64 {
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub(super) struct Attempt<'a> {
    request: u64,
    attempt: usize,
    provider: &'a str,
    track: &'a EntityRef,
    started: Instant,
    provider_try: AtomicU64,
    stage: Mutex<(&'static str, &'static str)>,
    finished: AtomicBool,
}
impl<'a> Attempt<'a> {
    pub(super) fn new(
        request: u64,
        attempt: usize,
        provider: &'a str,
        track: &'a EntityRef,
    ) -> Self {
        Self {
            request,
            attempt,
            provider,
            track,
            started: Instant::now(),
            provider_try: AtomicU64::new(0),
            stage: Mutex::new(("context", "resolution_cancelled_or_context_invalid")),
            finished: AtomicBool::new(false),
        }
    }
    pub(super) fn start_try(&self) {
        self.provider_try.fetch_add(1, Ordering::Relaxed);
        self.stage("context", "provider_or_origin_context_invalid");
    }
    pub(super) fn stage(&self, stage: &'static str, reason: &'static str) {
        if let Ok(mut current) = self.stage.lock() {
            *current = (stage, reason);
        }
    }
    fn message(&self, code: &str, success: bool) -> String {
        let (stage, reason) = self
            .stage
            .lock()
            .map(|v| *v)
            .unwrap_or(("diagnostics", "stage_unavailable"));
        format!("operation=resource_resolve request={} attempt={} provider_try={} source={} provider={} stage={} elapsed_ms={} code={} reason={} track_kind={:?} track_id={}", self.request, self.attempt, self.provider_try.load(Ordering::Relaxed), crate::infrastructure::diagnostics::field(self.track.source.as_str()), crate::infrastructure::diagnostics::field(self.provider), if success { "complete" } else { stage }, self.started.elapsed().as_millis(), code, if success { "resolved" } else { reason }, self.track.kind, crate::infrastructure::diagnostics::field(self.track.id.as_str()))
    }
    pub(super) fn finish<T>(&self, result: &MusicResult<T>) {
        let (code, level) = match result {
            Ok(_) => ("ok".to_owned(), log::Level::Info),
            Err(error) => (
                format!("{:?}", error.code),
                if matches!(error.code, ErrorCode::Cancelled | ErrorCode::StaleContext) {
                    log::Level::Info
                } else {
                    log::Level::Warn
                },
            ),
        };
        log::log!(target: "nons::music", level, "{}", self.message(&code, result.is_ok()));
        self.finished.store(true, Ordering::Relaxed);
    }
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.finished.load(Ordering::Relaxed) {
            log::info!(target: "nons::music", "{}", self.message("Cancelled", false));
        }
    }
}

pub(super) fn request_reason(
    request: &ResourceRequest,
    descriptor: &ResourceProviderDescriptor,
) -> &'static str {
    if let Err(reason) = request.track.validate_with_reason(&request.track.reference) {
        return reason;
    }
    if !descriptor
        .platforms
        .contains(&request.track.reference.source)
    {
        return "track_platform_not_declared_by_provider";
    }
    "preferred_quality_must_contain_1_to_128_bytes"
}
pub(super) fn metadata_reason(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::InvalidData => "actual_quality_must_contain_1_to_128_bytes",
        ErrorCode::PermissionDenied => "preview_resource_is_not_full_playback",
        ErrorCode::DeadlineExceeded => "resource_expiry_is_not_in_the_future",
        _ => "resource_metadata_invalid",
    }
}
pub(super) fn access_reason(access: &ResourceAccess) -> &'static str {
    let ResourceAccess::Http(http) = access;
    if http.url.len() > 8192 {
        return "audio_url_exceeds_8192_bytes";
    }
    let Ok(url) = url::Url::parse(&http.url) else {
        return "audio_url_parse_failed";
    };
    if !matches!(url.scheme(), "https" | "http") {
        return "audio_url_scheme_not_http_or_https";
    }
    if url.host_str().is_none() {
        return "audio_url_host_missing";
    }
    if !url.username().is_empty() || url.password().is_some() {
        return "audio_url_contains_userinfo";
    }
    if url.fragment().is_some() {
        return "audio_url_contains_fragment";
    }
    if !http.headers.is_empty() {
        return "audio_transport_does_not_support_headers";
    }
    "resource_access_invalid"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_attempts_share_request_and_preserve_failure_stage_without_transport_data() {
        let reference = EntityRef {
            source: SourceId::try_from("catalog".to_owned()).unwrap(),
            kind: EntityKind::Track,
            id: OpaqueId::try_from("track/opaque".to_owned()).unwrap(),
        };
        let id = next_request_id();
        let failed = Attempt::new(id, 1, "first", &reference);
        failed.start_try();
        failed.stage("resource_access", "audio_url_contains_userinfo");
        let message = failed.message("InvalidData", false);
        for field in [
            format!("request={id}"),
            "attempt=1".into(),
            "provider_try=1".into(),
            "source=catalog".into(),
            "provider=first".into(),
            "stage=resource_access".into(),
            "code=InvalidData".into(),
            "reason=audio_url_contains_userinfo".into(),
            "track_id=track%2Fopaque".into(),
        ] {
            assert!(message.contains(&field), "{message}");
        }
        let fallback = Attempt::new(id, 2, "second", &reference);
        let success = fallback.message("ok", true);
        assert!(success.contains(&format!("request={id}")));
        assert!(success.contains("attempt=2"));
        assert!(success.contains("stage=complete"));
        assert!(!success.contains("account="));
    }
    #[test]
    fn access_diagnostics_report_rules_without_urls_or_headers() {
        for (url, headers, expected) in [
            ("invalid", vec![], "audio_url_parse_failed"),
            (
                "file:///SECRET",
                vec![],
                "audio_url_scheme_not_http_or_https",
            ),
            (
                "https://user:SECRET@example.com",
                vec![],
                "audio_url_contains_userinfo",
            ),
            (
                "https://example.com/#SECRET",
                vec![],
                "audio_url_contains_fragment",
            ),
            (
                "https://example.com/",
                vec![("Authorization".into(), "SECRET".into())],
                "audio_transport_does_not_support_headers",
            ),
        ] {
            let access = ResourceAccess::Http(HttpAccess {
                url: url.into(),
                headers,
            });
            assert_eq!(access_reason(&access), expected);
            assert!(!access_reason(&access).contains("SECRET"));
        }
        assert_eq!(
            metadata_reason(ErrorCode::InvalidData),
            "actual_quality_must_contain_1_to_128_bytes"
        );
    }
    #[test]
    fn opaque_identity_is_redacted_without_hiding_stable_fields() {
        let reference = EntityRef {
            source: SourceId::try_from("catalog".to_owned()).unwrap(),
            kind: EntityKind::Track,
            id: OpaqueId::try_from("opaque/[value] token=SECRET".to_owned()).unwrap(),
        };
        let diagnostic = Attempt::new(42, 3, "fallback", &reference);
        diagnostic.stage("provider_resolve", "provider_request_failed");
        let message =
            crate::infrastructure::diagnostics::redact(&diagnostic.message("InvalidData", false));
        assert!(message.contains("request=42 attempt=3"));
        assert!(message.contains("provider=fallback stage=provider_resolve"));
        assert!(message.contains("code=InvalidData"));
        assert!(!message.contains("SECRET"));
        assert!(!message.contains('\n'));
    }

    #[test]
    fn diagnostic_access_classification_matches_the_actual_validator() {
        let urls = [
            "https://example.com/audio".to_owned(),
            "http://example.com/audio?token=SECRET".to_owned(),
            "invalid".to_owned(),
            "file:///SECRET".to_owned(),
            "https://user:SECRET@example.com".to_owned(),
            "https://:SECRET@example.com".to_owned(),
            "https://example.com/#SECRET".to_owned(),
            format!("https://example.com/{}", "a".repeat(8192)),
        ];
        for url in urls {
            for headers in [vec![], vec![("Authorization".into(), "SECRET".into())]] {
                let access = ResourceAccess::Http(HttpAccess {
                    url: url.clone(),
                    headers,
                });
                let result = validate_access(&access);
                let reason = access_reason(&access);
                assert_eq!(
                    result.is_ok(),
                    reason == "resource_access_invalid",
                    "{reason}"
                );
                if let Err(error) = result {
                    assert_eq!(
                        error.code,
                        if reason == "audio_transport_does_not_support_headers" {
                            ErrorCode::Unsupported
                        } else {
                            ErrorCode::InvalidData
                        }
                    );
                }
                assert!(!reason.contains("SECRET"));
            }
        }
    }
}
