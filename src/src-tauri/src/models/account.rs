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

impl Account {
    pub fn empty() -> Self {
        Self {
            id: String::new(),
            md5: String::new(),
            name: String::new(),
            token: String::new(),
            avatar_url: String::new(),
            is_logged_in: false,
            key: String::new(),
        }
    }

    pub fn not_logged_in() -> Self {
        Self {
            name: "未登录".into(),
            ..Self::empty()
        }
    }
}
