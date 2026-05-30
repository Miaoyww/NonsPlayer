use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("adapter not found: {0}")]
    AdapterNotFound(String),

    #[error("song not found: {0}")]
    SongNotFound(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("audio parse error: {0}")]
    AudioParse(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Allow converting Error to String for Tauri command return
impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}
