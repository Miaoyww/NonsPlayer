//! 煎饼搜: exact platform ID resource resolution through the shared website protocol.
#[cfg(target_arch = "wasm32")]
mod component;

use nons_music_web_provider::Config;
#[cfg(target_arch = "wasm32")]
use nons_music_web_provider::{AdapterResult, Http, Request, Response};

const CONFIG: Config = Config {
    base: "https://www.jbsou.cn/",
    user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36",
    manifest: include_str!("../manifest.json"),
};

pub fn call(http: &impl nons_music_web_provider::Http, method: &str, args: &str) -> String {
    nons_music_web_provider::call(http, method, args, CONFIG)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct NoNetwork;
    impl nons_music_web_provider::Http for NoNetwork {
        fn send(
            &self,
            _: nons_music_web_provider::Request,
        ) -> nons_music_web_provider::AdapterResult<nons_music_web_provider::Response> {
            panic!("descriptor and unsupported methods must not access network")
        }
    }
    #[test]
    fn descriptor_identity_and_no_search() {
        let value: serde_json::Value =
            serde_json::from_str(&call(&NoNetwork, "music.resource-descriptor", "null")).unwrap();
        assert_eq!(value["data"]["id"], "jbsou");
        assert_eq!(value["data"]["searchPlatforms"], serde_json::json!([]));
        let value: serde_json::Value =
            serde_json::from_str(&call(&NoNetwork, "music.search-resources", "null")).unwrap();
        assert_eq!(value["data"]["code"], "unsupported");
    }
}
