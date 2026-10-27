//! QR challenge state and the QQ MQTT v5 protocol over the bounded Host transport.
use super::{credential_from, expiry, raw_rpc, string, HttpClient};
use crate::{crypto, Error, Event, Host, Result};
use bytes::BytesMut;
use mqttbytes::{
    v5::{
        self, Connect, ConnectProperties, ConnectReturnCode, Packet, Publish, Subscribe,
        SubscribeProperties,
    },
    QoS,
};
use serde_json::{json, Value};
use url::Url;

const WECHAT_APP_ID: &str = "wx48db31d50e334801";
const MAX_MQTT: usize = 64 * 1024;
const MAX_EVENTS: usize = 32;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Created,
    Waiting,
    Scanned,
    Confirmed,
    Failed,
    Expired,
}

pub(super) struct Login {
    pub key: String,
    pub method: String,
    pub image: String,
    pub issued: Option<Value>,
    pub published: bool,
    device: Value,
    identifier: String,
    deadline: u64,
    state: State,
    http: HttpClient,
    wechat_http: HttpClient,
    mqtt: Option<Mqtt>,
    poll_errors: u8,
}

impl Login {
    pub fn new(host: &impl Host, method: &str, device: Value) -> Result<Self> {
        Ok(Self {
            key: crypto::hex(&host.random(24)?),
            method: method.into(),
            image: String::new(),
            issued: None,
            published: false,
            device,
            identifier: String::new(),
            deadline: host.now_ms().saturating_add(180_000),
            state: State::Created,
            http: HttpClient::default(),
            wechat_http: HttpClient::default(),
            mqtt: None,
            poll_errors: 0,
        })
    }

    pub fn progress(&self) -> u16 {
        match self.state {
            State::Confirmed => 803,
            State::Scanned => 802,
            State::Failed | State::Expired => 800,
            _ => 801,
        }
    }
    fn terminal(&self) -> bool {
        matches!(
            self.state,
            State::Confirmed | State::Failed | State::Expired
        )
    }
    pub fn close(&mut self, host: &impl Host) {
        if let Some(mqtt) = self.mqtt.take() {
            let _ = host.websocket_close(mqtt.socket);
        }
        self.http.clear();
        self.wechat_http.clear();
    }
    fn fail(&mut self, host: &impl Host, error: &Error) {
        if self.terminal() {
            return;
        }
        self.state = State::Failed;
        host.diagnostic(&error.stage, "QrFailure", &error.code);
        self.close(host);
    }
    fn check_expiry(&mut self, host: &impl Host) -> bool {
        if !self.terminal() && host.now_ms() >= self.deadline {
            self.state = State::Expired;
            host.diagnostic("qr-event", "qr-timeout", "sourceUnavailable");
            self.close(host);
        }
        self.terminal()
    }
    pub fn create_qr(&mut self, host: &impl Host) -> Result<()> {
        if self.check_expiry(host) {
            return Err(Error::new("sourceUnavailable", "qr.expired"));
        }
        if !self.image.is_empty() {
            return Ok(());
        }
        if self.state != State::Created {
            return Err(Error::new("staleContext", "qr.state"));
        }
        let result = if self.method == "qq" {
            self.create_qq(host)
        } else {
            self.create_wechat(host)
        };
        if let Err(error) = &result {
            self.fail(host, error);
        }
        result
    }
    fn create_qq(&mut self, host: &impl Host) -> Result<()> {
        let qr = raw_rpc(
            host,
            &mut self.http,
            &self.device,
            None,
            "music.login.LoginServer",
            "CreateQRCode",
            json!({"tmeAppID":"qqmusic","ct":11,"cv":14090008}),
            json!({"ct":23,"cv":0}),
        )?;
        let id = string(&qr["qrcodeID"]);
        let encoded = string(&qr["qrcode"]);
        let bytes = crypto::decode_b64(encoded.rsplit(',').next().unwrap_or(""))?;
        if id.is_empty() || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(Error::new("invalidData", "qr.png"));
        }
        self.identifier = id;
        self.image = format!("data:image/png;base64,{}", crypto::b64(&bytes));
        if let Some(ttl) = crate::model::safe_integer(&qr["expiresIn"])
            .and_then(|value| u64::try_from(value).ok())
            .filter(|ttl| *ttl > 0)
        {
            self.deadline = self
                .deadline
                .min(host.now_ms().saturating_add(ttl.saturating_mul(1000)));
        }
        let mut mqtt = Mqtt::open(host, &self.identifier)?;
        let pending = std::mem::take(&mut mqtt.pending);
        self.mqtt = Some(mqtt);
        self.state = State::Waiting;
        for publish in pending {
            if self.terminal() {
                break;
            }
            self.publish(host, publish)?;
        }
        self.drain_packets(host)
    }
    fn create_wechat(&mut self, host: &impl Host) -> Result<()> {
        let mut url = Url::parse("https://open.weixin.qq.com/connect/qrconnect")
            .map_err(|_| Error::new("invalidData", "qr.wechat-url"))?;
        url.query_pairs_mut().extend_pairs([
            ("appid", WECHAT_APP_ID),
            (
                "redirect_uri",
                "https://y.qq.com/portal/wx_redirect.html?login_type=2&surl=https://y.qq.com/",
            ),
            ("response_type", "code"),
            ("scope", "snsapi_login"),
            ("state", "STATE"),
            (
                "href",
                "https://y.qq.com/mediastyle/music_v17/src/css/popup_wechat.css#wechat_redirect",
            ),
        ]);
        let page =
            self.wechat_http
                .request(host, "GET", url.as_str(), Vec::new(), Vec::new(), 10000)?;
        let text = std::str::from_utf8(&page.body)
            .map_err(|_| Error::new("invalidData", "qr.wechat-html"))?;
        let id = text
            .split_once("uuid=")
            .and_then(|(_, tail)| tail.split_once('"').map(|(id, _)| id))
            .unwrap_or("");
        if id.is_empty() || id.len() > 512 {
            return Err(Error::new("invalidData", "qr.wechat-uuid"));
        }
        // A UUID is opaque path data; do not let it change the endpoint or query.
        let mut image_url = Url::parse("https://open.weixin.qq.com/connect/qrcode/")
            .map_err(|_| Error::new("invalidData", "qr.wechat-image-url"))?;
        image_url
            .path_segments_mut()
            .map_err(|_| Error::new("invalidData", "qr.wechat-image-url"))?
            .pop_if_empty()
            .push(id);
        let image = self.wechat_http.request(
            host,
            "GET",
            image_url.as_str(),
            vec![(
                "Referer".into(),
                "https://open.weixin.qq.com/connect/qrconnect".into(),
            )],
            Vec::new(),
            10000,
        )?;
        let mime = if image.body.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if image.body.starts_with(&[0xff, 0xd8, 0xff]) {
            "image/jpeg"
        } else {
            return Err(Error::new("invalidData", "qr.wechat-image"));
        };
        self.identifier = id.into();
        self.image = format!("data:{mime};base64,{}", crypto::b64(&image.body));
        self.state = State::Waiting;
        Ok(())
    }

    pub fn advance(&mut self, host: &impl Host, budget_ms: u32) -> Result<()> {
        if self.check_expiry(host) {
            return Ok(());
        }
        let result = if self.method == "wechat" {
            self.poll_wechat(host, budget_ms)
        } else {
            self.poll_qq(host, budget_ms)
        };
        if let Err(error) = &result {
            if !matches!(
                error.code.as_str(),
                "cancelled" | "staleContext" | "permissionDenied"
            ) {
                self.fail(host, error);
                return Ok(());
            }
            self.close(host);
        }
        result
    }
    fn poll_wechat(&mut self, host: &impl Host, budget_ms: u32) -> Result<()> {
        if self.identifier.is_empty() {
            return Ok(());
        }
        let mut url = Url::parse("https://lp.open.weixin.qq.com/connect/l/qrconnect")
            .map_err(|_| Error::new("invalidData", "qr.wechat-poll-url"))?;
        url.query_pairs_mut()
            .append_pair("uuid", &self.identifier)
            .append_pair("_", &host.now_ms().to_string());
        let response = self.wechat_http.request(
            host,
            "GET",
            url.as_str(),
            vec![("Referer".into(), "https://open.weixin.qq.com/".into())],
            Vec::new(),
            budget_ms.max(1).min(1500),
        );
        let response = match response {
            Ok(value) => {
                self.poll_errors = 0;
                value
            }
            Err(error) if error.code == "deadlineExceeded" => return Ok(()),
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "cancelled" | "staleContext" | "permissionDenied"
                ) =>
            {
                return Err(error)
            }
            Err(error) => {
                self.poll_errors = self.poll_errors.saturating_add(1);
                if self.poll_errors > 3 {
                    return Err(error);
                }
                return Ok(());
            }
        };
        let text = std::str::from_utf8(&response.body)
            .map_err(|_| Error::new("invalidData", "qr.wechat-poll"))?;
        let (number, tail) = text
            .split_once("window.wx_errcode=")
            .and_then(|(_, v)| v.split_once(";window.wx_code='"))
            .ok_or_else(|| Error::new("invalidData", "qr.wechat-poll"))?;
        let code = tail
            .split_once('\'')
            .map(|(code, _)| code)
            .ok_or_else(|| Error::new("invalidData", "qr.wechat-poll"))?;
        match number.parse::<u16>().ok() {
            Some(408) => self.state = State::Waiting,
            Some(404) => self.state = State::Scanned,
            Some(405) => self.finalize_wechat(host, code)?,
            Some(402 | 403) => {
                self.state = State::Expired;
                self.close(host);
            }
            _ => return Err(Error::new("sourceUnavailable", "qr.wechat-status")),
        }
        Ok(())
    }
    fn poll_qq(&mut self, host: &impl Host, budget_ms: u32) -> Result<()> {
        self.drain_packets(host)?;
        let started = host.now_ms();
        for _ in 0..MAX_EVENTS {
            if self.terminal() {
                break;
            }
            self.ping(host)?;
            let elapsed = host.now_ms().saturating_sub(started);
            let remaining = (budget_ms as u64).saturating_sub(elapsed);
            let event = host.next_event(remaining.min(25) as u32)?;
            match event {
                Some(event) => self.accept_event(host, event)?,
                None => {
                    if remaining == 0 || host.now_ms().saturating_sub(started) >= budget_ms as u64 {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn event(&mut self, host: &impl Host, value: &Value) -> Result<()> {
        if self.check_expiry(host) {
            return Ok(());
        }
        if self.method != "qq" {
            return Ok(());
        }
        let result = (|| {
            match value["kind"].as_str() {
                Some("binary") => {
                    let socket = value["socket"]
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or_else(|| Error::new("invalidData", "mqtt.event-socket"))?;
                    let encoded = value["data"]
                        .as_str()
                        .ok_or_else(|| Error::new("invalidData", "mqtt.event-bytes"))?;
                    self.accept_event(
                        host,
                        Event::Binary {
                            socket,
                            data: crypto::decode_b64(encoded)?,
                        },
                    )?;
                }
                Some("closed") => {
                    if let Some(socket) =
                        value["socket"].as_u64().and_then(|v| u32::try_from(v).ok())
                    {
                        self.accept_event(host, Event::Closed(socket))?;
                    }
                }
                Some("tick") => self.ping(host)?,
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.fail(host, error);
            if !matches!(
                error.code.as_str(),
                "cancelled" | "staleContext" | "permissionDenied"
            ) {
                return Ok(());
            }
        }
        result
    }
    fn ping(&mut self, host: &impl Host) -> Result<()> {
        if let Some(mqtt) = &mut self.mqtt {
            if host.now_ms().saturating_sub(mqtt.last_ping) >= 30000 {
                host.websocket_send(mqtt.socket, &[0xc0, 0])?;
                mqtt.last_ping = host.now_ms();
            }
        }
        Ok(())
    }
    fn accept_event(&mut self, host: &impl Host, event: Event) -> Result<()> {
        let mqtt = match &mut self.mqtt {
            Some(mqtt) => mqtt,
            None => return Ok(()),
        };
        match event {
            Event::Closed(socket) if socket == mqtt.socket => {
                return Err(Error::new("network", "mqtt.closed"))
            }
            Event::Binary { socket, data } if socket == mqtt.socket => mqtt.feed(&data)?,
            _ => return Ok(()),
        }
        self.drain_packets(host)
    }
    fn drain_packets(&mut self, host: &impl Host) -> Result<()> {
        let mqtt = match &mut self.mqtt {
            Some(mqtt) => mqtt,
            None => return Ok(()),
        };
        let mut publishes = Vec::new();
        let mut packet_count = 0;
        while let Some(packet) = mqtt.packet()? {
            packet_count += 1;
            if packet_count > MAX_EVENTS {
                return Err(Error::new("rateLimited", "mqtt.event-count"));
            }
            if let Packet::Publish(publish) = packet {
                publishes.push(publish);
            }
        }
        for publish in publishes {
            if !self.terminal() {
                self.publish(host, publish)?;
            }
        }
        Ok(())
    }
    fn publish(&mut self, host: &impl Host, publish: Publish) -> Result<()> {
        let kind = publish
            .properties
            .as_ref()
            .and_then(|props| {
                props
                    .user_properties
                    .iter()
                    .find(|(key, _)| key == "type")
                    .map(|(_, value)| value.as_str())
            })
            .unwrap_or("");
        let payload = if publish.payload.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&publish.payload)
                .map_err(|_| Error::new("invalidData", "mqtt.payload-json"))?
        };
        match kind {
            "waiting" => self.state = State::Waiting,
            "scanned" => self.state = State::Scanned,
            "cookies" => self.finalize_qq(host, &payload)?,
            "canceled" | "timeout" | "loginFailed" => {
                self.state = State::Expired;
                host.diagnostic(
                    "qr-event",
                    match kind {
                        "canceled" => "user-canceled",
                        "timeout" => "qr-timeout",
                        _ => "login-rejected",
                    },
                    "sourceUnavailable",
                );
                self.close(host);
            }
            _ => {}
        }
        Ok(())
    }
    fn finalize_qq(&mut self, host: &impl Host, payload: &Value) -> Result<()> {
        let id = string(&payload["cookies"]["qqmusic_uin"]["value"]);
        let token = string(&payload["cookies"]["qqmusic_key"]["value"]);
        if id.is_empty() || token.is_empty() || self.identifier.is_empty() {
            return Err(Error::new("invalidData", "credential-payload"));
        }
        self.state = State::Scanned;
        let numeric = id
            .parse::<u64>()
            .map_err(|_| Error::new("invalidData", "credential-payload"))?;
        let exchanged = raw_rpc(
            host,
            &mut self.http,
            &self.device,
            None,
            "music.login.LoginServer",
            "Login",
            json!({"musicid":numeric,"qrCodeID":self.identifier,"token":token}),
            json!({"tmeLoginType":6}),
        );
        let credential = match exchanged.and_then(|value| credential_from(value, 6)) {
            Ok(value) => value,
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "cancelled" | "staleContext" | "permissionDenied"
                ) =>
            {
                return Err(error)
            }
            Err(_) => {
                let direct = json!({"musicid":id,"str_musicid":id,"musickey":token,"loginType":6});
                raw_rpc(
                    host,
                    &mut self.http,
                    &self.device,
                    Some(&direct),
                    "music.UserInfo.userInfoServer",
                    "GetLoginUserInfo",
                    json!({}),
                    json!({}),
                )?;
                direct
            }
        };
        self.issue(host, credential)
    }
    fn finalize_wechat(&mut self, host: &impl Host, code: &str) -> Result<()> {
        if code.is_empty() {
            return Err(Error::new("invalidData", "credential-payload"));
        }
        self.state = State::Scanned;
        let credential = credential_from(
            raw_rpc(
                host,
                &mut self.http,
                &self.device,
                None,
                "music.login.LoginServer",
                "Login",
                json!({"code":code,"strAppid":WECHAT_APP_ID}),
                json!({"tmeLoginType":1}),
            )?,
            1,
        )?;
        let checked = raw_rpc(
            host,
            &mut self.http,
            &self.device,
            Some(&credential),
            "music.UserInfo.userInfoServer",
            "GetLoginUserInfo",
            json!({}),
            json!({}),
        );
        let credential = match checked {
            Ok(_) => credential,
            Err(error) if error.code == "unauthenticated" => credential_from(
                raw_rpc(
                    host,
                    &mut self.http,
                    &self.device,
                    Some(&credential),
                    "music.login.LoginServer",
                    "Login",
                    json!({
                "openid":string(&credential["openid"]),"refresh_token":string(&credential["refresh_token"]),
                "str_musicid":super::music_id(&credential),"musickey":credential["musickey"],"unionid":string(&credential["unionid"]),
                "refresh_key":string(&credential["refresh_key"]),"loginMode":2}),
                    json!({"tmeLoginType":1}),
                )?,
                1,
            )?,
            Err(error) => return Err(error),
        };
        self.issue(host, credential)
    }
    fn issue(&mut self, host: &impl Host, credential: Value) -> Result<()> {
        let session = json!({"token":"current","credential":credential,"device":self.device,"expiresAt":expiry(&credential,host.now_ms())});
        if !super::valid_session(&session) {
            return Err(Error::new("invalidData", "session-issue"));
        }
        self.issued = Some(session);
        self.state = State::Confirmed;
        self.close(host);
        Ok(())
    }
}

struct Mqtt {
    socket: u32,
    buffer: BytesMut,
    pending: Vec<Publish>,
    last_ping: u64,
}
impl Mqtt {
    fn open(host: &impl Host, id: &str) -> Result<Self> {
        let mut path = "/ws/handshake".to_string();
        for redirect in 0..=3 {
            let socket = host.websocket_open(&format!("wss://mu.y.qq.com{path}"), "mqtt", 15000)?;
            let mut mqtt = Self {
                socket,
                buffer: BytesMut::new(),
                pending: Vec::new(),
                last_ping: host.now_ms(),
            };
            let opened = (|| {
                let random = host
                    .random(4)?
                    .iter()
                    .map(|byte| char::from(b'0' + byte % 10))
                    .collect::<String>();
                let mut connect = Connect::new(format!("{}{random}", host.now_ms()));
                connect.keep_alive = 45;
                connect.properties = Some(ConnectProperties {
                    session_expiry_interval: None,
                    receive_maximum: None,
                    max_packet_size: None,
                    topic_alias_max: None,
                    request_response_info: None,
                    request_problem_info: None,
                    authentication_method: Some("pass".into()),
                    authentication_data: None,
                    user_properties: vec![
                        ("tmeAppID".into(), "qqmusic".into()),
                        ("business".into(), "management".into()),
                        ("hashTag".into(), id.into()),
                        ("clientTag".into(), "management.user".into()),
                        ("userID".into(), id.into()),
                    ],
                });
                let mut bytes = BytesMut::new();
                connect
                    .write(&mut bytes)
                    .map_err(|_| Error::new("invalidData", "mqtt.connect-encode"))?;
                host.websocket_send(socket, &bytes)?;
                let packet = mqtt
                    .next(host, 20000)?
                    .ok_or_else(|| Error::new("deadlineExceeded", "mqtt.connack-timeout"))?;
                match packet {
                    Packet::ConnAck(ack) if ack.code == ConnectReturnCode::Success => Ok(None),
                    Packet::ConnAck(ack)
                        if matches!(
                            ack.code,
                            ConnectReturnCode::UseAnotherServer | ConnectReturnCode::ServerMoved
                        ) =>
                    {
                        let reference = ack
                            .properties
                            .and_then(|properties| properties.server_reference)
                            .filter(|reference| !reference.is_empty())
                            .ok_or_else(|| {
                                Error::new("sourceUnavailable", "mqtt.redirect-reference")
                            })?;
                        Ok(Some(reference))
                    }
                    _ => Err(Error::new("sourceUnavailable", "mqtt.connack")),
                }
            })();
            match opened {
                Ok(None) => {
                    let subscription = mqtt.subscribe(host, id);
                    if let Err(error) = subscription {
                        let _ = host.websocket_close(socket);
                        return Err(error);
                    }
                    return Ok(mqtt);
                }
                Ok(Some(reference)) => {
                    let _ = host.websocket_close(socket);
                    if redirect == 3 {
                        return Err(Error::new("sourceUnavailable", "mqtt.redirect-limit"));
                    }
                    // The protocol's reference is a path segment on the same authorized broker.
                    if reference.len() > 1024
                        || reference.chars().any(|ch| matches!(ch, '?' | '#' | '\\'))
                        || reference.starts_with('/')
                    {
                        return Err(Error::new("invalidData", "mqtt.redirect-reference"));
                    }
                    let mut pieces = path
                        .trim_end_matches('/')
                        .split('/')
                        .map(str::to_owned)
                        .collect::<Vec<_>>();
                    if pieces.last().map_or(false, |part| part.contains(':')) {
                        pieces.pop();
                    }
                    pieces.push(reference);
                    path = pieces.join("/");
                }
                Err(error) => {
                    let _ = host.websocket_close(socket);
                    return Err(error);
                }
            }
        }
        Err(Error::new("sourceUnavailable", "mqtt.redirect-limit"))
    }
    fn subscribe(&mut self, host: &impl Host, id: &str) -> Result<()> {
        let mut subscribe =
            Subscribe::new(format!("management.qrcode_login/{id}"), QoS::AtMostOnce);
        subscribe.pkid = 1;
        subscribe.properties = Some(SubscribeProperties {
            id: None,
            user_properties: vec![
                ("authorization".into(), "tmelogin".into()),
                ("pubsub".into(), "unicast".into()),
            ],
        });
        let mut bytes = BytesMut::new();
        subscribe
            .write(&mut bytes)
            .map_err(|_| Error::new("invalidData", "mqtt.subscribe-encode"))?;
        host.websocket_send(self.socket, &bytes)?;
        let started = host.now_ms();
        for _ in 0..MAX_EVENTS {
            let remaining = 20_000_u64.saturating_sub(host.now_ms().saturating_sub(started));
            if remaining == 0 {
                return Err(Error::new("deadlineExceeded", "mqtt.suback-timeout"));
            }
            match self.next(host, remaining as u32)? {
                Some(Packet::SubAck(ack))
                    if ack.pkid == 1
                        && !ack.return_codes.is_empty()
                        && ack.return_codes.iter().all(|code| (*code as u8) < 0x80) =>
                {
                    return Ok(())
                }
                Some(Packet::SubAck(_)) => {
                    return Err(Error::new("sourceUnavailable", "mqtt.suback"))
                }
                Some(Packet::Publish(publish)) => {
                    if self.pending.len() >= MAX_EVENTS {
                        return Err(Error::new("rateLimited", "mqtt.event-count"));
                    }
                    self.pending.push(publish);
                }
                Some(_) => {}
                None => return Err(Error::new("deadlineExceeded", "mqtt.suback-timeout")),
            }
        }
        Err(Error::new("rateLimited", "mqtt.handshake-count"))
    }
    fn next(&mut self, host: &impl Host, timeout_ms: u32) -> Result<Option<Packet>> {
        let started = host.now_ms();
        loop {
            if let Some(packet) = self.packet()? {
                return Ok(Some(packet));
            }
            let remaining =
                (timeout_ms as u64).saturating_sub(host.now_ms().saturating_sub(started));
            if remaining == 0 {
                return Ok(None);
            }
            match host.next_event(remaining.min(1000) as u32)? {
                Some(Event::Binary { socket, data }) if socket == self.socket => {
                    self.feed(&data)?
                }
                Some(Event::Closed(socket)) if socket == self.socket => {
                    return Err(Error::new("network", "mqtt.closed"))
                }
                None if host.now_ms().saturating_sub(started) >= timeout_ms as u64 => {
                    return Ok(None)
                }
                _ => {}
            }
        }
    }
    fn feed(&mut self, data: &[u8]) -> Result<()> {
        if self.buffer.len().saturating_add(data.len()) > MAX_MQTT {
            return Err(Error::new("invalidData", "mqtt.buffer-size"));
        }
        self.buffer.extend_from_slice(data);
        Ok(())
    }
    fn packet(&mut self) -> Result<Option<Packet>> {
        match v5::read(&mut self.buffer, MAX_MQTT) {
            Ok(packet) => Ok(Some(packet)),
            Err(mqttbytes::Error::InsufficientBytes(_)) => Ok(None),
            Err(_) => Err(Error::new("invalidData", "mqtt.packet")),
        }
    }
}
