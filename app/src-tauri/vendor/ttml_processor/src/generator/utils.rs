use std::borrow::Cow;

use crate::{
    model::Syllable,
    utils::build_full_text,
};

/// 栈上格式化的时间戳
///
/// 避免 [`format!`] 的堆分配与 `core::fmt` 开销。最长形式为 `h:mm:ss.mmm`，
/// 共 14 个字符，缓冲区留有余量。
pub struct Timestamp {
    buf: [u8; 16],
    len: usize,
}

impl Timestamp {
    /// 以字符串切片的形式返回时间戳
    pub fn as_str(&self) -> &str {
        // 缓冲区只会写入 ASCII 数字、':' 与 '.'，必定是合法 UTF-8
        std::str::from_utf8(&self.buf[..self.len]).expect("时间戳缓冲区只包含 ASCII 字符")
    }
}

/// 将 `value` 以至少 `width` 位（左侧补零）追加写入缓冲区
#[inline]
const fn push_padded_digits(buf: &mut [u8; 16], pos: &mut usize, value: u32, width: usize) {
    let mut digits = [0u8; 10];
    let mut remaining = value;
    let mut count = 0;

    loop {
        digits[count] = b'0' + (remaining % 10) as u8;
        remaining /= 10;
        count += 1;
        if remaining == 0 {
            break;
        }
    }
    while count < width {
        digits[count] = b'0';
        count += 1;
    }
    while count > 0 {
        count -= 1;
        buf[*pos] = digits[count];
        *pos += 1;
    }
}

pub const fn format_timestamp(total_ms: u32) -> Timestamp {
    let ms = total_ms % 1000;
    let total_seconds = total_ms / 1000;
    let s = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let m = total_minutes % 60;
    let h = total_minutes / 60;

    let mut buf = [0u8; 16];
    let mut pos = 0;

    if h > 0 {
        push_padded_digits(&mut buf, &mut pos, h, 1);
        buf[pos] = b':';
        pos += 1;
        push_padded_digits(&mut buf, &mut pos, m, 2);
    } else if m > 0 {
        push_padded_digits(&mut buf, &mut pos, m, 1);
    } else {
        push_padded_digits(&mut buf, &mut pos, s, 1);
        buf[pos] = b'.';
        pos += 1;
        push_padded_digits(&mut buf, &mut pos, ms, 3);

        return Timestamp { buf, len: pos };
    }

    buf[pos] = b':';
    pos += 1;
    push_padded_digits(&mut buf, &mut pos, s, 2);
    buf[pos] = b'.';
    pos += 1;
    push_padded_digits(&mut buf, &mut pos, ms, 3);

    Timestamp { buf, len: pos }
}

/// 将毫秒格式化为秒数
///
/// 零写作 `"0"`，非零固定保留 3 位小数，例如 `"0.040"`、`"0.046"`、`"-0.271"`。
pub fn format_seconds(total_ms: i64) -> String {
    if total_ms == 0 {
        return "0".to_string();
    }

    let sign = if total_ms < 0 { "-" } else { "" };
    let abs = total_ms.unsigned_abs();

    format!("{sign}{}.{:03}", abs / 1000, abs % 1000)
}

/// 取出用于逐行输出的纯文本
///
/// `text` 为空时回退到从音节拼接，以兼容只填写了 `words` 的调用方
///
/// `space_joined`：为 `true` 时音节间始终插入空格，一般用于逐字音译
pub fn line_text<'a>(
    text: &'a str,
    words: Option<&[Syllable]>,
    space_joined: bool,
) -> Cow<'a, str> {
    if !text.is_empty() {
        return Cow::Borrowed(text);
    }

    words.map_or(Cow::Borrowed(""), |words| {
        Cow::Owned(build_full_text(words, space_joined))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(1152).as_str(), "1.152");
        assert_eq!(format_timestamp(46).as_str(), "0.046");
        assert_eq!(format_timestamp(10254).as_str(), "10.254");
        assert_eq!(format_timestamp(0).as_str(), "0.000");
        assert_eq!(format_timestamp(1000).as_str(), "1.000");
        assert_eq!(format_timestamp(500).as_str(), "0.500");
        assert_eq!(format_timestamp(10).as_str(), "0.010");
        assert_eq!(format_timestamp(100).as_str(), "0.100");

        assert_eq!(format_timestamp(216_120).as_str(), "3:36.120");
        assert_eq!(format_timestamp(60000).as_str(), "1:00.000");

        assert_eq!(format_timestamp(3_816_120).as_str(), "1:03:36.120");
    }

    #[test]
    fn test_format_seconds() {
        assert_eq!(format_seconds(0), "0");
        assert_eq!(format_seconds(40), "0.040");
        assert_eq!(format_seconds(46), "0.046");
        assert_eq!(format_seconds(500), "0.500");
        assert_eq!(format_seconds(-271), "-0.271");
        assert_eq!(format_seconds(1152), "1.152");
    }
}
