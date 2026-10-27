use super::*;
use crate::Event;

pub(crate) struct Fixture {
    pub response: Value,
    pub device: Option<Value>,
}
impl Fixture {
    fn rpc(code: Value) -> Self {
        Self {
            response: json!({"code":0,"req_0":{"code":code,"data":{"info":{}}}}),
            device: None,
        }
    }
}
impl Host for Fixture {
    fn http(
        &self,
        _: &str,
        _: &str,
        _: Vec<(String, String)>,
        _: Vec<u8>,
        _: u32,
    ) -> Result<Response> {
        Ok(Response {
            status: 200,
            headers: vec![],
            body: self.response.to_string().into_bytes(),
        })
    }
    fn now_ms(&self) -> u64 {
        1_700_000_000_000
    }
    fn random(&self, _: usize) -> Result<Vec<u8>> {
        panic!("unexpected random")
    }
    fn state_get(&self, _: &str) -> Result<Option<String>> {
        Ok(self.device.as_ref().map(Value::to_string))
    }
    fn state_set(&self, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
    fn credential_read(&self) -> Result<Vec<u8>> {
        panic!("unexpected credential read")
    }
    fn credential_publish(&self, _: &str, _: &[u8]) -> Result<()> {
        panic!("unexpected credential publish")
    }
    fn credential_replace(&self, _: &[u8]) -> Result<()> {
        panic!("unexpected credential replace")
    }
    fn credential_invalidate(&self) -> Result<()> {
        panic!("unexpected credential invalidation")
    }
    fn websocket_open(&self, _: &str, _: &str, _: u32) -> Result<u32> {
        panic!("unexpected websocket open")
    }
    fn websocket_send(&self, _: u32, _: &[u8]) -> Result<()> {
        panic!("unexpected websocket send")
    }
    fn websocket_close(&self, _: u32) -> Result<()> {
        panic!("unexpected websocket close")
    }
    fn next_event(&self, _: u32) -> Result<Option<Event>> {
        panic!("unexpected event")
    }
    fn diagnostic(&self, _: &str, _: &str, _: &str) {}
}

#[test]
fn numeric_protocol_rpc_string_rejection_is_not_success() {
    let fixture = Fixture::rpc(json!("1000"));
    let result = raw_rpc(
        &fixture,
        &mut HttpClient::default(),
        &json!({}),
        None,
        "test",
        "read",
        json!({}),
        json!({}),
    );
    assert_eq!(result.unwrap_err().code, "unauthenticated");
}

#[test]
fn numeric_protocol_root_string_rejection_is_not_success() {
    let mut fixture = Fixture::rpc(json!(0));
    fixture.response["code"] = json!("1000");
    let result = raw_rpc(
        &fixture,
        &mut HttpClient::default(),
        &json!({}),
        None,
        "test",
        "read",
        json!({}),
        json!({}),
    );
    assert_eq!(result.unwrap_err().code, "sourceUnavailable");
}

#[test]
fn numeric_protocol_zero_strings_and_omitted_codes_allow_valid_data() {
    for response in [
        json!({"code":"0","req_0":{"code":"0","data":{"result":1}}}),
        json!({"req_0":{"data":{"result":1}}}),
    ] {
        let fixture = Fixture {
            response,
            device: None,
        };
        let result = raw_rpc(
            &fixture,
            &mut HttpClient::default(),
            &json!({}),
            None,
            "test",
            "read",
            json!({}),
            json!({}),
        )
        .unwrap();
        assert_eq!(result["result"], 1);
    }
}

#[test]
fn numeric_protocol_malformed_error_codes_are_not_success() {
    for code in [
        json!("not-a-code"),
        json!("Infinity"),
        json!(1000.5),
        json!(9_007_199_254_740_992_u64),
    ] {
        let fixture = Fixture::rpc(code);
        let result = raw_rpc(
            &fixture,
            &mut HttpClient::default(),
            &json!({}),
            None,
            "test",
            "read",
            json!({}),
            json!({}),
        );
        assert!(result.is_err(), "malformed error code accepted");
    }
}

#[test]
fn numeric_protocol_wechat_string_login_type_is_preserved() {
    let credential = credential_from(
        json!({"musicid":0,"str_musicid":"wechat-id","musickey":"fixture-key","loginType":"1"}),
        6,
    )
    .unwrap();
    assert_eq!(credential["loginType"], 1);
}

#[test]
fn numeric_protocol_login_type_defaults_only_when_omitted() {
    let credential = credential_from(json!({"musicid":42,"musickey":"fixture-key"}), 6).unwrap();
    assert_eq!(credential["loginType"], 6);
    for invalid in [json!("NaN"), json!(1.5), json!(9_007_199_254_740_992_u64)] {
        assert!(credential_from(
            json!({"musicid":42,"musickey":"fixture-key","loginType":invalid}),
            6
        )
        .is_err());
    }
}

#[test]
fn numeric_protocol_string_expiry_uses_credential_lifetime() {
    let now = 1_700_000_000_000;
    assert_eq!(
        expiry(
            &json!({"musickeyCreateTime":"1700000000","keyExpiresIn":"259200"}),
            now
        ),
        now + 3 * 24 * HOUR
    );
}
