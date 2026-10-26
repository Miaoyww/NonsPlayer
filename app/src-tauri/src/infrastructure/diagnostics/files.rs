//! Session files share the plugin's bounded rotation; expired sessions are removed at startup.
use std::{
    fs, io,
    path::Path,
    sync::LazyLock,
    time::{Duration, SystemTime},
};
use time::OffsetDateTime;

static OWNED: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(?:[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}-[0-9]{2}-[0-9]{2}-[0-9]+(?:-[0-9]+)?(?:_[0-9_-]+)?|nons(?:_[0-9_-]+)?)\.log(?:\.bak)?$").unwrap()
});

fn owned_files(directory: &Path) -> crate::model::AppResult<Vec<std::path::PathBuf>> {
    Ok(
        crate::infrastructure::other_cache::resource_files(directory, false)?
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| OWNED.is_match(&name.to_string_lossy()))
            })
            .collect(),
    )
}

pub(super) fn cache_status(
    directory: &Path,
) -> crate::model::AppResult<crate::infrastructure::other_cache::Usage> {
    crate::infrastructure::other_cache::file_usage(&owned_files(directory)?)
}

pub(super) fn clear_cache(directory: &Path, active: Option<&Path>) -> crate::model::AppResult<()> {
    let active = match active.map(Path::canonicalize).transpose() {
        Ok(active) => active,
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    for file in owned_files(directory)? {
        crate::infrastructure::other_cache::checked_resource(directory, &file)?;
        let result = if active.as_ref().is_some_and(|active| active == &file) {
            // Keep the logger's open file handle valid and ready for the next append.
            fs::OpenOptions::new()
                .write(true)
                .open(&file)
                .and_then(|file| file.set_len(0))
        } else {
            fs::remove_file(&file)
        };
        match result {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

pub(super) fn timestamp(now: OffsetDateTime) -> String {
    format!(
        "{} {:02}:{:02}:{:02}.{:03} {}",
        now.date(),
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond(),
        now.offset()
    )
}

pub(super) fn reserve(directory: &Path, now: OffsetDateTime, pid: u32) -> io::Result<String> {
    fs::create_dir_all(directory)?;
    let base = format!(
        "{}_{:02}-{:02}-{:02}-{pid}",
        now.date(),
        now.hour(),
        now.minute(),
        now.second()
    );
    for collision in 0.. {
        let name = if collision == 0 {
            base.clone()
        } else {
            format!("{base}-{collision}")
        };
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{name}.log")))
        {
            Ok(_) => return Ok(name),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    unreachable!()
}

pub(super) fn prune(directory: &Path, now: SystemTime) -> io::Result<()> {
    let mut first_error = None;
    for entry in fs::read_dir(directory)? {
        let result = (|| -> io::Result<()> {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || !OWNED.is_match(&entry.file_name().to_string_lossy())
            {
                return Ok(());
            }
            if now
                .duration_since(entry.metadata()?.modified()?)
                .unwrap_or_default()
                > Duration::from_secs(7 * 24 * 60 * 60)
            {
                fs::remove_file(entry.path())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clear_logs_removes_owned_history_and_current_session_can_keep_appending() {
        use std::io::Write;
        let directory = tempfile::tempdir().unwrap();
        let active = directory.path().join("2026-10-26_12-00-00-123.log");
        let history = directory.path().join("2026-10-25_12-00-00-123.log");
        let rotation = directory
            .path()
            .join("2026-10-26_12-00-00-123_2026-10-26_12-05-00.log");
        let unrelated = directory.path().join("notes.log");
        let mut writer = fs::OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&active)
            .unwrap();
        writer.write_all(b"old current").unwrap();
        fs::write(&history, "history").unwrap();
        fs::write(&rotation, "rotation").unwrap();
        fs::write(&unrelated, "notes").unwrap();
        assert_eq!(cache_status(directory.path()).unwrap().entries, 3);
        clear_cache(directory.path(), Some(&active)).unwrap();
        assert!(!history.exists());
        assert!(!rotation.exists());
        assert_eq!(fs::read(&active).unwrap().len(), 0);
        writer.write_all(b"new event").unwrap();
        writer.flush().unwrap();
        assert_eq!(fs::read_to_string(&active).unwrap(), "new event");
        assert_eq!(fs::read_to_string(&unrelated).unwrap(), "notes");
        assert_eq!(cache_status(directory.path()).unwrap().used_bytes, 9);
    }
    #[test]
    fn every_launch_reserves_a_separate_local_time_file() {
        let dir = tempfile::tempdir().unwrap();
        let utc = OffsetDateTime::from_unix_timestamp(1792217935).unwrap();
        let now = utc.to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
        let first = reserve(dir.path(), now, 123).unwrap();
        fs::write(dir.path().join(format!("{first}.log")), "previous session").unwrap();
        let second = reserve(dir.path(), now, 123).unwrap();
        assert_ne!(first, second);
        assert_eq!(first, "2026-10-17_14-18-55-123");
        assert_eq!(
            fs::read_to_string(dir.path().join(format!("{first}.log"))).unwrap(),
            "previous session"
        );
        assert!(timestamp(now).starts_with("2026-10-17 14:18:55.000 +08:00"));
    }
    #[test]
    fn retention_removes_only_owned_logs_older_than_seven_days() {
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let old = now - Duration::from_secs(8 * 86400);
        for name in [
            "2026-10-17_14-18-55-123.log",
            "2026-10-17_14-18-55-123_2026-10-17_14-19-00.log",
            "nons.log",
            "notes.log",
        ] {
            let file = fs::File::create(dir.path().join(name)).unwrap();
            file.set_modified(old).unwrap();
        }
        let recent = reserve(dir.path(), OffsetDateTime::now_utc(), 456).unwrap();
        prune(dir.path(), now).unwrap();
        assert!(dir.path().join(format!("{recent}.log")).exists());
        assert!(dir.path().join("notes.log").exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}
