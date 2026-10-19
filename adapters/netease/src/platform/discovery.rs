use super::{
    checked,
    library::{array, collection, Collection},
    track_from_json, CollectionPage, Netease,
};
use crate::model::{AppResult, Track};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct PlaylistCategory {
    pub name: String,
    pub group: String,
}

// The recommendation endpoint has one batch; subsequent pages use the public catalogue.
fn catalogue_offset(section: &str, offset: u32) -> u32 {
    if section == "recommended" {
        offset.saturating_sub(30)
    } else {
        offset
    }
}

fn playlist_page(body: &Value, field: &str, recommendation: bool) -> CollectionPage {
    let values = array(body, field);
    CollectionPage {
        items: values
            .iter()
            .take(30)
            .filter_map(|v| collection(v, "playlist"))
            .collect(),
        more: recommendation
            || (!values.is_empty()
                && body
                    .get("more")
                    .and_then(Value::as_bool)
                    .unwrap_or(values.len() == 30)),
    }
}

impl Netease {
    pub async fn discovery_playlists(
        &self,
        section: &str,
        category: &str,
        order: &str,
        offset: u32,
    ) -> AppResult<CollectionPage> {
        if offset > 100_000 || category.len() > 128 || !["hot", "new"].contains(&order) {
            return Err("歌单筛选参数无效".into());
        }
        let query = self
            .query()?
            .param("limit", "30")
            .param("offset", &catalogue_offset(section, offset).to_string())
            .param("cat", category)
            .param("order", order);
        let personalized = self.account_credentials()?.is_some();
        let body = match section {
            "recommended" if offset == 0 && personalized => {
                checked(self.client.recommend_resource(&query)).await?
            }
            "recommended" if offset == 0 => checked(self.client.personalized(&query)).await?,
            "recommended" | "square" => checked(self.client.top_playlist(&query)).await?,
            _ => return Err("发现分类无效".into()),
        };
        let recommendation = section == "recommended" && offset == 0;
        Ok(playlist_page(
            &body,
            if recommendation {
                if personalized {
                    "recommend"
                } else {
                    "result"
                }
            } else {
                "playlists"
            },
            recommendation,
        ))
    }
    pub async fn discovery_radar(&self) -> AppResult<Collection> {
        let body = self
            .playlist_metadata(self.query()?.param("id", "3136952023"))
            .await?;
        collection(&body["playlist"], "playlist").ok_or_else(|| "私人雷达信息缺失".into())
    }
    pub async fn discovery_categories(&self) -> AppResult<Vec<PlaylistCategory>> {
        let body = checked(self.client.playlist_catlist(&self.query()?)).await?;
        Ok(array(&body, "sub")
            .iter()
            .take(200)
            .filter_map(|v| {
                let index = v.get("category")?.as_u64()?.to_string();
                Some(PlaylistCategory {
                    name: v.get("name")?.as_str()?.into(),
                    group: body["categories"][&index].as_str().unwrap_or("其他").into(),
                })
            })
            .collect())
    }
    pub async fn discovery_tracks(&self, kind: &str) -> AppResult<Vec<Track>> {
        let query = self.query()?;
        let body = match kind {
            "daily" => checked(self.client.recommend_songs(&query)).await?,
            "fm" => checked(self.client.personal_fm(&query)).await?,
            _ => return Err("推荐类型无效".into()),
        };
        let values = if kind == "daily" {
            body.pointer("/data/dailySongs")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
        } else {
            array(&body, "data")
        };
        Ok(values
            .iter()
            .take(100)
            .filter_map(track_from_json)
            .collect())
    }
    pub async fn discovery_dislike(&self, id: u64) -> AppResult<()> {
        if id == 0 {
            return Err("歌曲 ID 无效".into());
        }
        checked(
            self.client
                .fm_trash(&self.query()?.param("id", &id.to_string())),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recommended_continuation_starts_catalogue_at_zero() {
        assert_eq!(catalogue_offset("recommended", 30), 0);
        assert_eq!(catalogue_offset("recommended", 60), 30);
        assert_eq!(catalogue_offset("square", 30), 30);
    }

    #[test]
    fn short_recommendation_batch_still_has_catalogue_continuation() {
        let page = playlist_page(&json!({"recommend":[{"id":1}]}), "recommend", true);
        assert_eq!(page.items.len(), 1);
        assert!(page.more);
        assert!(playlist_page(&json!({"recommend":[]}), "recommend", true).more);
    }

    #[test]
    fn catalogue_end_and_empty_pages_stop_continuation() {
        assert!(
            !playlist_page(
                &json!({"playlists":[{"id":1}],"more":false}),
                "playlists",
                false
            )
            .more
        );
        assert!(
            playlist_page(
                &json!({"playlists":[{"id":1}],"more":true}),
                "playlists",
                false
            )
            .more
        );
        assert!(!playlist_page(&json!({"playlists":[],"more":true}), "playlists", false).more);
    }
}
