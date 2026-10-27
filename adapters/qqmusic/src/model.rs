use crate::{Error, Result};
use serde_json::{json, Value};

pub fn text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    }
}
pub fn first<'a>(value: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter()
        .map(|key| &value[*key])
        .find(|value| !value.is_null())
        .unwrap_or(&Value::Null)
}
pub fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => {
            if value.trim().is_empty() {
                Some(0.0)
            } else {
                value.trim().parse().ok()
            }
        }
        Value::Null => Some(0.0),
        Value::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        _ => None,
    }
    .filter(|value| value.is_finite())
}
pub(crate) fn safe_integer(value: &Value) -> Option<i64> {
    let value = number(value)?;
    (value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_991.0).then_some(value as i64)
}
pub fn json_number(value: f64) -> Value {
    if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
        json!(value as u64)
    } else {
        json!(value)
    }
}
pub fn positive_id(value: &Value) -> Option<u64> {
    let value = number(value)?;
    (value > 0.0 && value <= 9_007_199_254_740_991.0 && value.fract() == 0.0)
        .then_some(value as u64)
}
pub fn alphanumeric(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|c| c.is_ascii_alphanumeric())
}
pub fn reference(kind: &str, id: &str) -> Value {
    json!({"source":"qqmusic","kind":kind,"id":id})
}
pub fn check_reference<'a>(value: &'a Value, kind: &str) -> Result<&'a str> {
    let id = value["id"].as_str().unwrap_or("");
    let numeric = id.strip_prefix("id:").is_some_and(|s| {
        !s.is_empty() && !s.starts_with('0') && s.bytes().all(|c| c.is_ascii_digit())
    });
    if value["source"] != "qqmusic"
        || value["kind"] != kind
        || !(alphanumeric(id) || kind == "track" && numeric)
    {
        return Err(Error::new("invalidData", "reference.source-kind-id"));
    }
    Ok(id)
}
pub fn image(value: &Value) -> Value {
    let url = text(value);
    json!({"original":url,"small":url,"normal":url,"large":url,"exlarge":url})
}
pub fn artwork(mid: &str, kind: &str) -> Value {
    if !alphanumeric(mid) {
        return image(&Value::Null);
    }
    let prefix = if kind == "artist" { "T001" } else { "T002" };
    let url =
        |size| format!("https://y.gtimg.cn/music/photo_new/{prefix}R{size}x{size}M000{mid}.jpg");
    json!({"original":url(800),"small":url(150),"normal":url(300),"large":url(500),"exlarge":url(800)})
}
pub fn credits(singers: &Value) -> Vec<Value> {
    singers.as_array().into_iter().flatten().filter_map(|value| {
        let name = text(&value["name"]); if name.is_empty() { return None; }
        let mid = ["mid", "singer_mid"].into_iter().map(|key| text(&value[key])).find(|s| !s.is_empty()).unwrap_or_default();
        Some(json!({"name":name,"reference":if mid.is_empty() {Value::Null} else {reference("artist", &mid)}}))
    }).collect()
}
pub fn original_song(value: &Value) -> &Value {
    let inner = first(value, &["songInfo", "musicData"]);
    if inner.is_null() {
        value
    } else {
        inner
    }
}
pub fn track(value: &Value, unavailable: Option<&str>) -> Result<Value> {
    let song = original_song(value);
    let mid = ["mid", "songmid"]
        .into_iter()
        .map(|key| text(&song[key]))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    let numeric = positive_id(first(song, &["id", "songid"]));
    let id = if !mid.is_empty() {
        mid.clone()
    } else if unavailable.is_some() {
        numeric.map(|id| format!("id:{id}")).unwrap_or_default()
    } else {
        String::new()
    };
    if id.is_empty() {
        let mut fields = song
            .as_object()
            .into_iter()
            .flat_map(|song| song.keys())
            .filter(|key| {
                !key.is_empty()
                    && key
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            })
            .collect::<Vec<_>>();
        fields.sort_unstable();
        let fields = fields
            .into_iter()
            .take(8)
            .map(|key| key.replace('_', "-"))
            .collect::<Vec<_>>()
            .join(".");
        let mut class = format!("MissingMid.{fields}");
        class.truncate(64);
        return Err(Error::new("invalidData", "track.mid").with_class(class));
    }
    let album_mid = text(if song["album"]["mid"].is_null() {
        &song["albummid"]
    } else {
        &song["album"]["mid"]
    });
    let artists = credits(&song["singer"]);
    let mut identifiers = Vec::new();
    if !mid.is_empty() {
        identifiers.push(json!({"kind":"song-mid","id":mid}));
    }
    if let Some(id) = numeric {
        identifiers.push(json!({"kind":"song-id","id":id.to_string()}));
    }
    let mut result = json!({"reference":reference("track",&id),"title":text(first(song,&["title","name","songname"])),"aliases":if text(&song["subtitle"]).is_empty() {vec![]} else {vec![text(&song["subtitle"])]},"artist":artists.iter().map(|v| text(&v["name"])).collect::<Vec<_>>().join(" / "),"artists":artists,"album":text(if song["album"]["name"].is_null(){ &song["albumname"] }else{ &song["album"]["name"] }),"albumReference":if album_mid.is_empty(){Value::Null}else{reference("album",&album_mid)},"durationMs":json_number((number(&song["interval"]).unwrap_or(0.0)*1000.0).round().max(0.0)),"cover":artwork(&album_mid,"album"),"lyricIdentifiers":identifiers,"associations":[]});
    if let Some(reason) = unavailable {
        result["unavailableReason"] = json!(reason);
    }
    Ok(result)
}
fn published_at(value: &Value) -> Option<i64> {
    let s = value.as_str()?;
    let b = s.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return None;
    }
    let year = s[..4].parse().ok()?;
    let month = time::Month::try_from(s[5..7].parse::<u8>().ok()?).ok()?;
    let day = s[8..].parse().ok()?;
    let date = time::Date::from_calendar_date(year, month, day).ok()?;
    Some(date.midnight().assume_utc().unix_timestamp() * 1000)
}
pub fn entity(value: &Value, kind: &str) -> Result<Value> {
    let id = text(first(
        value,
        if kind == "playlist" {
            &["disstid", "tid", "dissid"]
        } else {
            &[
                "mid",
                "albumMid",
                "albummid",
                "singer_mid",
                "singerMID",
                "singermid",
            ]
        },
    ));
    if id.is_empty() {
        return Err(Error::new("invalidData", "entity.id"));
    }
    let subtitle = if kind == "playlist" {
        [
            &value["creator"]["name"],
            &value["creator"]["nick"],
            &value["creator"]["nickname"],
            &value["nickname"],
            &value["nick"],
            &value["creatorName"],
            &value["hostname"],
        ]
        .into_iter()
        .map(text)
        .find(|s| !s.is_empty())
        .unwrap_or_default()
    } else {
        text(first(value, &["desc", "subtitle"]))
    };
    Ok(
        json!({"reference":reference(kind,&id),"name":text(first(value,&["name","title","dirName","dissname","albumName","singerName","singer_name"])),"cover":if kind=="playlist"{image(first(value,&["bigpicUrl","picUrl","logo","cover","picurl"]))}else{artwork(&id,kind)},"subtitle":subtitle,"trackCount":json_number(number(first(value,&["songnum","songNum","song_count","total_song_num","total_song"])).unwrap_or(0.0).max(0.0)),"creator":value["ownerAccount"],"liked":false,"playCount":null,"publishedAt":published_at(first(value,&["publishDate","publish_date","aDate"])),"artists":credits(&value["singer"])}),
    )
}
pub struct Page {
    pub offset: usize,
    pub limit: usize,
    pub scope: String,
}
pub fn page_state(page: &Value, scope: String) -> Result<Page> {
    let n = if page["limit"].is_null() {
        25.0
    } else {
        number(&page["limit"]).unwrap_or(f64::NAN).clamp(1.0, 25.0)
    };
    if !n.is_finite() || n.fract() != 0.0 {
        return Err(Error::new("invalidData", "page.limit"));
    }
    let limit = n as usize;
    let mut offset = 0;
    if let Some(cursor) = page["cursor"].as_str().filter(|s| !s.is_empty()) {
        let cursor: Value = serde_json::from_str(cursor)
            .map_err(|_| Error::new("invalidData", "page.cursor-json"))?;
        let o = cursor["offset"].as_u64();
        if cursor["scope"] != scope
            || cursor["limit"].as_u64() != Some(limit as u64)
            || !o.is_some_and(|o| o <= 100000)
        {
            return Err(Error::new("invalidData", "page.cursor-scope"));
        }
        offset = o.unwrap() as usize;
    } else if !page["cursor"].is_null()
        && page["cursor"] != false
        && page["cursor"] != 0
        && page["cursor"] != ""
    {
        return Err(Error::new("invalidData", "page.cursor-json"));
    }
    Ok(Page {
        offset,
        limit,
        scope,
    })
}
pub fn page_result(items: Vec<Value>, state: &Page, total: Option<f64>) -> Value {
    let more = total
        .map(|t| ((state.offset + items.len()) as f64) < t)
        .unwrap_or(items.len() == state.limit);
    let cursor = if more && !items.is_empty() {
        Some(
            json!({"offset":state.offset+items.len(),"limit":state.limit,"scope":state.scope})
                .to_string(),
        )
    } else {
        None
    };
    json!({"items":items,"nextCursor":cursor})
}
pub fn account_record(profile: &Value, credential: &Value) -> Result<Value> {
    let id = ["str_musicid", "musicid"]
        .into_iter()
        .map(|k| text(&credential[k]))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    if id.is_empty() {
        return Err(Error::new("invalidData", "account.id"));
    }
    let name = [
        &profile["info"]["nick"],
        &profile["info"]["nickname"],
        &profile["info"]["name"],
        &profile["nickname"],
        &credential["nick"],
    ]
    .into_iter()
    .find(|v| !v.is_null())
    .map(text)
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| id.clone());
    let avatar = [
        &profile["info"]["logo"],
        &profile["info"]["avatarUrl"],
        &profile["avatarUrl"],
        &credential["logo"],
    ]
    .into_iter()
    .find(|v| !v.is_null())
    .unwrap_or(&Value::Null);
    Ok(json!({"reference":{"source":"qqmusic","id":id},"displayName":name,"avatar":image(avatar)}))
}

#[cfg(test)]
mod tests;
