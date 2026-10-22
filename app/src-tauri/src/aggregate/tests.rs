use super::store::*;
use crate::{
    model::{Track, TrackSource},
    storage::Store,
    test_support::TestDir,
};
fn song(id: u64) -> Track {
    serde_json::from_value(serde_json::json!({"key":format!("netease:{id}"),"title":format!("歌曲{id}"),"artist":"歌手","album":"专辑","durationMs":1000,"cover":"cover.jpg","source":{"kind":"netease","id":id}})).unwrap()
}
fn link() -> Link {
    Link {
        reference: serde_json::from_value(
            serde_json::json!({"source":"netease","kind":"playlist","id":"100"}),
        )
        .unwrap(),
        account: serde_json::from_value(serde_json::json!({"source":"netease","id":"42"})).unwrap(),
        last_success: 0,
        last_attempt: 0,
        error: None,
    }
}
fn keys(store: &Store, id: &str) -> Vec<String> {
    queue(store, id)
        .unwrap()
        .0
        .into_iter()
        .map(|t| t.key)
        .collect()
}
#[test]
fn migrates_existing_ids_order_and_retains_unavailable_local_metadata() {
    let dir = TestDir::new();
    let path = dir.0.join("library.sqlite");
    let mut a = song(1);
    a.key = "local:a".into();
    a.source = TrackSource::Local {
        path: "missing.mp3".into(),
        netease_id: None,
        cue: None,
    };
    // A genuine pre-migration schema, not data written through the new API.
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE local_playlists(id INTEGER PRIMARY KEY AUTOINCREMENT,name TEXT NOT NULL);CREATE TABLE local_playlist_tracks(playlist_id INTEGER,key TEXT,position INTEGER,PRIMARY KEY(playlist_id,key));CREATE TABLE tracks(key TEXT PRIMARY KEY,source TEXT,title TEXT,artist TEXT,value TEXT);INSERT INTO local_playlists(id,name) VALUES(17,'原歌单');INSERT INTO local_playlist_tracks VALUES(17,'local:a',8);").unwrap();
    db.execute(
        "INSERT INTO tracks VALUES(?1,'local',?2,?3,?4)",
        rusqlite::params![a.key, a.title, a.artist, serde_json::to_string(&a).unwrap()],
    )
    .unwrap();
    drop(db);
    let store = Store::open(&path).unwrap();
    assert_eq!(playlist(&store, "17").unwrap().name, "原歌单");
    assert!(!tracks(&store, "17", 0, false).unwrap().items[0].remote);
    store
        .0
        .lock()
        .unwrap()
        .execute_batch(
            "DELETE FROM tracks WHERE key='local:a';DELETE FROM local_origins WHERE key='local:a';",
        )
        .unwrap();
    let rows = tracks(&store, "17", 0, false).unwrap();
    assert_eq!(rows.items[0].track.title, a.title);
    assert!(rows.items[0].unavailable.is_some());
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(playlist(&store, "17").unwrap().track_count, 1);
}
#[test]
fn sync_always_follows_original_members_and_preserves_only_other_sources() {
    let dir = TestDir::new();
    let path = dir.0.join("db");
    let store = Store::open(&path).unwrap();
    let l = link();
    let id = apply(&store, None, &l, "原名", &[song(1), song(2), song(3)]).unwrap();
    let mut extra = song(8);
    extra.key = "local:extra".into();
    extra.source = TrackSource::Local {
        path: "extra.mp3".into(),
        netease_id: None,
        cue: None,
    };
    store.save_tracks(&[extra.clone(), song(9)]).unwrap();
    write(
        &store,
        &Write::Add {
            id: id.clone(),
            keys: vec![extra.key.clone(), song(9).key],
        },
    )
    .unwrap();
    // Mixed removal must roll back even the preceding extra member deletion.
    assert!(write(
        &store,
        &Write::Remove {
            id: id.clone(),
            keys: vec![extra.key.clone(), song(1).key]
        }
    )
    .is_err());
    assert_eq!(playlist(&store, &id).unwrap().track_count, 5);
    write(
        &store,
        &Write::Reorder {
            id: id.clone(),
            keys: vec![
                song(3).key,
                song(2).key,
                song(1).key,
                extra.key.clone(),
                song(9).key,
            ],
        },
    )
    .unwrap();
    // Simulate flags left by the previous retention/exclusion feature.
    store
        .0
        .lock()
        .unwrap()
        .execute(
            "UPDATE local_playlist_tracks SET manual=1,excluded=1 WHERE playlist_id=?1 AND key=?2",
            rusqlite::params![id, song(1).key],
        )
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(tracks(&store, &id, 0, true).unwrap().items.is_empty());
    assert_eq!(playlist(&store, &id).unwrap().track_count, 5);
    apply(
        &store,
        Some(&id),
        &l,
        "远端改名",
        &[song(1), song(3), song(4)],
    )
    .unwrap();
    // Local unavailable extra remains in members even though playback skips it.
    let rows = tracks(&store, &id, 0, false).unwrap();
    assert_eq!(
        rows.items
            .iter()
            .map(|m| m.track.key.clone())
            .collect::<Vec<_>>(),
        vec![song(3).key, song(1).key, extra.key.clone(), song(4).key]
    );
    assert_eq!(playlist(&store, &id).unwrap().name, "远端改名");
    apply(&store, Some(&id), &l, "空的原歌单", &[]).unwrap();
    assert_eq!(playlist(&store, &id).unwrap().track_count, 1);
    write(
        &store,
        &Write::Remove {
            id: id.clone(),
            keys: vec![extra.key],
        },
    )
    .unwrap();
    assert_eq!(playlist(&store, &id).unwrap().track_count, 0);
    apply(&store, Some(&id), &l, "恢复", &[song(1)]).unwrap();
    write(&store, &Write::Unlink { id: id.clone() }).unwrap();
    assert!(apply(&store, Some(&id), &l, "迟到", &[song(5)]).is_err());
    write(
        &store,
        &Write::Remove {
            id: id.clone(),
            keys: vec![song(1).key],
        },
    )
    .unwrap();
    write(
        &store,
        &Write::Rename {
            id: id.clone(),
            name: "独立".into(),
        },
    )
    .unwrap();
    write(&store, &Write::Delete { id }).unwrap();
    assert!(store.track(&song(1).key).is_ok());
}
#[test]
fn batch_add_and_sync_capacity_fail_atomically_and_cross_platform_ids_remain_distinct() {
    let dir = TestDir::new();
    let store = Store::open(&dir.0.join("db")).unwrap();
    store.save_tracks(&[song(1)]).unwrap();
    let id = write(
        &store,
        &Write::Create {
            name: "集合".into(),
            keys: vec![song(1).key],
        },
    )
    .unwrap();
    assert!(write(
        &store,
        &Write::Add {
            id: id.clone(),
            keys: vec![song(1).key, "missing".into()]
        }
    )
    .is_err());
    assert_eq!(keys(&store, &id), vec![song(1).key]);
    assert!(write(
        &store,
        &Write::Create {
            name: "失败".into(),
            keys: vec!["missing".into()]
        }
    )
    .is_err());
    assert_eq!(playlists(&store).unwrap().len(), 1);
    let start = std::time::Instant::now();
    let large = (1..=10000).map(song).collect::<Vec<_>>();
    let id2 = apply(&store, None, &link(), "一万首", &large).unwrap();
    let load = std::time::Instant::now();
    let (queue, skipped) = queue(&store, &id2).unwrap();
    assert_eq!(queue.len(), 10000);
    assert_eq!(skipped, 0);
    eprintln!(
        "aggregate benchmark debug windows: save10000={:?} read10000={:?}",
        load - start,
        load.elapsed()
    );
    assert!(apply(
        &store,
        Some(&id2),
        &link(),
        "超限",
        &(1..=10001).map(song).collect::<Vec<_>>()
    )
    .is_err());
    assert_eq!(playlist(&store, &id2).unwrap().name, "一万首");
    store.save_tracks(&[song(10001)]).unwrap();
    assert!(write(
        &store,
        &Write::Add {
            id: id2.clone(),
            keys: vec![song(10001).key]
        }
    )
    .is_err());
    assert_eq!(playlist(&store, &id2).unwrap().track_count, 10000);
    // Identity is the actual source + ID; local and online values never collapse by title.
    let mut local = song(1);
    local.key = "local:1".into();
    local.source = TrackSource::Local {
        path: "same.mp3".into(),
        netease_id: None,
        cue: None,
    };
    store.save_tracks(&[local.clone()]).unwrap();
    write(
        &store,
        &Write::Add {
            id: id.clone(),
            keys: vec![local.key.clone(), song(1).key],
        },
    )
    .unwrap();
    assert_eq!(playlist(&store, &id).unwrap().track_count, 2);
    assert!(write(
        &store,
        &Write::Reorder {
            id: id.clone(),
            keys: vec![song(1).key]
        }
    )
    .is_err());
}

#[test]
fn invalidated_commit_rolls_back_first_derivation_and_existing_changes() {
    use std::cell::Cell;
    let dir = TestDir::new();
    let store = Store::open(&dir.0.join("db")).unwrap();
    let calls = Cell::new(0);
    let result = apply_checked(&store, None, &link(), "迟到", &[song(1)], || {
        calls.set(calls.get() + 1);
        if calls.get() >= 3 {
            Err("已取消".into())
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    assert!(playlists(&store).unwrap().is_empty());
    let id = apply(&store, None, &link(), "保留", &[song(1)]).unwrap();
    let calls = Cell::new(0);
    assert!(apply_checked(
        &store,
        Some(&id),
        &link(),
        "不应改名",
        &[song(2)],
        || {
            calls.set(calls.get() + 1);
            if calls.get() >= 3 {
                Err("账号已变化".into())
            } else {
                Ok(())
            }
        }
    )
    .is_err());
    assert_eq!(keys(&store, &id), vec![song(1).key]);
    assert_eq!(playlist(&store, &id).unwrap().name, "保留");
}

#[test]
fn playlists_limit_is_atomic_and_actual_online_sources_do_not_merge() {
    let dir = TestDir::new();
    let store = Store::open(&dir.0.join("db")).unwrap();
    let mut tracks = vec![song(1)];
    tracks.push(Track::from_music(serde_json::from_value(serde_json::json!({"reference":{"source":"qqmusic","kind":"track","id":"1"},"title":"歌曲1","aliases":[],"artist":"歌手","artists":[],"album":"专辑","durationMs":1000,"cover":"","associations":[]})).unwrap()).unwrap());
    store.save_tracks(&tracks).unwrap();
    let id = write(
        &store,
        &Write::Create {
            name: "双平台".into(),
            keys: tracks.iter().map(|t| t.key.clone()).collect(),
        },
    )
    .unwrap();
    assert_eq!(keys(&store, &id).len(), 2);
    for _ in 1..200 {
        write(
            &store,
            &Write::Create {
                name: "歌单".into(),
                keys: vec![],
            },
        )
        .unwrap();
    }
    assert!(write(
        &store,
        &Write::Create {
            name: "超限".into(),
            keys: vec![]
        }
    )
    .is_err());
    assert!(apply(&store, None, &link(), "超限派生", &[song(2)]).is_err());
    assert_eq!(playlists(&store).unwrap().len(), 200);
}
