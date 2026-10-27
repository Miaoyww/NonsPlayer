use super::*;
use crate::auth::tests::Fixture;

#[test]
fn numeric_protocol_device_string_rejection_is_not_success() {
    let device = json!({"display":"display","product":"product","device":"device","board":"board","model":"model","fingerprint":"fingerprint","procVersion":"kernel","imei":"imei","brand":"brand","androidId":"android","openUdid":"udid","osRelease":"10","sdk":29,"qimei":"q16","qimei36":"q36","qimeiSavedAt":1_700_000_000_000_u64});
    let fixture = Fixture {
        device: Some(device),
        response: json!({"code":0,"req_0":{"code":"1000","data":{"session":{"uid":"id","sid":"sid"}}}}),
    };
    assert_eq!(prepare(&fixture).unwrap_err().code, "sourceUnavailable");
}
