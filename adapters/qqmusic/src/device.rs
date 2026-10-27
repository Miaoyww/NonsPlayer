//! Synthetic Android identity and device-bound QQ protocol bootstrap.
use crate::{crypto, Error, Host, Result};
use serde_json::{json, Value};
use time::OffsetDateTime;
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

const CHANNEL_ID: &str = "10003505";
const QIMEI_APP_KEY: &str = "0AND0HD6FE4HY80F";
const QIMEI_SECRET: &str = "ZdJqM15EeO2zWc08";
const QIMEI_URL: &str = "https://api.tencentmusic.com/tme/trpc/proxy";
const QIMEI_TTL_MS: u64 = 24 * 60 * 60 * 1000;
const QIMEI_RSA_MODULUS: &str = concat!(
    "c4231830a2eb5fc2827170641e79d80fec51bda9a22e4b4ab37d1f205a4ae44d928cda25879f66a3429051663312a12",
    "7faf8a246bdaaf63918417e90d7c95b5908aa6a2d0f852e4a6770294a548ac1c2fe8f1f252fb826f4ac86ab9a00e7ce4",
    "7d002a56e7c4b51eb889acc60ca6adbc9f72e81f4d31b1dd7464805264530ab1d"
);

pub fn identifier(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}
pub fn credential_music_id(credential: &Value) -> String {
    let string_id = identifier(&credential["str_musicid"]).trim().to_owned();
    let raw_id = identifier(&credential["musicid"]).trim().to_owned();
    if !string_id.is_empty() && string_id != "0" {
        string_id
    } else if !raw_id.is_empty() && raw_id != "0" {
        raw_id
    } else if !string_id.is_empty() {
        string_id
    } else {
        raw_id
    }
}
pub fn is_valid(value: &Value) -> bool {
    const REQUIRED: &[&str] = &[
        "display",
        "product",
        "device",
        "board",
        "model",
        "fingerprint",
        "procVersion",
        "imei",
        "brand",
        "androidId",
        "openUdid",
        "osRelease",
    ];
    const OPTIONAL: &[&str] = &["qimei", "qimei36", "sessionUid", "sessionSid"];
    let text = |key: &str| value[key].as_str().is_some_and(|text| !text.is_empty());
    value.is_object()
        && value["sdk"].as_f64().is_some_and(f64::is_finite)
        && (value.get("qimeiSavedAt").is_none()
            || value["qimeiSavedAt"].as_f64().is_some_and(f64::is_finite))
        && REQUIRED.iter().all(|key| text(key))
        && OPTIONAL
            .iter()
            .all(|key| value.get(*key).is_none() || text(key))
}
pub fn random_hex(host: &impl Host, length: usize) -> Result<String> {
    if length > 128 {
        return Err(Error::new("invalidData", "device.random-size"));
    }
    let count = length.div_ceil(2);
    let bytes = host.random(count)?;
    if bytes.len() != count {
        return Err(Error::new("invalidData", "device.random-length"));
    }
    Ok(crypto::hex(&bytes)[..length].to_owned())
}
fn random_int(host: &impl Host, maximum: u32) -> Result<u32> {
    if maximum <= 1 {
        return Ok(0);
    }
    let limit = (1u64 << 32) / maximum as u64 * maximum as u64;
    loop {
        let bytes = host.random(4)?;
        let bytes: [u8; 4] = bytes
            .try_into()
            .map_err(|_| Error::new("invalidData", "device.random-length"))?;
        let value = u32::from_le_bytes(bytes);
        if (value as u64) < limit {
            return Ok(value % maximum);
        }
    }
}
pub fn random_digits(host: &impl Host, length: usize) -> Result<String> {
    if length > 128 {
        return Err(Error::new("invalidData", "device.random-size"));
    }
    (0..length)
        .map(|_| random_int(host, 10).map(|digit| char::from(b'0' + digit as u8)))
        .collect()
}
fn create(host: &impl Host) -> Result<Value> {
    let mut imei = random_digits(host, 14)?;
    let checksum: u32 = imei
        .bytes()
        .enumerate()
        .map(|(index, byte)| {
            let mut digit = u32::from(byte - b'0');
            if index % 2 == 1 {
                digit *= 2;
                if digit > 9 {
                    digit -= 9;
                }
            }
            digit
        })
        .sum();
    imei.push(char::from(b'0' + ((10 - checksum % 10) % 10) as u8));
    Ok(json!({
        "display": format!("QMAPI.{}.001", random_digits(host, 6)?),
        "product": "iarim", "device": "sagit", "board": "eomam", "model": "MI 6",
        "fingerprint": format!("xiaomi/iarim/sagit:10/eomam.200122.001/{}:user/release-keys", random_digits(host, 7)?),
        "procVersion": format!("Linux 5.4.0-54-generic-{} (android-build@google.com)", random_hex(host, 8)?),
        "imei": imei, "brand": "Xiaomi", "androidId": random_hex(host, 16)?,
        "openUdid": random_hex(host, 32)?, "osRelease": "10", "sdk": 29
    }))
}
fn utc(now: u64) -> Result<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(now) * 1_000_000)
        .map_err(|_| Error::new("invalidData", "device.clock-range"))
}
fn beacon(host: &impl Host, now: u64) -> Result<String> {
    let date = utc(now)?;
    let month = format!("{:04}-{:02}-01", date.year(), u8::from(date.month()));
    let first = random_digits(host, 6)?;
    let second = random_digits(host, 9)?;
    let dated = [1, 2, 13, 14, 17, 18, 21, 22, 25, 26, 29, 30, 33, 34, 37, 38];
    let mut fields = Vec::with_capacity(40);
    for key in 1..=40 {
        let value = if dated.contains(&key) {
            format!("{month}{first}.{second}")
        } else if key == 3 {
            "0000000000000000".into()
        } else if key == 4 {
            random_hex(host, 16)?.replace('0', "1")
        } else {
            random_int(host, 10_000)?.to_string()
        };
        fields.push(format!("k{key}:{value}"));
    }
    Ok(format!("{};", fields.join(";")))
}
fn payload(host: &impl Host, device: &Value, now: u64) -> Result<Value> {
    let uptime = utc(now.saturating_sub(u64::from(random_int(host, 14_401)?) * 1000))?;
    let uptime = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        uptime.year(),
        u8::from(uptime.month()),
        uptime.day(),
        uptime.hour(),
        uptime.minute(),
        uptime.second()
    );
    let reserved = json!({
        "harmony": "0", "clone": "0", "containe": "",
        "oz": "UhYmelwouA+V2nPWbOvLTgN2/m8jwGB+yUB5v9tysQg=", "oo": "Xecjt+9S1+f8Pz2VLSxgpw==",
        "kelong": "0", "uptimes": uptime, "multiUser": "0", "bod": device["brand"],
        "dv": device["device"], "firstLevel": "", "manufact": device["brand"],
        "name": device["model"], "host": "se.infra", "kernel": device["procVersion"]
    });
    Ok(json!({
        "androidId": device["androidId"], "platformId": 1, "appKey": QIMEI_APP_KEY,
        "appVersion": "14.9.0.8", "beaconIdSrc": beacon(host, now)?, "brand": device["brand"],
        "channelId": CHANNEL_ID, "cid": "", "imei": device["imei"], "imsi": "", "mac": "",
        "model": device["model"], "networkType": "unknown", "oaid": "",
        "osVersion": format!("Android {},level {}", identifier(&device["osRelease"]), identifier(&device["sdk"])),
        "qimei": "", "qimei36": "", "sdkVersion": "1.2.13.6", "targetSdkVersion": "33",
        "audit": "", "userId": "{}", "packageId": "com.tencent.qqmusic", "deviceType": "Phone",
        "sdkName": "", "reserved": reserved.to_string()
    }))
}
pub fn qimei_request(
    host: &impl Host,
    device: &Value,
    now: u64,
) -> Result<(Vec<(String, String)>, Value)> {
    let crypt_key = Zeroizing::new(random_hex(host, 16)?);
    let nonce = random_hex(host, 16)?;
    let timestamp = now / 1000;
    let modulus = crypto::decode_hex(QIMEI_RSA_MODULUS)?;
    let encrypted_key = crypto::rsaes_pkcs1_encrypt(host, &modulus, 65_537, crypt_key.as_bytes())?;
    let plaintext = Zeroizing::new(payload(host, device, now)?.to_string());
    let encrypted_payload = crypto::aes_cbc_encrypt(
        crypt_key.as_bytes(),
        crypt_key.as_bytes(),
        plaintext.as_bytes(),
    )?;
    let key = crypto::b64(&encrypted_key);
    let params = crypto::b64(&encrypted_payload);
    let extra = format!("{{\"appKey\":\"{QIMEI_APP_KEY}\"}}");
    let sign = crypto::md5_hex(
        format!(
            "{key}{params}{}{nonce}{QIMEI_SECRET}{extra}",
            timestamp * 1000
        )
        .as_bytes(),
    );
    // Host determines connection framing; protocol headers contain no forged Host field.
    let headers = vec![
        ("Content-Type".into(), "application/json".into()),
        ("method".into(), "GetQimei".into()),
        (
            "service".into(),
            "trpc.tme_datasvr.qimeiproxy.QimeiProxy".into(),
        ),
        ("appid".into(), "qimei_qq_android".into()),
        (
            "sign".into(),
            crypto::md5_hex(
                format!("qimei_qq_androidpzAuCmaFAaFaHrdakPjLIEqKrGnSOOvH{timestamp}").as_bytes(),
            ),
        ),
        ("user-agent".into(), "QQMusic".into()),
        ("timestamp".into(), timestamp.to_string()),
    ];
    Ok((
        headers,
        json!({ "app": 0, "os": 1, "qimeiParams": {
            "key": key, "params": params, "time": timestamp.to_string(), "nonce": nonce, "sign": sign, "extra": extra
        }}),
    ))
}
fn dictionary(value: &Value) -> Value {
    if value.is_object() {
        return value.clone();
    }
    value
        .as_str()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}
fn json_response(response: crate::Response, stage: &str) -> Result<Value> {
    if !(200..300).contains(&response.status) {
        return Err(Error::new(
            if response.status == 429 {
                "rateLimited"
            } else {
                "network"
            },
            stage,
        ));
    }
    serde_json::from_slice(&response.body).map_err(|_| Error::new("invalidData", stage))
}
pub fn persist(host: &impl Host, device: &Value) -> Result<()> {
    let mut safe = device.clone();
    let object = safe
        .as_object_mut()
        .ok_or_else(|| Error::new("invalidData", "device.shape"))?;
    object.retain(|key, _| !key.starts_with("session"));
    host.state_set("device", &safe.to_string())
}
fn ensure_qimei(host: &impl Host, device: &mut Value) -> Result<()> {
    let now = host.now_ms();
    let has_text = |key: &str| device[key].as_str().is_some_and(|text| !text.is_empty());
    if has_text("qimei")
        && has_text("qimei36")
        && device["qimeiSavedAt"]
            .as_u64()
            .is_some_and(|saved| saved > 0 && now.saturating_sub(saved) < QIMEI_TTL_MS)
    {
        return Ok(());
    }
    let (headers, body) = qimei_request(host, device, now)?;
    let response = json_response(
        host.http(
            "POST",
            QIMEI_URL,
            headers,
            body.to_string().into_bytes(),
            10_000,
        )?,
        "qimei.response",
    )?;
    let outer = dictionary(&response);
    let inner = dictionary(&outer["data"]);
    let q16 = inner["data"]["q16"]
        .as_str()
        .filter(|text| !text.is_empty());
    let q36 = inner["data"]["q36"]
        .as_str()
        .filter(|text| !text.is_empty());
    let (Some(q16), Some(q36)) = (q16, q36) else {
        let code = crate::model::safe_integer(&outer["code"])
            .or_else(|| crate::model::safe_integer(&inner["code"]))
            .map(|code| format!("upstream-{code}"))
            .unwrap_or_else(|| "invalidData".into());
        host.diagnostic("device-bootstrap", "qimei", &code);
        return Err(Error::new("sourceUnavailable", "device-bootstrap"));
    };
    device["qimei"] = json!(q16);
    device["qimei36"] = json!(q36);
    device["qimeiSavedAt"] = json!(now);
    persist(host, device)
}
pub fn comm(device: &Value, credential: Option<&Value>, overrides: &Value) -> Value {
    let mut comm = json!({
        "ct": 11, "cv": 14090008, "v": 14090008, "chid": CHANNEL_ID, "tmeAppID": "qqmusic",
        "QIMEI": device["qimei"].as_str().unwrap_or(""), "QIMEI36": device["qimei36"].as_str().unwrap_or(""),
        "OpenUDID": device["openUdid"], "udid": device["openUdid"], "OpenUDID2": device["openUdid"],
        "aid": device["androidId"], "os_ver": device["osRelease"], "phonetype": device["model"],
        "devicelevel": identifier(&device["sdk"]), "newdevicelevel": identifier(&device["sdk"]),
        "rom": device["fingerprint"]
    });
    let object = comm.as_object_mut().expect("comm is an object");
    for (from, to) in [("sessionUid", "uid"), ("sessionSid", "sid")] {
        if let Some(value) = device[from].as_str().filter(|text| !text.is_empty()) {
            object.insert(to.into(), json!(value));
        }
    }
    if let Some(credential) = credential {
        object.insert("qq".into(), json!(credential_music_id(credential)));
        object.insert("authst".into(), credential["musickey"].clone());
        object.insert("tmeLoginType".into(), credential["loginType"].clone());
    }
    if let Some(overrides) = overrides.as_object() {
        for (key, value) in overrides {
            object.insert(key.clone(), value.clone());
        }
    }
    comm
}
pub fn prepare(host: &impl Host) -> Result<Value> {
    let mut device = match host.state_get("device")? {
        Some(value) => {
            let value: Value = serde_json::from_str(&value)
                .map_err(|_| Error::new("invalidData", "device.restore-json"))?;
            if !is_valid(&value) {
                return Err(Error::new("invalidData", "device.restore-shape"));
            }
            value
        }
        None => {
            let value = create(host)?;
            persist(host, &value)?;
            value
        }
    };
    ensure_qimei(host, &mut device)?;
    let body = json!({ "comm": comm(&device, None, &json!({})), "req_0": {
        "module": "music.getSession.session", "method": "GetSession",
        "param": { "uid": device["sessionUid"].as_str().unwrap_or(""), "vkey": 0, "caller": 0 }
    }});
    let response = json_response(
        host.http(
            "POST",
            "https://u.y.qq.com/cgi-bin/musicu.fcg",
            vec![
                ("Content-Type".into(), "application/json".into()),
                (
                    "User-Agent".into(),
                    format!(
                        "QQMusic 14090008(android {})",
                        identifier(&device["osRelease"])
                    ),
                ),
            ],
            body.to_string().into_bytes(),
            10_000,
        )?,
        "session-bootstrap.response",
    )?;
    let root = crate::model::safe_integer(&response["code"])
        .ok_or_else(|| Error::new("invalidData", "session-bootstrap.global-code"))?;
    let upstream = crate::model::safe_integer(&response["req_0"]["code"])
        .ok_or_else(|| Error::new("invalidData", "session-bootstrap.code"))?;
    if !response["req_0"].is_object() || root != 0 || upstream != 0 {
        host.diagnostic(
            "session-bootstrap",
            "musicu",
            &format!("upstream-{}", if upstream != 0 { upstream } else { root }),
        );
        return Err(Error::new("sourceUnavailable", "session-bootstrap"));
    }
    let session = &response["req_0"]["data"]["session"];
    let uid = identifier(&session["uid"]);
    let sid = identifier(&session["sid"]);
    if uid.is_empty() || sid.is_empty() {
        return Err(Error::new("invalidData", "session-bootstrap.identity"));
    }
    device["sessionUid"] = json!(uid);
    device["sessionSid"] = json!(sid);
    device["sessionVkey"] = session["vkey"].clone();
    persist(host, &device)?;
    Ok(device)
}
