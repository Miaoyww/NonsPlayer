use super::Query;
use crate::error::Result;
/// 歌单详情
/// 对应 Node.js module/playlist_detail.js
use crate::request::{ApiClient, ApiResponse, CryptoType};
use serde_json::json;

impl ApiClient {
    /// 歌单详情
    /// 对应 /playlist/detail
    pub async fn playlist_detail(&self, query: &Query) -> Result<ApiResponse> {
        let data = json!({
            "id": query.get_or("id", "0"),
            "n": query.get_or("n", "100000").parse::<u32>().unwrap_or(100000).min(100000),
            "s": query.get_or("s", "8").parse::<i64>().unwrap_or(8)
        });
        self.request(
            "/api/v6/playlist/detail",
            data,
            query.to_option(CryptoType::default()),
        )
        .await
    }
}
