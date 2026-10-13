use crate::model::AppResult;
#[cfg(desktop)]
use font_kit::source::SystemSource;

#[tauri::command]
pub async fn system_fonts() -> AppResult<Vec<String>> {
    #[cfg(target_os = "android")]
    return Ok(Vec::new());
    #[cfg(desktop)]
    tauri::async_runtime::spawn_blocking(|| {
        // Only enumerate family names; no font files or glyph data are loaded.
        let mut families = SystemSource::new()
            .all_families()
            .map_err(|e| format!("无法读取系统字体：{e}"))?;
        families.retain(|family| {
            !family.trim().is_empty()
                && family.chars().count() <= 256
                && !family.chars().any(char::is_control)
                && !family.starts_with('@')
        });
        families.sort_by_cached_key(|family| family.to_lowercase());
        families.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
        families.truncate(4096);
        Ok(families)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires installed desktop fonts"]
    fn installed_fonts_can_be_enumerated() {
        let families = tauri::async_runtime::block_on(super::system_fonts()).unwrap();
        assert!(
            !families.is_empty(),
            "the desktop should have installed font families"
        );
        #[cfg(windows)]
        assert!(families.iter().any(|family| family == "Segoe UI"));
        println!("Enumerated {} installed font families", families.len());
    }
}
