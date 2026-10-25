use crate::{model::AppResult, storage::Store};
use rusqlite::{params, OptionalExtension};

#[derive(Default)]
pub(super) struct CoverState {
    pub signature: String,
    pub embedded: Vec<String>,
}

impl Store {
    pub(super) fn local_cover_state(&self, key: &str) -> AppResult<Option<CoverState>> {
        let db = self.0.lock().map_err(|_| "曲库锁不可用")?;
        db.query_row(
            "SELECT signature,embedded FROM local_cover_state WHERE key=?1",
            [key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .map(|(signature, embedded)| {
            let embedded = serde_json::from_str(&embedded)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn(
                        "local.cover_state.read_decode",
                        error,
                    );
                })
                .unwrap_or_default();
            Ok(CoverState {
                signature,
                embedded,
            })
        })
        .transpose()
    }

    pub(super) fn save_local_cover_state(&self, key: &str, state: &CoverState) -> AppResult<()> {
        self.0.lock().map_err(|_| "曲库锁不可用")?.execute(
            "INSERT INTO local_cover_state(key,signature,embedded) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET signature=excluded.signature,embedded=excluded.embedded",
            params![key, state.signature, serde_json::to_string(&state.embedded).map_err(|e| e.to_string())?],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }
}
