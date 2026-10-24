//! Read-only search smoke test. Prints counts, never account data or cookies.
#[path = "support/infrastructure.rs"]
#[allow(dead_code)]
mod infrastructure;
// Standalone probes do not redirect storage into the app performance workspace.
mod performance {
    pub fn root() -> Option<std::path::PathBuf> {
        None
    }
}
#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;
#[path = "support/netease.rs"]
#[allow(dead_code, unused_imports)]
mod netease;

#[tokio::main]
async fn main() -> Result<(), String> {
    let api = netease::probe_client()?;
    let keyword = "周杰伦";
    let suggestions = api.search_suggestions(keyword).await?;
    assert!(!suggestions.is_empty() && suggestions.len() <= 5);
    println!("suggestions: {}", suggestions.len());
    let tracks = api.search(keyword, 0).await?;
    assert!(!tracks.is_empty());
    println!("songs: {}", tracks.len());
    for kind in ["playlist", "artist", "album"] {
        let page = api.search_collections(keyword, kind, 0).await?;
        assert!(!page.items.is_empty());
        assert!(page
            .items
            .iter()
            .all(|item| item.kind == kind && !item.name.is_empty()));
        println!("{kind}: {}; more={}", page.items.len(), page.more);
        if page.more {
            let next = api.search_collections(keyword, kind, 30).await?;
            assert!(!next.items.is_empty());
            assert_ne!(page.items[0].id, next.items[0].id);
            println!("{kind} continuation: {}", next.items.len());
        }
    }
    Ok(())
}
