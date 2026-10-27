use super::*;

#[test]
fn playlists_retain_names_and_creator_labels() {
    let created = entity(
        &json!({"tid":123,"dirName":"我的自建歌单","desc":"QQ音乐","creator":{"name":"创建者"}}),
        "playlist",
    )
    .unwrap();
    assert_eq!(created["name"], "我的自建歌单");
    assert_eq!(created["subtitle"], "创建者");
    let collected = entity(
        &json!({"dissid":"456","dissname":"收藏歌单","desc":"QQ音乐","nickname":"其他作者"}),
        "playlist",
    )
    .unwrap();
    assert_eq!(collected["name"], "收藏歌单");
    assert_eq!(collected["subtitle"], "其他作者");
    assert_eq!(
        entity(
            &json!({"tid":123,"creator":{"name":""},"nickname":"","creatorName":"Miaoyww"}),
            "playlist"
        )
        .unwrap()["subtitle"],
        "Miaoyww"
    );
}

#[test]
fn track_identity_is_song_mid_and_preserves_lyric_identifiers() {
    let value=track(&json!({"id":42,"mid":"SongMID","file":{"media_mid":"AudioMID"},"title":"曲目","interval":123,"singer":[{"name":"歌手","mid":"ArtistMID"}],"album":{"name":"专辑","mid":"AlbumMID"}}),None).unwrap();
    assert_eq!(value["reference"], reference("track", "SongMID"));
    assert_eq!(
        value["lyricIdentifiers"],
        json!([{"kind":"song-mid","id":"SongMID"},{"kind":"song-id","id":"42"}])
    );
    assert_eq!(value["durationMs"].as_f64(), Some(123000.0));
    assert_eq!(value["artists"][0]["reference"]["id"], "ArtistMID");
    assert_eq!(value["albumReference"]["id"], "AlbumMID");
    assert!(track(&json!({"id":42,"title":"缺少 mid"}), None).is_err());
    let missing = track(&json!({"id":42,"title":"缺少 mid"}), Some("notFound")).unwrap();
    assert_eq!(missing["reference"]["id"], "id:42");
    assert_eq!(missing["unavailableReason"], "notFound");
}

#[test]
fn cursors_reject_query_page_size_and_offset_changes() {
    let state = page_state(&json!({"limit":25}), "query-a".into()).unwrap();
    let result = page_result(vec![json!({}); 25], &state, Some(26.0));
    let page = json!({"cursor":result["nextCursor"],"limit":25});
    assert_eq!(page_state(&page, "query-a".into()).unwrap().offset, 25);
    assert!(page_state(&page, "query-b".into()).is_err());
    assert!(page_state(
        &json!({"cursor":result["nextCursor"],"limit":10}),
        "query-a".into()
    )
    .is_err());
    assert!(page_state(
        &json!({"cursor":json!({"scope":"query-a","limit":25,"offset":-1}).to_string()}),
        "query-a".into()
    )
    .is_err());
    assert!(page_result(vec![], &state, None)["nextCursor"].is_null());
    assert!(check_reference(
        &json!({"source":"other","kind":"track","id":"SongMID"}),
        "track"
    )
    .is_err());
}

#[test]
fn account_identity_uses_real_string_music_id_without_exposing_secrets() {
    let record = account_record(
        &json!({"info":{"nick":"测试账号"}}),
        &json!({"musicid":0,"str_musicid":"wx-real-id","musickey":"secret"}),
    )
    .unwrap();
    assert_eq!(record["reference"]["id"], "wx-real-id");
    assert_eq!(record["displayName"], "测试账号");
    assert!(!record.to_string().contains("secret"));
    let missing = account_record(&json!({"info":{}}), &json!({"musicid":42})).unwrap();
    assert_eq!(missing["avatar"], image(&json!("")));
}
