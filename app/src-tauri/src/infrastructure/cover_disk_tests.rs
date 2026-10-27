use super::*;

const A: &str = "https://images.example/one.png?public=1";
const B: &str = "https://images.example/two.png";
const C: &str = "https://images.example/three.png";

fn put(cache: &DiskCache, url: &str, bytes: &[u8]) -> Asset {
    cache
        .publish(url, bytes, "image/png", cache.generation().unwrap())
        .unwrap()
}

#[test]
fn restart_reuses_original_bytes_and_index_has_only_url_hashes() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("online-covers");
    let cache = DiskCache::open(folder.clone(), 100).unwrap();
    let first = put(&cache, A, b"original encoding");
    assert!(first
        .path
        .extension()
        .is_some_and(|extension| extension == "png"));
    let fetched_at = first.fetched_at;
    drop(cache);
    let cache = DiskCache::open(folder, 100).unwrap();
    let restored = cache.lookup(A).unwrap().asset.unwrap();
    assert_eq!(restored.path, first.path);
    assert_eq!(restored.fetched_at, fetched_at);
    assert_eq!(cache.read(&restored).unwrap(), b"original encoding");
    let stored: String = cache
        .state
        .lock()
        .unwrap()
        .db
        .query_row("SELECT url_key FROM covers", [], |row| row.get(0))
        .unwrap();
    assert_eq!(stored, digest(A.as_bytes()));
    assert!(
        !String::from_utf8_lossy(&fs::read(cache.root.join("index.sqlite3")).unwrap())
            .contains("images.example")
    );
}

#[test]
fn corrupt_index_rebuilds_only_the_dedicated_cache() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("cache");
    let cache = DiskCache::open(folder.clone(), 100).unwrap();
    let asset = put(&cache, A, b"image");
    fs::write(folder.join("keep.txt"), b"unmanaged").unwrap();
    drop(cache);
    fs::write(folder.join("index.sqlite3"), b"not a sqlite database").unwrap();
    let cache = DiskCache::open(folder.clone(), 100).unwrap();
    assert!(cache.lookup(A).unwrap().asset.is_none());
    assert!(!asset.path.exists());
    assert_eq!(cache.status().unwrap().entries, 0);
    assert_eq!(fs::read(folder.join("keep.txt")).unwrap(), b"unmanaged");
    put(&cache, A, b"restored");
}

#[test]
fn reconciliation_checks_every_page_even_when_rows_are_removed() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 100).unwrap();
    let asset = put(&cache, A, b"shared");
    {
        let state = cache.state.lock().unwrap();
        for index in 0..300 {
            let filename = if index % 2 == 0 {
                asset.filename.as_str()
            } else {
                "invalid.png"
            };
            state.db.execute("INSERT INTO covers(url_key,filename,bytes,mime,fetched,accessed) VALUES(?1,?2,6,'image/png',0,1)", params![format!("entry-{index:04}"), filename]).unwrap();
        }
    }
    assert_eq!(cache.status().unwrap().used_bytes, 6);
    let entries: i64 = cache
        .state
        .lock()
        .unwrap()
        .db
        .query_row("SELECT COUNT(*) FROM covers", [], |row| row.get(0))
        .unwrap();
    assert_eq!(entries, 151);
    assert_eq!(cache.read(&asset).unwrap(), b"shared");
}

#[test]
fn content_deduplicates_across_urls_and_update_has_a_new_asset_path() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 20).unwrap();
    let first = put(&cache, A, b"shared");
    let duplicate = put(&cache, B, b"shared");
    assert_eq!(duplicate.path, first.path);
    assert_eq!(cache.status().unwrap().entries, 1);
    assert_eq!(cache.status().unwrap().used_bytes, 6);
    let updated = put(&cache, A, b"replacement");
    assert_ne!(updated.path, first.path);
    assert_eq!(cache.read(&first).unwrap(), b"shared");
    assert_eq!(cache.lookup(B).unwrap().asset.unwrap().path, first.path);
    cache.invalidate(B).unwrap();
    assert!(!first.path.exists());
    assert_eq!(cache.lookup(A).unwrap().asset.unwrap().path, updated.path);
}

#[test]
fn capacity_evicts_least_recently_read_content_and_rejects_oversized_writes() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 8).unwrap();
    let a = put(&cache, A, b"aaaa");
    let b = put(&cache, B, b"bbbb");
    assert!(cache.lookup(A).unwrap().asset.is_some());
    let c = put(&cache, C, b"cccc");
    assert!(a.path.exists());
    assert!(!b.path.exists());
    assert!(cache.lookup(B).unwrap().asset.is_none());
    assert_eq!(cache.status().unwrap().used_bytes, 8);
    assert!(cache
        .publish(B, b"too large", "image/png", cache.generation().unwrap())
        .is_err());
    assert_eq!(cache.read(&c).unwrap(), b"cccc");
    assert!(cache.read(&b).is_err());
}

#[test]
fn missing_truncated_and_equal_length_corruption_become_cache_misses() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 100).unwrap();
    for replacement in [
        None,
        Some(b"short".as_slice()),
        Some(b"modified".as_slice()),
    ] {
        let asset = put(&cache, A, b"original");
        match replacement {
            Some(bytes) => fs::write(&asset.path, bytes).unwrap(),
            None => fs::remove_file(&asset.path).unwrap(),
        }
        assert!(cache.lookup(A).unwrap().asset.is_none());
        assert_eq!(cache.status().unwrap().used_bytes, 0);
    }
}

#[test]
fn tampered_huge_file_is_rejected_before_loading_its_bytes() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 100).unwrap();
    let asset = put(&cache, A, b"small");
    fs::OpenOptions::new()
        .write(true)
        .open(&asset.path)
        .unwrap()
        .set_len(1024 * 1024 * 1024)
        .unwrap();
    assert!(cache.read(&asset).is_err());
    assert!(cache.lookup(A).unwrap().asset.is_none());
    assert!(!asset.path.exists());
}

#[test]
fn clear_and_single_url_invalidation_reject_previously_started_publications() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 100).unwrap();
    let a = put(&cache, A, b"first");
    let b = put(&cache, B, b"second");
    let initial = cache.lookup(A).unwrap().generation;
    cache.invalidate(A).unwrap();
    assert_ne!(cache.generation().unwrap(), initial);
    assert!(cache.publish(A, b"late", "image/png", initial).is_err());
    assert!(cache
        .publish(C, b"also late", "image/png", initial)
        .is_err());
    assert!(cache.read(&a).is_err());
    assert_eq!(cache.read(&b).unwrap(), b"second");
    let next = cache.generation().unwrap();
    cache.clear().unwrap();
    assert!(cache.publish(B, b"late", "image/png", next).is_err());
    assert!(cache.read(&b).is_err());
    assert_eq!(cache.status().unwrap().entries, 0);
    assert!(cache.lookup(A).unwrap().asset.is_none());
}

#[test]
fn clear_preserves_unmanaged_files_and_never_reads_index_path_traversal() {
    let root = tempfile::tempdir().unwrap();
    let cache = DiskCache::open(root.path().join("cache"), 100).unwrap();
    let outside = root.path().join("user.png");
    fs::write(&outside, b"user music cover").unwrap();
    fs::write(cache.root.join("notes.txt"), b"unmanaged").unwrap();
    fs::write(
        cache.images.join("keep.txt"),
        b"unmanaged image directory file",
    )
    .unwrap();
    cache.state.lock().unwrap().db.execute("INSERT INTO covers(url_key,filename,bytes,mime,fetched,accessed) VALUES(?1,'../../user.png',16,'image/png',0,1)", [digest(A.as_bytes())]).unwrap();
    assert!(cache.lookup(A).unwrap().asset.is_none());
    cache.clear().unwrap();
    assert_eq!(fs::read(outside).unwrap(), b"user music cover");
    assert_eq!(
        fs::read(cache.root.join("notes.txt")).unwrap(),
        b"unmanaged"
    );
    assert!(cache.images.join("keep.txt").exists());
}

#[cfg(any(unix, windows))]
fn directory_link(link: &Path, target: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[cfg(any(unix, windows))]
#[test]
fn accepts_host_parent_aliases_but_rejects_a_linked_cache_root_below_them() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("real-parent");
    fs::create_dir(&parent).unwrap();
    let alias = root.path().join("parent-alias");
    assert!(directory_link(&alias, &parent));
    let cache = DiskCache::open(alias.join("cache"), 100).unwrap();
    let asset = put(&cache, A, b"public cover");
    assert!(asset.path.starts_with(parent.canonicalize().unwrap()));
    assert_eq!(cache.read(&asset).unwrap(), b"public cover");
    drop(cache);
    let reopened = DiskCache::open(alias.join("cache"), 100).unwrap();
    assert_eq!(reopened.lookup(A).unwrap().asset.unwrap().path, asset.path);
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    assert!(directory_link(&alias.join("linked-cache"), &outside));
    assert!(DiskCache::open(alias.join("linked-cache"), 100).is_err());
}

#[cfg(any(unix, windows))]
#[test]
fn rejects_root_or_images_directory_links_without_touching_the_target() {
    let root = tempfile::tempdir().unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep.png"), b"user cover").unwrap();
    let linked_root = root.path().join("linked-cache");
    assert!(directory_link(&linked_root, &outside));
    assert!(DiskCache::open(linked_root, 100).is_err());
    let cache = DiskCache::open(root.path().join("cache"), 100).unwrap();
    fs::remove_dir(&cache.images).unwrap();
    assert!(directory_link(&cache.images, &outside));
    assert!(cache.lookup(A).is_err());
    assert!(cache.clear().is_err());
    assert!(cache.publish(A, b"cover", "image/png", 0).is_err());
    assert_eq!(fs::read(outside.join("keep.png")).unwrap(), b"user cover");
}

#[test]
fn refuses_link_or_non_file_at_index_and_asset_locations() {
    let root = tempfile::tempdir().unwrap();
    let cache_root = root.path().join("cache");
    fs::create_dir(&cache_root).unwrap();
    fs::create_dir(cache_root.join("index.sqlite3")).unwrap();
    assert!(DiskCache::open(cache_root, 100).is_err());
    let cache = DiskCache::open(root.path().join("valid"), 100).unwrap();
    let asset = put(&cache, A, b"image");
    fs::remove_file(&asset.path).unwrap();
    fs::create_dir(&asset.path).unwrap();
    assert!(cache.read(&asset).is_err());
    assert!(cache
        .publish(A, b"image", "image/png", cache.generation().unwrap())
        .is_err());
}
