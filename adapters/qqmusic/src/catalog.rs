use crate::{auth::Auth, model::*, Error, Host, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn items(value: &Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}
fn total(value: &Value) -> Option<f64> {
    value.as_number().and_then(|v| v.as_f64())
}
fn merge(left: &Value, right: &Value) -> Value {
    let mut result = left.as_object().cloned().unwrap_or_default();
    if let Some(values) = right.as_object() {
        result.extend(values.clone());
    }
    Value::Object(result)
}
fn send(host: &impl Host, body: Value) -> Result<Value> {
    let response = host.http(
        "POST",
        "https://u.y.qq.com/cgi-bin/musicu.fcg",
        vec![
            ("Content-Type".into(), "application/json".into()),
            ("Cookie".into(), "tmeLoginType=-1;".into()),
            ("User-Agent".into(), "okhttp/3.14.9".into()),
        ],
        body.to_string().into_bytes(),
        10000,
    )?;
    if !(200..300).contains(&response.status) {
        return Err(Error::new(
            if response.status == 429 {
                "rateLimited"
            } else {
                "network"
            },
            "catalog.validation",
        ));
    }
    serde_json::from_slice(&response.body).map_err(|_| Error::new("invalidData", "rpc.json"))
}
pub fn rpc(
    host: &impl Host,
    auth: &mut Auth,
    module: &str,
    method: &str,
    param: Value,
    authenticated: bool,
) -> Result<Value> {
    if authenticated {
        return auth.rpc(host, module, method, param);
    }
    let mut body = send(
        host,
        json!({"comm":{"ct":11,"cv":"1003006","v":"1003006","tmeAppID":"qqmusiclight","uid":"0","udid":"0"},"request":{"module":module,"method":method,"param":&param}}),
    )?;
    let desktop = module == "music.search.SearchCgiService"
        && method == "DoSearchForQQMusicLite"
        && (body["code"] == 2001 || body["code"] == 0 && body["request"]["code"] == 2001);
    if desktop {
        host.diagnostic("search.retry-desktop", "UpstreamRejected", "upstream-2001");
        body = send(
            host,
            json!({"request":{"module":module,"method":"DoSearchForQQMusicDesktop","param":{"query":param["query"],"search_type":param["search_type"],"num_per_page":param["num_per_page"],"page_num":param["page_num"]}}}),
        )?;
    }
    if method == "get_song_detail_yqq"
        && body["code"] == 0
        && body["request"]["code"] == 500
        && text(&body["request"]["data"]["track_info"]["mid"]).is_empty()
    {
        return Err(Error::new("notFound", "track.detail-not-found"));
    }
    if body["code"] != 0 || body["request"]["code"] != 0 {
        let code = if body["request"]["code"].is_null() {
            &body["code"]
        } else {
            &body["request"]["code"]
        };
        let stage = code
            .as_i64()
            .filter(|n| n.unsigned_abs() <= 9_007_199_254_740_991)
            .map(|n| format!("rpc.upstream-{n}"))
            .unwrap_or_else(|| "rpc.upstream".into());
        return Err(Error::new("sourceUnavailable", &stage));
    }
    let mut data = body["request"]["data"].clone();
    if desktop {
        let keys = match param["search_type"].as_u64() {
            Some(0) => ("song", "item_song"),
            Some(1) => ("singer", "singer"),
            Some(2) => ("album", "item_album"),
            Some(3) => ("songlist", "item_songlist"),
            _ => return Err(Error::new("invalidData", "search.desktop-data")),
        };
        let entries = data["body"][keys.0]["list"]
            .as_array()
            .ok_or_else(|| Error::new("invalidData", "search.desktop-data"))?;
        let values: Vec<Value> = entries
            .iter()
            .map(|v| {
                let mut fields = v
                    .as_object()
                    .cloned()
                    .ok_or_else(|| Error::new("invalidData", "search.desktop-data"))?;
                fields.insert(
                    "albumMid".into(),
                    first(v, &["albumMID", "albumMid"]).clone(),
                );
                fields.insert("picurl".into(), first(v, &["imgurl", "picurl"]).clone());
                Ok(Value::Object(fields))
            })
            .collect::<Result<Vec<_>>>()?;
        data["body"] = json!({(keys.1):values});
    }
    Ok(data)
}
fn album(host: &impl Host, id: &str) -> Result<Value> {
    let url=format!("https://c.y.qq.com/v8/fcg-bin/fcg_v8_album_info_cp.fcg?format=json&outCharset=utf-8&albummid={id}");
    let response = host.http(
        "GET",
        &url,
        vec![("Referer".into(), "https://y.qq.com/".into())],
        vec![],
        10000,
    )?;
    if !(200..300).contains(&response.status) {
        return Err(Error::new("network", "catalog.validation"));
    }
    let value: Value = serde_json::from_slice(&response.body)
        .map_err(|_| Error::new("invalidData", "album.json"))?;
    if value["code"] != 0 || value["data"].is_null() {
        return Err(Error::new("notFound", "catalog.validation"));
    }
    Ok(value["data"].clone())
}
fn collections(
    host: &impl Host,
    auth: &mut Auth,
    session: &Value,
    filter: &str,
) -> Result<Vec<Value>> {
    let credential = &session["credential"];
    let uin = if text(&credential["str_musicid"]).is_empty() {
        text(&credential["musicid"])
    } else {
        text(&credential["str_musicid"])
    };
    let created = rpc(
        host,
        auth,
        "music.musicasset.PlaylistBaseRead",
        "GetPlaylistByUin",
        json!({"uin":uin}),
        true,
    )?;
    let mut owner = text(&credential["nick"]);
    if owner.is_empty() {
        owner = text(&account_record(&auth.profile(host)?, credential)?["displayName"]);
    }
    let owner_ref = account_record(&json!({}), credential)?["reference"].clone();
    let mut values: Vec<Value> = items(&created["v_playlist"])
        .into_iter()
        .filter(|v| number(first(v, &["dirId", "dirid"])) != Some(201.0))
        .map(|v| {
            let mut fields = v
                .as_object()
                .cloned()
                .ok_or_else(|| Error::new("invalidData", "library.playlist-data"))?;
            if text(&v["creatorName"]).is_empty() {
                fields.insert("creatorName".into(), json!(owner));
            }
            fields.insert("ownerAccount".into(), owner_ref.clone());
            Ok(Value::Object(fields))
        })
        .collect::<Result<Vec<_>>>()?;
    if filter == "created" {
        return Ok(values);
    }
    let encrypted = text(&credential["encryptUin"]);
    let mut favorites = Vec::new();
    if !encrypted.is_empty() {
        let mut offset = 0;
        while offset < 1000 {
            let data = rpc(
                host,
                auth,
                "music.musicasset.PlaylistFavRead",
                "CgiGetPlaylistFavInfo",
                json!({"uin":encrypted,"offset":offset,"size":100}),
                true,
            )?;
            let page = items(&data["v_list"]);
            let count = page.len();
            favorites.extend(page);
            offset += count;
            if count == 0 || !(data["hasmore"] == true || data["hasmore"] == 1) {
                break;
            }
        }
    }
    if filter == "collected" {
        return Ok(favorites);
    }
    values.extend(favorites);
    let mut seen = HashSet::new();
    Ok(values
        .into_iter()
        .filter(|v| {
            let id = text(first(v, &["tid", "dissid", "disstid"]));
            !id.is_empty() && seen.insert(id)
        })
        .collect())
}
fn song(host: &impl Host, auth: &mut Auth, id: &str) -> Result<Value> {
    let numeric = id.strip_prefix("id:");
    let number = if let Some(id) = numeric {
        Some(positive_id(&json!(id)).ok_or_else(|| Error::new("invalidData", "track.numeric-id"))?)
    } else {
        None
    };
    let data = rpc(
        host,
        auth,
        "music.pf_song_detail_svr",
        "get_song_detail_yqq",
        json!({"song_type":0,"song_mid":if numeric.is_some(){""}else{id},"song_id":number.map(|v|json!(v)).unwrap_or(json!(""))}),
        false,
    )?;
    let value = &data["track_info"];
    if value.is_null()
        || number
            .map(|n| positive_id(&value["id"]) != Some(n))
            .unwrap_or_else(|| value["mid"] != id)
    {
        return Err(Error::new("notFound", "track.detail-identity"));
    }
    Ok(value.clone())
}
pub fn read_track(host: &impl Host, auth: &mut Auth, request: &Value) -> Result<Value> {
    let id = check_reference(request, "track")?;
    let mut result = track(&song(host, auth, id)?, None)?;
    result["reference"] = request.clone();
    Ok(result)
}
fn tracks_response(
    host: &impl Host,
    auth: &mut Auth,
    values: &[Value],
    state: &Page,
    total: Option<f64>,
    description: Value,
) -> Result<Value> {
    let mut repaired: HashMap<u64, Option<Value>> = HashMap::new();
    let mut tracks = Vec::new();
    for value in values.iter().take(state.limit) {
        match track(value, None) {
            Ok(value) => tracks.push(value),
            Err(error) => {
                if error.stage != "track.mid" {
                    return Err(error);
                }
                let original = original_song(value);
                let Some(id) = positive_id(first(original, &["id", "songid"])) else {
                    return Err(error);
                };
                if !repaired.contains_key(&id) {
                    let result =
                        song(host, auth, &format!("id:{id}")).and_then(|v| track(&v, None));
                    let result = match result {
                        Ok(v) => Some(v),
                        Err(e) if e.code == "notFound" || e.stage == "track.mid" => None,
                        Err(e) => return Err(e),
                    };
                    repaired.insert(id, result);
                }
                tracks.push(match &repaired[&id] {
                    Some(v) => v.clone(),
                    None => track(original, Some("notFound"))?,
                });
            }
        }
    }
    if tracks.iter().any(|v| !v["unavailableReason"].is_null()) {
        host.diagnostic("track.mid-unavailable", "MissingMid", "notFound");
    }
    let mut data = page_result(tracks, state, total);
    data["total"] = total.map(json_number).unwrap_or(Value::Null);
    data["description"] = description;
    Ok(json!({"type":"tracks","data":data}))
}
struct Playlist {
    data: Value,
    tracks: Vec<Value>,
    total: Option<f64>,
    description: Value,
}
fn playlist(host: &impl Host, auth: &mut Auth, id: &str, state: &Page) -> Result<Playlist> {
    let session = auth.session(host)?;
    if id == "liked" {
        let data = auth.liked_songs(host, state.offset, state.limit)?;
        return Ok(Playlist {
            tracks: items(&data["songlist"]),
            total: total(&data["total_song_num"]),
            description: Value::Null,
            data: merge(&data, &json!({"disstid":id,"dissname":"我喜欢"})),
        });
    }
    let id_num = id
        .parse::<u64>()
        .ok()
        .filter(|n| *n <= 9_007_199_254_740_991);
    if id.is_empty() || !id.bytes().all(|c| c.is_ascii_digit()) || id_num.is_none() {
        return Err(Error::new("invalidData", "playlist.numeric-id"));
    }
    if let Some(session) = &session {
        let own = collections(host, auth, session, "created")?
            .into_iter()
            .find(|v| text(first(v, &["tid", "disstid", "dissid"])) == id);
        if let Some(own) = own {
            let data=auth.owned_playlist_songs(host,json!({"disstid":id_num,"dirid":json_number(number(first(&own,&["dirId","dirid"])).unwrap_or(0.0)),"offset":state.offset,"limit":state.limit}))?;
            return Ok(Playlist {
                tracks: items(&data["songlist"]),
                total: total(&data["total_song_num"]),
                description: data["desc"].clone(),
                data: merge(&own, &data),
            });
        }
    }
    let data = rpc(
        host,
        auth,
        "music.srfDissInfo.aiDissInfo",
        "uniform_get_Dissinfo",
        json!({"disstid":id_num,"userinfo":1,"tag":1,"orderlist":1,"song_begin":state.offset,"song_num":state.limit,"onlysonglist":0,"enc_host_uin":""}),
        session.is_some(),
    )?;
    if !data["code"].is_null() && data["code"] != 0 {
        return Err(Error::new("sourceUnavailable", "playlist.upstream-result"));
    }
    let mut tracks = items(&data["songlist"]);
    if tracks.len() > state.limit {
        tracks = tracks
            .into_iter()
            .skip(state.offset)
            .take(state.limit)
            .collect();
    }
    let total = number(&data["total_song_num"]);
    let data = merge(&data["dirinfo"], &data);
    let description = data["desc"].clone();
    Ok(Playlist {
        data,
        tracks,
        total,
        description,
    })
}
fn entities_response(
    values: &[Value],
    kind: &str,
    state: &Page,
    count: Option<f64>,
) -> Result<Value> {
    let items = values
        .iter()
        .take(state.limit)
        .map(|v| entity(v, kind))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"type":"entities","data":page_result(items,state,count)}))
}
pub fn business(host: &impl Host, auth: &mut Auth, request: &Value) -> Result<Value> {
    let operation = request["operation"].as_str().unwrap_or("");
    let kind = request["kind"].as_str().unwrap_or("");
    if operation == "search" {
        let (search_type, key) = match kind {
            "track" => (0, "item_song"),
            "artist" => (1, "singer"),
            "album" => (2, "item_album"),
            "playlist" => (3, "item_songlist"),
            _ => return Err(Error::new("invalidData", "catalog.validation")),
        };
        let state = page_state(
            &request["page"],
            json!(["search", kind, request["keyword"]]).to_string(),
        )?;
        let mut random = host.random(16)?;
        if random.len() != 16 {
            return Err(Error::new("invalidData", "search.random"));
        }
        random[6] = (random[6] & 15) | 64;
        random[8] = (random[8] & 63) | 128;
        let search_id = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let data = rpc(
            host,
            auth,
            "music.search.SearchCgiService",
            "DoSearchForQQMusicLite",
            json!({"query":request["keyword"],"search_type":search_type,"num_per_page":state.limit,"page_num":state.offset/state.limit+1,"highlight":0,"nqc_flag":0,"page_id":1,"grp":1,"remoteplace":"search.android.keyboard","search_id":search_id}),
            false,
        )?;
        let values = items(&data["body"][key]);
        let count = total(&data["meta"]["sum"]);
        return if kind == "track" {
            tracks_response(host, auth, &values, &state, count, Value::Null)
        } else {
            entities_response(&values, kind, &state, count)
        };
    }
    if operation == "trackInformation" {
        let v = read_track(host, auth, &request["track"])?;
        return Ok(
            json!({"type":"information","data":{"artists":v["artists"],"albumReference":v["albumReference"],"publishedAt":null}}),
        );
    }
    let entity_kind = request["entity"]["kind"].as_str().unwrap_or("");
    if operation == "tracks" && entity_kind == "playlist" {
        let id = check_reference(&request["entity"], "playlist")?;
        let state = page_state(&request["page"], json!(["playlist", id]).to_string())?;
        let p = playlist(host, auth, id, &state)?;
        return tracks_response(host, auth, &p.tracks, &state, p.total, p.description);
    }
    if (operation == "detail" || operation == "tracks") && entity_kind == "album" {
        let id = check_reference(&request["entity"], "album")?;
        let data = album(host, id)?;
        if operation == "detail" {
            return Ok(
                json!({"type":"detail","data":{"item":entity(&data,"album")?,"description":data["desc"],"albumCount":null}}),
            );
        }
        let state = page_state(&request["page"], json!(["album", id]).to_string())?;
        let values = items(&data["list"]);
        let count = values.len();
        let values = values
            .into_iter()
            .skip(state.offset)
            .take(state.limit)
            .collect::<Vec<_>>();
        return tracks_response(host, auth, &values, &state, Some(count as f64), Value::Null);
    }
    if operation == "detail" && entity_kind == "artist"
        || operation == "artistTracks"
        || operation == "artistAlbums"
    {
        let r = if request["entity"].is_null() {
            &request["artist"]
        } else {
            &request["entity"]
        };
        let id = check_reference(r, "artist")?;
        let state = page_state(&request["page"], json!([operation, id]).to_string())?;
        if operation == "artistAlbums" {
            let data = rpc(
                host,
                auth,
                "music.musichallAlbum.AlbumListServer",
                "GetAlbumList",
                json!({"sort":5,"singermid":id,"begin":state.offset,"num":state.limit}),
                false,
            )?;
            return entities_response(
                &items(&data["albumList"]),
                "album",
                &state,
                total(&data["total"]),
            );
        }
        let data = rpc(
            host,
            auth,
            "music.web_singer_info_svr",
            "get_singer_detail_info",
            json!({"sort":5,"singermid":id,"sin":state.offset,"num":if operation=="detail"{1}else{state.limit}}),
            false,
        )?;
        if operation == "detail" {
            return Ok(
                json!({"type":"detail","data":{"item":entity(&merge(&data["singer_info"],&json!({"mid":id,"total_song":data["total_song"]})),"artist")?,"description":data["singer_brief"],"albumCount":data["total_album"]}}),
            );
        }
        return tracks_response(
            host,
            auth,
            &items(&data["songlist"]),
            &state,
            total(&data["total_song"]),
            Value::Null,
        );
    }
    if operation == "detail" && entity_kind == "playlist" {
        let id = check_reference(&request["entity"], "playlist")?;
        let state = page_state(&json!({"limit":1}), json!(["playlist", id]).to_string())?;
        let p = playlist(host, auth, id, &state)?;
        return Ok(
            json!({"type":"detail","data":{"item":entity(&merge(&p.data,&json!({"disstid":id})),"playlist")?,"description":p.data["desc"],"albumCount":null}}),
        );
    }
    if operation == "librarySummary" {
        let session = auth
            .session(host)?
            .ok_or_else(|| Error::new("unauthenticated", "catalog.validation"))?;
        let profile = auth.profile(host)?;
        let state = Page {
            limit: 25,
            offset: 0,
            scope: String::new(),
        };
        let liked = auth.liked_songs(host, 0, 25).and_then(|data| {
            let response = tracks_response(
                host,
                auth,
                &items(&data["songlist"]),
                &state,
                total(&data["total_song_num"]),
                Value::Null,
            )?;
            let playlist = entity(
                &json!({"disstid":"liked","dissname":"我喜欢","songnum":data["total_song_num"]}),
                "playlist",
            )?;
            Ok((response["data"]["items"].clone(), playlist))
        });
        let (tracks, playlist, error) = match liked {
            Ok((t, p)) => (t, p, Value::Null),
            Err(_) => (
                json!([]),
                Value::Null,
                json!({"code":"sourceUnavailable","retryAfterMs":null}),
            ),
        };
        return Ok(
            json!({"type":"summary","data":{"account":account_record(&profile,&session["credential"] )?,"likedPlaylist":playlist,"likedTracks":tracks,"likedError":error}}),
        );
    }
    if operation == "favoritesPage" {
        let state = page_state(&request["page"], json!(["playlist", "liked"]).to_string())?;
        let p = playlist(host, auth, "liked", &state)?;
        let response = tracks_response(host, auth, &p.tracks, &state, p.total, p.description)?;
        let refs = items(&response["data"]["items"])
            .into_iter()
            .map(|v| v["reference"].clone())
            .collect::<Vec<_>>();
        return Ok(
            json!({"type":"favoritesPage","data":{"items":refs,"nextCursor":response["data"]["nextCursor"]}}),
        );
    }
    if operation == "libraryCollections" {
        let session = auth
            .session(host)?
            .ok_or_else(|| Error::new("unauthenticated", "catalog.validation"))?;
        let state = page_state(
            &request["page"],
            json!(["library", kind, request["filter"]]).to_string(),
        )?;
        if kind == "album" {
            let data = auth.user_albums(host, state.offset, state.limit)?;
            let values = if data["albumlist"].is_null() {
                items(&data["list"])
            } else {
                items(&data["albumlist"])
            };
            return entities_response(&values, "album", &state, total(&data["totalalbum"]));
        }
        if kind == "playlist" {
            let values = collections(
                host,
                auth,
                &session,
                request["filter"].as_str().unwrap_or(""),
            )?;
            let count = values.len();
            let selected = values
                .into_iter()
                .skip(state.offset)
                .take(state.limit)
                .collect::<Vec<_>>();
            return entities_response(&selected, "playlist", &state, Some(count as f64));
        }
    }
    Err(Error::new("unsupported", "catalog.validation"))
}

pub fn resolve_playback(host: &impl Host, auth: &mut Auth, request: &Value) -> Result<Value> {
    let identifier = check_reference(&request["track"], "track")?;
    if auth.session(host)?.is_none() {
        return Err(Error::new("unauthenticated", "catalog.validation"));
    }
    let value = song(host, auth, identifier)?;
    let mid = ["mid", "songmid"]
        .into_iter()
        .map(|k| text(&value[k]))
        .find(|s| !s.is_empty())
        .ok_or_else(|| Error::new("notFound", "playback.track-mid"))?;
    let order = ["hires", "lossless", "exhigh", "higher", "standard"];
    let preferred = request["preferredQuality"].as_str().unwrap_or("");
    let start = order
        .iter()
        .position(|q| *q == preferred)
        .ok_or_else(|| Error::new("invalidData", "catalog.validation"))?;
    let downgrade = request["allowDowngrade"] == true;
    if !downgrade && !["standard", "exhigh", "lossless"].contains(&preferred) {
        return Err(Error::new("unsupported", "catalog.validation"));
    }
    let mut rejected = Error::new("permissionDenied", "playback.file-size");
    for quality in order
        .iter()
        .skip(start)
        .take(if downgrade { order.len() } else { 1 })
    {
        let (prefix, extension, size_key) = match *quality {
            "standard" => ("M500", ".mp3", "size_128mp3"),
            "exhigh" => ("M800", ".mp3", "size_320mp3"),
            "lossless" => ("F000", ".flac", "size_flac"),
            _ => continue,
        };
        let Some(size) = positive_id(&value["file"][size_key]) else {
            continue;
        };
        let media = text(&value["file"]["media_mid"]);
        if !alphanumeric(&media) {
            return Err(Error::new("invalidData", "catalog.validation"));
        }
        let result = auth.playback(host, &mid, quality, &media)?;
        let url = playback_url(host, &result, &mid, prefix)?;
        let Some(url) = url else {
            rejected = Error::new("sourceUnavailable", "playback.url-missing");
            continue;
        };
        let parsed =
            url::Url::parse(&url).map_err(|_| Error::new("invalidData", "playback.url"))?;
        let filename = parsed.path().rsplit('/').next().unwrap_or("");
        if !filename.starts_with(prefix) || !filename.ends_with(extension) {
            rejected = Error::new("invalidData", "playback.filename");
            continue;
        }
        let response = host.http(
            "HEAD",
            &url,
            vec![("Accept-Encoding".into(), "identity".into())],
            vec![],
            10000,
        )?;
        if response.status != 200 {
            rejected = Error::new(
                match response.status {
                    401 => "unauthenticated",
                    403 => "permissionDenied",
                    404 => "notFound",
                    429 => "rateLimited",
                    _ => "sourceUnavailable",
                },
                &format!("playback.http-{}", response.status),
            );
            continue;
        }
        let actual = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.trim().parse::<u64>().ok());
        if actual != Some(size) {
            rejected = Error::new("invalidData", "playback.content-length");
            continue;
        }
        return Ok(
            json!({"metadata":{"actualQuality":quality,"extent":{"kind":"full"},"expiresAtMs":null},"url":url,"headers":[]}),
        );
    }
    Err(rejected)
}
fn playback_url(
    host: &impl Host,
    result: &Value,
    mid: &str,
    prefix: &str,
) -> Result<Option<String>> {
    if let Some(url) = result[mid]["url"].as_str().filter(|s| !s.is_empty()) {
        return Ok(Some(url.into()));
    }
    if result[mid].is_object() {
        return Ok(None);
    }
    let data = if !result["req_0"]["data"].is_null() {
        &result["req_0"]["data"]
    } else {
        result
    };
    let entries = items(&data["midurlinfo"]);
    let code = |v: &Value| {
        let valid = v.is_number()
            || v.as_str().is_some_and(|s| {
                let s = s.strip_prefix('-').unwrap_or(s);
                !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
            });
        if !valid {
            return "notProvided".into();
        }
        number(v)
            .filter(|n| n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0)
            .map(|n| format!("upstream-{n:.0}"))
            .unwrap_or_else(|| "notProvided".into())
    };
    if entries.is_empty() {
        host.diagnostic(
            &format!("playback.vkey-entries.{prefix}"),
            "EntriesMissing",
            &code(if result["req_0"]["code"].is_null() {
                &result["code"]
            } else {
                &result["req_0"]["code"]
            }),
        );
    }
    for entry in entries.iter().take(8) {
        if text(&entry["purl"]).is_empty() {
            host.diagnostic(
                &format!("playback.vkey-empty.{prefix}"),
                "EmptyPurl",
                &code(&entry["result"]),
            );
        } else if text(&entry["songmid"]).trim() != mid {
            host.diagnostic(
                &format!("playback.vkey-identity.{prefix}"),
                "UnmatchedSong",
                &code(&entry["result"]),
            );
        }
    }
    let entry = entries
        .iter()
        .find(|v| text(&v["songmid"]).trim() == mid && !text(&v["purl"]).is_empty());
    let Some(entry) = entry else { return Ok(None) };
    let purl = text(&entry["purl"]);
    if purl.starts_with("https://") || purl.starts_with("http://") {
        return Ok(Some(purl));
    }
    let base = data["sip"]
        .as_array()
        .and_then(|v| v.first())
        .and_then(Value::as_str)
        .unwrap_or("");
    if base.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "{}{}",
        base.trim_end_matches('/'),
        format!("/{}", purl.trim_start_matches('/'))
    )))
}
