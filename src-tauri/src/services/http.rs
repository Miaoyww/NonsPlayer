use reqwest::Client;

pub fn create_client() -> Client {
    Client::builder()
        .user_agent("NonsPlayer/0.1.0")
        .build()
        .expect("failed to create HTTP client")
}
