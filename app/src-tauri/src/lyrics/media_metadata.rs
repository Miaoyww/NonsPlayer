//! Validated line-only payloads parsed by AMLL; no second lyric parser or persistent cache.
use crate::model::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Line {
    pub start_time: u64,
    pub end_time: u64,
    pub text: String,
    pub translation: String,
    pub romanization: String,
}
pub(crate) fn validate(lines: &[Line]) -> AppResult<()> {
    if lines.len() > 5000 {
        return Err("系统歌词行数过多".into());
    }
    let mut bytes = 0usize;
    let mut previous = 0;
    for line in lines {
        if line.start_time < previous
            || line.end_time < line.start_time
            || line.end_time > 24 * 60 * 60 * 1000
            || [&line.text, &line.translation, &line.romanization]
                .iter()
                .any(|s| s.len() > 4096)
        {
            return Err("系统歌词时间轴或文本无效".into());
        }
        previous = line.start_time;
        bytes += line.text.len() + line.translation.len() + line.romanization.len();
    }
    if bytes > 1024 * 1024 {
        return Err("系统歌词内容过大".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(start: u64, end: u64) -> Line {
        Line {
            start_time: start,
            end_time: end,
            text: "fixture".into(),
            translation: String::new(),
            romanization: String::new(),
        }
    }
    #[test]
    fn validates_timeline_before_crossing_native_bridge() {
        assert!(validate(&[line(0, 1000), line(1000, 2000)]).is_ok());
        assert!(validate(&[line(1000, 2000), line(0, 1000)]).is_err());
        assert!(validate(&[line(1000, 500)]).is_err());
        assert!(validate(&[line(0, u64::MAX)]).is_err());
        let mut long = line(0, 1000);
        long.text = "x".repeat(4097);
        assert!(validate(&[long]).is_err());
        assert!(validate(&(0..5001).map(|i| line(i, i + 1)).collect::<Vec<_>>()).is_err());
    }
}
