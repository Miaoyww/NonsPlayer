use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub md5: String,
    pub name: String,
    pub token: String,
    pub avatar_url: String,
    pub is_logged_in: bool,
    pub key: String,
}
