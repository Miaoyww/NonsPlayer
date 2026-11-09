use crate::model::Syllable;

/// 从给定文本中移除括号
///
/// 只有在最外侧有左括号和右括号时才移除
pub fn strip_outer_parens(text: &mut String) {
    if let Some(stripped) = strip_outer_parens_str(text) {
        *text = stripped.into();
    }
}

fn strip_outer_parens_str(text: &str) -> Option<&str> {
    let trimmed = text.trim();

    let has_left = trimmed.starts_with(['(', '（']);
    let has_right = trimmed.ends_with([')', '）']);

    if has_left && has_right {
        let mut chars = trimmed.chars();
        chars.next();
        chars.next_back();
        Some(chars.as_str().trim())
    } else {
        None
    }
}

/// 从给定逐字歌词音节数组中移除括号
///
/// 只有在第一个和最后一个音节分别在最外侧有左括号和右括号时才移除
pub fn strip_outer_parens_from_words(words: &mut [Syllable]) {
    if words.is_empty() {
        return;
    }

    if words.len() == 1 {
        if let Some(first) = words.first_mut()
            && let Some(stripped) = strip_outer_parens_str(&first.text)
        {
            first.text = stripped.into();
        }
        return;
    }

    let first_has_left = words
        .first()
        .is_some_and(|w| w.text.trim_start().starts_with(['(', '（']));
    let last_has_right = words
        .last()
        .is_some_and(|w| w.text.trim_end().ends_with([')', '）']));

    if first_has_left && last_has_right {
        if let Some(first) = words.first_mut()
            && let Some(idx) = first.text.find(['(', '（'])
        {
            first.text.remove(idx);
        }
        if let Some(last) = words.last_mut()
            && let Some(idx) = last.text.rfind([')', '）'])
        {
            last.text.remove(idx);
        }
    }
}

/// 从给定逐字歌词音节数组构建纯文本
#[must_use]
pub fn build_full_text(words: &[Syllable], always_space: bool) -> String {
    let capacity = words.iter().map(|w| w.text.len() + 1).sum();
    let mut full_text = String::with_capacity(capacity);

    for word in words {
        full_text.push_str(&word.text);
        if always_space || word.ends_with_space.unwrap_or_default() {
            full_text.push(' ');
        }
    }

    while full_text.ends_with(' ') {
        full_text.pop();
    }

    full_text
}

/// 规范化逐行歌词的文本，将连续空白折叠为单个空格并去除首尾空格
#[allow(clippy::missing_panics_doc)]
pub fn normalize_line_text(text: &mut String) {
    if text.is_empty() || !text.contains(char::is_whitespace) {
        return;
    }

    let bytes = unsafe { text.as_mut_vec() };
    let len = bytes.len();
    let mut read_idx = 0;
    let mut write_idx = 0;
    let mut last_was_space = true;

    while read_idx < len {
        let b = bytes[read_idx];
        if b < 0x80 {
            // ASCII 快速路径
            // U+000B 属于 Unicode White_Space，但不在 `is_ascii_whitespace` 的集合内
            if b.is_ascii_whitespace() || b == 0x0B {
                if !last_was_space {
                    bytes[write_idx] = b' ';
                    write_idx += 1;
                    last_was_space = true;
                }
            } else {
                bytes[write_idx] = b;
                write_idx += 1;
                last_was_space = false;
            }
            read_idx += 1;
        } else {
            // 多字节 UTF-8 字符
            // 安全性：text 是合法 UTF-8，read_idx 始终对齐在合法字符边界
            let s = unsafe { std::str::from_utf8_unchecked(&bytes[read_idx..]) };
            let ch = s.chars().next().unwrap();
            let ch_len = ch.len_utf8();
            if ch.is_whitespace() {
                if !last_was_space {
                    bytes[write_idx] = b' ';
                    write_idx += 1;
                    last_was_space = true;
                }
            } else {
                bytes.copy_within(read_idx..read_idx + ch_len, write_idx);
                write_idx += ch_len;
                last_was_space = false;
            }
            read_idx += ch_len;
        }
    }

    if write_idx > 0 && bytes[write_idx - 1] == b' ' {
        write_idx -= 1;
    }

    bytes.truncate(write_idx);
}

/// 规范化给定歌词音节数组的空格
///
/// 会提取前导和尾随空格并分别标记上一个音节和当前音节的 `ends_with_space`
/// 标志，同时从音节文本内移除空格。如果移除空格后音节变为空，则会将其从数组中移除。
pub fn normalize_words_spaces(words: &mut Vec<Syllable>) {
    for i in 0..words.len() {
        let text = &words[i].text;
        let original_len = text.len();

        if original_len == 0 {
            continue;
        }

        let trimmed_start = text.trim_start();
        let leading_spaces_len = original_len - trimmed_start.len();

        // 空音节删除内容并标记上一个音节的空格
        if trimmed_start.is_empty() {
            if i > 0 {
                words[i - 1].ends_with_space = Some(true);
            }
            words[i].text.clear();
            continue;
        }

        let trimmed_both = trimmed_start.trim_end();
        let trailing_spaces_len = trimmed_start.len() - trimmed_both.len();

        if leading_spaces_len > 0 && i > 0 {
            words[i - 1].ends_with_space = Some(true);
        }

        if trailing_spaces_len > 0 {
            words[i].ends_with_space = Some(true);
        }

        if trailing_spaces_len > 0 {
            let new_len = original_len - trailing_spaces_len;
            words[i].text.truncate(new_len);
        }
        if leading_spaces_len > 0 {
            let _ = words[i].text.drain(..leading_spaces_len);
        }
    }

    // 移除所有文本为空的音节，包括原本就为空，以及只包含空格被清空的音节
    words.retain(|w| !w.text.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_line_text() {
        let mut s = String::new();
        normalize_line_text(&mut s);
        assert_eq!(s, "");

        let mut s = "   ".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "");

        let mut s = "hello".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "hello");

        let mut s = "  hello   world  ".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "hello world");

        let mut s = "a\t\nb\r\n\tc".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "a b c");

        let mut s = " \u{000B}a\u{000B}\u{000B}b\u{000B} ".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "a b");

        let mut s = "想简简单单爱".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "想简简单单爱");

        let mut s = "  \u{3000}你好   世界\u{3000}  ".to_string();
        normalize_line_text(&mut s);
        assert_eq!(s, "你好 世界");
    }
}
