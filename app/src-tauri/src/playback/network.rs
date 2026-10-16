//! Configure native HTTP sources; API requests already use reqwest's proxy support.
use gstreamer::{self as gst, prelude::*};

pub fn prefer_http_source() {
    static REGISTER: std::sync::Once = std::sync::Once::new();
    REGISTER
        .call_once(|| gstreqwest::plugin_register_static().expect("register bundled HTTP source"));
}

pub fn configure_source(source: &gst::Element) {
    if source.find_property("timeout").is_some() {
        source.set_property_from_str("timeout", "12");
    }
    if source.find_property("retries").is_some() {
        source.set_property_from_str("retries", "2");
    }
    #[cfg(windows)]
    if source.find_property("proxy").is_some() {
        // libcurl does not read Windows' manual system proxy. Respect explicit
        // environment configuration first; do not change the system settings.
        if [
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
        ]
        .iter()
        .any(|key| std::env::var(key).is_ok_and(|v| !v.is_empty()))
        {
            return;
        }
        if let Ok(proxy) = (sysproxy::Sysproxy::get_system_proxy()).inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("optional_success", error);
        }) {
            if proxy.enable && !proxy.host.is_empty() {
                let location = source
                    .find_property("location")
                    .and_then(|_| source.property::<Option<String>>("location"));
                let host = location
                    .as_deref()
                    .and_then(|s| {
                        url::Url::parse(s)
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("optional_result", error);
                            })
                            .ok()
                    })
                    .and_then(|u| u.host_str().map(str::to_owned));
                let bypass = host.is_some_and(|host| {
                    proxy.bypass.split([';', ',']).any(|pattern| {
                        let pattern = pattern.trim();
                        (pattern.eq_ignore_ascii_case("<local>") && !host.contains('.'))
                            || glob::Pattern::new(pattern).is_ok_and(|p| {
                                p.matches_with(
                                    &host,
                                    glob::MatchOptions {
                                        case_sensitive: false,
                                        require_literal_separator: false,
                                        require_literal_leading_dot: false,
                                    },
                                )
                            })
                    })
                });
                if !bypass {
                    let mut address =
                        url::Url::parse("http://localhost").expect("constant proxy URL");
                    if address.set_host(Some(&proxy.host)).is_ok()
                        && address.set_port(Some(proxy.port)).is_ok()
                    {
                        source.set_property("proxy", address.as_str());
                    }
                }
            }
        }
    }
}
