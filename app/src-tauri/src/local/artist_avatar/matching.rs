use crate::music::{
    adapter::PageRequest,
    business::{BusinessRequest, BusinessResponse, MusicEntity},
    identity::{EntityKind, SourceId},
    service::MusicService,
};
use nons_music_adapter_sdk::cover::{CoverImage, CoverSize};

pub(super) fn normalize_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|value| value.is_alphanumeric())
        .collect()
}

pub(super) async fn search_candidates(
    music: &MusicService,
    source: &SourceId,
    keyword: &str,
) -> crate::model::AppResult<Vec<MusicEntity>> {
    let request = BusinessRequest::Search {
        keyword: keyword.to_owned(),
        kind: EntityKind::Artist,
        page: PageRequest {
            cursor: None,
            limit: 30,
        },
    };
    match music
        .query(source, &request, false)
        .await
        .map_err(|e| e.to_string())?
    {
        BusinessResponse::Entities(page) => Ok(page.items),
        _ => Err("艺术家搜索响应无效".into()),
    }
}

pub(super) async fn artist_detail(
    music: &MusicService,
    source: &SourceId,
    reference: crate::music::identity::EntityRef,
) -> crate::model::AppResult<MusicEntity> {
    let request = BusinessRequest::Detail { entity: reference };
    match music
        .query(source, &request, false)
        .await
        .map_err(|e| e.to_string())?
    {
        BusinessResponse::Detail(detail) => Ok(detail.item),
        _ => Err("艺术家详情响应无效".into()),
    }
}

pub(super) fn preferred_cover(cover: &CoverImage) -> Option<(&str, CoverSize)> {
    [
        CoverSize::Exlarge,
        CoverSize::Large,
        CoverSize::Original,
        CoverSize::Normal,
        CoverSize::Small,
    ]
    .into_iter()
    .find_map(|size| {
        let url = cover.url(size);
        (!url.is_empty()).then_some((url, size))
    })
}

pub(super) enum MatchDecision {
    Exact(MusicEntity),
    Pending(usize),
    NotFound,
}

pub(super) fn decide_match(candidates: Vec<MusicEntity>, local_name: &str) -> MatchDecision {
    if candidates.is_empty() {
        return MatchDecision::NotFound;
    }
    let count = candidates.len();
    let name = normalize_name(local_name);
    let mut exact = candidates
        .into_iter()
        .filter(|candidate| normalize_name(&candidate.name) == name);
    let Some(first) = exact.next() else {
        return MatchDecision::Pending(count);
    };
    let exact_count = 1 + exact.count();
    if exact_count > 1 {
        return MatchDecision::Pending(exact_count);
    }
    MatchDecision::Exact(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::{
        account::AccountRef,
        identity::{EntityRef, OpaqueId},
    };

    fn candidate(name: &str) -> MusicEntity {
        MusicEntity {
            reference: EntityRef {
                source: SourceId::try_from("netease".to_owned()).unwrap(),
                kind: EntityKind::Artist,
                id: OpaqueId::try_from(name.to_owned()).unwrap(),
            },
            name: name.to_owned(),
            cover: CoverImage::default(),
            subtitle: String::new(),
            track_count: 0,
            creator: None::<AccountRef>,
            liked: false,
            play_count: None,
            published_at: None,
            artists: vec![],
        }
    }

    #[test]
    fn names_ignore_case_spacing_and_punctuation_but_require_a_unique_match() {
        assert_eq!(normalize_name(" AC/DC "), "acdc");
        assert!(matches!(
            decide_match(vec![candidate("AC/DC")], "ACDC"),
            MatchDecision::Exact(_)
        ));
        assert!(matches!(
            decide_match(vec![candidate("AC/DC"), candidate("ACDC")], "ACDC"),
            MatchDecision::Pending(_)
        ));
        assert!(matches!(
            decide_match(vec![candidate("Other")], "ACDC"),
            MatchDecision::Pending(_)
        ));
        assert!(matches!(
            decide_match(vec![], "ACDC"),
            MatchDecision::NotFound
        ));
    }

    #[test]
    fn preferred_cover_uses_the_largest_available_adapter_size() {
        let cover: CoverImage = serde_json::from_value(serde_json::json!({
            "original": "",
            "small": "https://cdn.example/small.jpg",
            "normal": "https://cdn.example/normal.jpg",
            "large": "",
            "exlarge": ""
        }))
        .unwrap();
        assert_eq!(
            preferred_cover(&cover),
            Some(("https://cdn.example/normal.jpg", CoverSize::Normal))
        );
        assert_eq!(preferred_cover(&CoverImage::default()), None);
    }
}
