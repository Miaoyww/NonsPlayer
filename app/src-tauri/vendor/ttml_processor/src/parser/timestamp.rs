/// 解析给定的时间戳字节数组为毫秒
///
/// 严格按照 Apple Music 会使用的时间戳格式来解析
///
/// # Returns
///
/// 如果时间戳解析失败，将返回 None，否则返回毫秒数
#[inline]
pub const fn parse_timestamp(bytes: &[u8]) -> Option<u32> {
    // 快速路径 1：M:SS.mmm（8 字节，如 "3:36.120"）
    if bytes.len() == 8 && bytes[1] == b':' && bytes[4] == b'.' {
        let d0 = bytes[0].wrapping_sub(b'0');
        let d2 = bytes[2].wrapping_sub(b'0');
        let d3 = bytes[3].wrapping_sub(b'0');
        let d5 = bytes[5].wrapping_sub(b'0');
        let d6 = bytes[6].wrapping_sub(b'0');
        let d7 = bytes[7].wrapping_sub(b'0');
        if d0 <= 9 && d2 <= 9 && d3 <= 9 && d5 <= 9 && d6 <= 9 && d7 <= 9 {
            let mins = d0 as u32;
            let secs = d2 as u32 * 10 + d3 as u32;
            let ms = d5 as u32 * 100 + d6 as u32 * 10 + d7 as u32;
            return Some((mins * 60 + secs) * 1000 + ms);
        }
    }
    // 快速路径 2：MM:SS.mmm（9 字节，如 "03:36.120"）
    if bytes.len() == 9 && bytes[2] == b':' && bytes[5] == b'.' {
        let d0 = bytes[0].wrapping_sub(b'0');
        let d1 = bytes[1].wrapping_sub(b'0');
        let d3 = bytes[3].wrapping_sub(b'0');
        let d4 = bytes[4].wrapping_sub(b'0');
        let d6 = bytes[6].wrapping_sub(b'0');
        let d7 = bytes[7].wrapping_sub(b'0');
        let d8 = bytes[8].wrapping_sub(b'0');
        if d0 <= 9 && d1 <= 9 && d3 <= 9 && d4 <= 9 && d6 <= 9 && d7 <= 9 && d8 <= 9 {
            let mins = d0 as u32 * 10 + d1 as u32;
            let secs = d3 as u32 * 10 + d4 as u32;
            let ms = d6 as u32 * 100 + d7 as u32 * 10 + d8 as u32;
            return Some((mins * 60 + secs) * 1000 + ms);
        }
    }
    // 快速路径 3：SS.mmm（6 字节，如 "10.254"）
    if bytes.len() == 6 && bytes[2] == b'.' {
        let d0 = bytes[0].wrapping_sub(b'0');
        let d1 = bytes[1].wrapping_sub(b'0');
        let d3 = bytes[3].wrapping_sub(b'0');
        let d4 = bytes[4].wrapping_sub(b'0');
        let d5 = bytes[5].wrapping_sub(b'0');
        if d0 <= 9 && d1 <= 9 && d3 <= 9 && d4 <= 9 && d5 <= 9 {
            let secs = d0 as u32 * 10 + d1 as u32;
            let ms = d3 as u32 * 100 + d4 as u32 * 10 + d5 as u32;
            return Some(secs * 1000 + ms);
        }
    }
    // 快速路径 4：S.mmm（5 字节，如 "1.152"）
    if bytes.len() == 5 && bytes[1] == b'.' {
        let d0 = bytes[0].wrapping_sub(b'0');
        let d2 = bytes[2].wrapping_sub(b'0');
        let d3 = bytes[3].wrapping_sub(b'0');
        let d4 = bytes[4].wrapping_sub(b'0');
        if d0 <= 9 && d2 <= 9 && d3 <= 9 && d4 <= 9 {
            let secs = d0 as u32;
            let ms = d2 as u32 * 100 + d3 as u32 * 10 + d4 as u32;
            return Some(secs * 1000 + ms);
        }
    }

    parse_timestamp_fallback(bytes)
}

#[inline(never)]
const fn parse_timestamp_fallback(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() {
        return Some(0);
    }
    let mut accum = 0u64;
    let mut current = 0u64;
    let mut colons = 0u8;
    let mut has_fraction = false;
    let mut split_idx = bytes.len();

    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'0'..=b'9' => {
                current = current.wrapping_mul(10).wrapping_add((b - b'0') as u64);
                if current > (u32::MAX as u64) {
                    return None;
                }
            }
            b':' => {
                if colons >= 2 {
                    return None;
                }
                accum = (accum + current).wrapping_mul(60);
                if accum > (u32::MAX as u64) {
                    return None;
                }
                current = 0;
                colons += 1;
            }
            b'.' => {
                accum += current;
                if accum > (u32::MAX as u64) {
                    return None;
                }
                has_fraction = true;
                split_idx = i + 1;
                break;
            }
            _ => return None,
        }
        i += 1;
    }

    let fraction = if has_fraction {
        let mut frac = 0u32;
        let mut digits = 0u32;
        while split_idx < bytes.len() {
            let b = bytes[split_idx];
            if b.is_ascii_digit() {
                if digits < 3 {
                    frac = frac * 10 + (b - b'0') as u32;
                    digits += 1;
                }
            } else {
                return None;
            }
            split_idx += 1;
        }
        if digits == 1 {
            frac * 100
        } else if digits == 2 {
            frac * 10
        } else {
            frac
        }
    } else {
        accum += current;
        if accum > (u32::MAX as u64) {
            return None;
        }
        0
    };

    let total_ms = accum.wrapping_mul(1000).wrapping_add(fraction as u64);
    if total_ms > (u32::MAX as u64) {
        None
    } else {
        Some(total_ms as u32)
    }
}

/// 解析十进制秒数为毫秒
///
/// 与 [`parse_timestamp`] 不同，允许前导 `-` 或 `+`，且只接受 `"0"`、`"0.046"`、
/// `"-0.271"` 这样的十进制秒数，不接受时钟形式。小数超过 3 位时截断；
/// 空串、缺少数字、多个小数点等非法输入返回 `None`。
///
/// 内部复用 [`parse_timestamp`]，因此数值范围也与其一致，
/// 即绝对值不超过 `u32::MAX` 毫秒。
#[inline]
pub fn parse_decimal_seconds(bytes: &[u8]) -> Option<i64> {
    let (negative, digits) = match *bytes.first()? {
        b'-' => (true, &bytes[1..]),
        b'+' => (false, &bytes[1..]),
        _ => (false, bytes),
    };

    // 时钟形式（含冒号）不属于十进制秒数；同时排除仅有符号或小数点的输入
    if digits.contains(&b':') || !digits.iter().any(u8::is_ascii_digit) {
        return None;
    }

    let ms = i64::from(parse_timestamp(digits)?);

    Some(if negative { -ms } else { ms })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::all, clippy::pedantic)]

    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(parse_timestamp(b"1.152").unwrap(), 1152);
        assert_eq!(parse_timestamp(b"0.046").unwrap(), 46);
        assert_eq!(parse_timestamp(b"10.254").unwrap(), 10254);

        assert_eq!(parse_timestamp(b"3:36.120").unwrap(), 216_120);
        assert_eq!(parse_timestamp(b"1:00").unwrap(), 60000);

        assert_eq!(parse_timestamp(b"1:03:36.120").unwrap(), 3_816_120);
        assert_eq!(parse_timestamp(b"4294967.295").unwrap(), u32::MAX);

        assert!(parse_timestamp(b"4294967.296").is_none());
        assert!(parse_timestamp(b"99999999999999999999").is_none());
        assert!(parse_timestamp(b"invalid").is_none());
    }

    #[test]
    fn test_parse_decimal_seconds() {
        assert_eq!(parse_decimal_seconds(b"0"), Some(0));
        assert_eq!(parse_decimal_seconds(b"0.040"), Some(40));
        assert_eq!(parse_decimal_seconds(b"0.046"), Some(46));
        assert_eq!(parse_decimal_seconds(b"0.5"), Some(500));
        assert_eq!(parse_decimal_seconds(b"1.152"), Some(1152));
        assert_eq!(parse_decimal_seconds(b"-0.271"), Some(-271));
        assert_eq!(parse_decimal_seconds(b"+1.5"), Some(1500));
        assert_eq!(parse_decimal_seconds(b"1.2345"), Some(1234));
        assert_eq!(parse_decimal_seconds(b"4294967.295"), Some(u32::MAX.into()));

        assert!(parse_decimal_seconds(b"4294967.296").is_none());
        assert!(parse_decimal_seconds(b"").is_none());
        assert!(parse_decimal_seconds(b"+").is_none());
        assert!(parse_decimal_seconds(b"-").is_none());
        assert!(parse_decimal_seconds(b".").is_none());
        assert!(parse_decimal_seconds(b"1:02").is_none());
        assert!(parse_decimal_seconds(b"1.2.3").is_none());
        assert!(parse_decimal_seconds(b"invalid").is_none());
    }

    #[test]
    fn test_u32_max_boundary_precision() {
        // 4294967 秒 + 295 毫秒 = 4,294,967,295 毫秒 = u32::MAX
        assert_eq!(parse_timestamp(b"4294967.295"), Some(u32::MAX));
        assert_eq!(parse_timestamp_fallback(b"4294967.295"), Some(u32::MAX));

        // 4294967 秒 + 296 毫秒 = 4,294,967,296 毫秒 = u32::MAX + 1 -> 必须返回 None
        assert_eq!(parse_timestamp(b"4294967.296"), None);
        assert_eq!(parse_timestamp_fallback(b"4294967.296"), None);

        // 邻近边界
        assert_eq!(parse_timestamp(b"4294967.294"), Some(u32::MAX - 1));
        assert_eq!(parse_timestamp_fallback(b"4294967.294"), Some(u32::MAX - 1));

        assert_eq!(parse_timestamp(b"4294967.000"), Some(4_294_967_000));
        assert_eq!(
            parse_timestamp_fallback(b"4294967.000"),
            Some(4_294_967_000)
        );

        // 溢出渐进值
        assert_eq!(parse_timestamp(b"4294967.297"), None);
        assert_eq!(parse_timestamp(b"4294967.999"), None);
        assert_eq!(parse_timestamp(b"4294968.000"), None);
        assert_eq!(parse_timestamp(b"4294968.295"), None);
        assert_eq!(parse_timestamp(b"5000000.000"), None);

        // 无小数点的秒数边界：4294967 * 1000 = 4294967000 <= u32::MAX
        assert_eq!(parse_timestamp(b"4294967"), Some(4_294_967_000));
        // 4294968 * 1000 = 4294968000 > u32::MAX -> None
        assert_eq!(parse_timestamp(b"4294968"), None);
        assert_eq!(parse_timestamp_fallback(b"4294968"), None);

        // 时钟形式的分秒边界：71582 分 47 秒 295 毫秒 = 4294967.295 秒
        assert_eq!(parse_timestamp(b"71582:47.295"), Some(u32::MAX));
        assert_eq!(parse_timestamp_fallback(b"71582:47.295"), Some(u32::MAX));

        assert_eq!(parse_timestamp(b"71582:47.296"), None);
        assert_eq!(parse_timestamp_fallback(b"71582:47.296"), None);
        assert_eq!(parse_timestamp(b"71582:48.000"), None);
        assert_eq!(parse_timestamp(b"71583:00.000"), None);
        assert_eq!(parse_timestamp(b"4294967295:00.000"), None);
        assert_eq!(parse_timestamp(b"4294967296:00.000"), None);

        // 时分秒边界：1193 小时 2 分 47 秒 295 毫秒 = 4294967.295 秒
        assert_eq!(parse_timestamp(b"1193:02:47.295"), Some(u32::MAX));
        assert_eq!(parse_timestamp_fallback(b"1193:02:47.295"), Some(u32::MAX));

        assert_eq!(parse_timestamp(b"1193:02:47.296"), None);
        assert_eq!(parse_timestamp_fallback(b"1193:02:47.296"), None);
        assert_eq!(parse_timestamp(b"1193:02:48.000"), None);
        assert_eq!(parse_timestamp(b"1193:03:00.000"), None);
        assert_eq!(parse_timestamp(b"1194:00:00.000"), None);
        assert_eq!(parse_timestamp(b"4294967295:00:00.000"), None);
        assert_eq!(parse_timestamp(b"4294967296:00:00.000"), None);

        // parse_decimal_seconds 符号与极值
        assert_eq!(parse_decimal_seconds(b"+0"), Some(0));
        assert_eq!(parse_decimal_seconds(b"-0"), Some(0));
        assert_eq!(parse_decimal_seconds(b"+0.0"), Some(0));
        assert_eq!(parse_decimal_seconds(b"-0.0"), Some(0));
        assert_eq!(parse_decimal_seconds(b"+0.000"), Some(0));
        assert_eq!(parse_decimal_seconds(b"-0.000"), Some(0));
        assert_eq!(parse_decimal_seconds(b"+0.001"), Some(1));
        assert_eq!(parse_decimal_seconds(b"-0.001"), Some(-1));
        assert_eq!(parse_decimal_seconds(b"+.1"), Some(100));
        assert_eq!(parse_decimal_seconds(b"-.1"), Some(-100));
        assert_eq!(
            parse_decimal_seconds(b"+4294967.295"),
            Some(u32::MAX as i64)
        );
        assert_eq!(
            parse_decimal_seconds(b"-4294967.295"),
            Some(-(u32::MAX as i64))
        );
        assert_eq!(parse_decimal_seconds(b"+4294967.296"), None);
        assert_eq!(parse_decimal_seconds(b"-4294967.296"), None);
        assert_eq!(parse_decimal_seconds(b"1:00"), None);
        assert_eq!(parse_decimal_seconds(b"-1:00"), None);
        assert_eq!(parse_decimal_seconds(b"+1:00"), None);
    }

    #[test]
    fn test_decimal_truncation_and_digit_validation() {
        // 基础截断
        assert_eq!(parse_timestamp(b"1.2345"), Some(1234));
        assert_eq!(parse_timestamp_fallback(b"1.2345"), Some(1234));

        assert_eq!(parse_timestamp(b"1.2340"), Some(1234));
        assert_eq!(parse_timestamp(b"1.2349999999"), Some(1234));
        assert_eq!(parse_timestamp(b"0.0001"), Some(0));
        assert_eq!(parse_timestamp(b"0.0009"), Some(0));
        assert_eq!(parse_timestamp(b"0.0010"), Some(1));

        // u32::MAX 边界的小数截断
        assert_eq!(parse_timestamp(b"4294967.2950"), Some(u32::MAX));
        assert_eq!(parse_timestamp(b"4294967.2959"), Some(u32::MAX));
        assert_eq!(parse_timestamp(b"4294967.2960"), None);

        // 截断部分必须严格是 ASCII 数字，混入任何非法字符必须返回 None
        assert_eq!(parse_timestamp(b"1.234a"), None);
        assert_eq!(parse_timestamp_fallback(b"1.234a"), None);
        assert_eq!(parse_timestamp(b"1.234 "), None);
        assert_eq!(parse_timestamp(b"1.234\0"), None);
        assert_eq!(parse_timestamp(b"1.234."), None);
        assert_eq!(parse_timestamp(b"1.234:"), None);
        assert_eq!(parse_timestamp(b"1.234-"), None);
        assert_eq!(parse_timestamp(b"1.234+"), None);

        // 极端长小数串（全数字）
        let long_fraction = format!("1.234{}", "0".repeat(5000));
        assert_eq!(parse_timestamp(long_fraction.as_bytes()), Some(1234));
        assert_eq!(
            parse_timestamp_fallback(long_fraction.as_bytes()),
            Some(1234)
        );

        let long_fraction_invalid = format!("1.234{}x", "0".repeat(5000));
        assert_eq!(parse_timestamp(long_fraction_invalid.as_bytes()), None);
        assert_eq!(
            parse_timestamp_fallback(long_fraction_invalid.as_bytes()),
            None
        );
    }

    #[test]
    fn test_malformed_inputs_robustness() {
        // 空串（符合既有规范与回退设计）
        assert_eq!(parse_timestamp(b""), Some(0));
        assert_eq!(parse_decimal_seconds(b""), None);

        // 纯标点符号
        assert_eq!(parse_timestamp(b":"), Some(0));
        assert_eq!(parse_timestamp(b"::"), Some(0));
        assert_eq!(parse_timestamp(b":::"), None);
        assert_eq!(parse_timestamp(b"::::"), None);
        assert_eq!(parse_timestamp(b"."), Some(0));
        assert_eq!(parse_timestamp(b".."), None);
        assert_eq!(parse_timestamp(b"..."), None);
        assert_eq!(parse_timestamp(b":."), Some(0));
        assert_eq!(parse_timestamp(b".:"), None);
        assert_eq!(parse_timestamp(b":.:"), None);

        // parse_decimal_seconds 纯标点必须全部拒绝
        assert_eq!(parse_decimal_seconds(b":"), None);
        assert_eq!(parse_decimal_seconds(b"."), None);
        assert_eq!(parse_decimal_seconds(b"+"), None);
        assert_eq!(parse_decimal_seconds(b"-"), None);
        assert_eq!(parse_decimal_seconds(b"+."), None);
        assert_eq!(parse_decimal_seconds(b"-."), None);

        // 非 ASCII 字符与控制字符
        assert_eq!(parse_timestamp("１.２３４".as_bytes()), None);
        assert_eq!(parse_timestamp("1.2\u{A0}4".as_bytes()), None);
        assert_eq!(parse_timestamp(b"1.234\n"), None);
        assert_eq!(parse_timestamp(b"\t1.234"), None);
        assert_eq!(parse_timestamp(b" 1.234 "), None);
        assert_eq!(parse_timestamp(b"\01.234"), None);
        assert_eq!(parse_timestamp(b"1.234\0"), None);
        assert_eq!(parse_timestamp(&[0xFF, 0xFE, 0xFD]), None);

        // 超长数字与防回绕测试
        assert_eq!(parse_timestamp(b"99999999999999999999"), None);
        assert_eq!(parse_timestamp(b"18446744073709551615"), None); // u64::MAX
        assert_eq!(parse_timestamp(b"18446744073709551616"), None); // u64::MAX + 1
        assert_eq!(
            parse_timestamp(b"340282366920938463463374607431768211455"),
            None
        ); // u128::MAX

        let extreme_digits = "9".repeat(10000);
        assert_eq!(parse_timestamp(extreme_digits.as_bytes()), None);

        // 前导 0 测试
        let leading_zeros = format!("{}1.123", "0".repeat(5000));
        assert_eq!(parse_timestamp(leading_zeros.as_bytes()), Some(1123));

        let leading_zeros_u32_max = format!("{}4294967.295", "0".repeat(5000));
        assert_eq!(
            parse_timestamp(leading_zeros_u32_max.as_bytes()),
            Some(u32::MAX)
        );

        let leading_zeros_overflow = format!("{}4294967.296", "0".repeat(5000));
        assert_eq!(parse_timestamp(leading_zeros_overflow.as_bytes()), None);
    }

    #[test]
    fn test_fast_path_and_fallback_exhaustive_consistency_s_mmm() {
        // 快速路径 4：S.mmm（5 字节，如 1.152）
        // 全空间覆盖：S (0..=9) * mmm (000..=999) = 10,000 种组合全量穷举
        for s in 0..=9 {
            for m in 0..=999 {
                let buf = format!("{s}.{m:03}");
                let bytes = buf.as_bytes();
                assert_eq!(bytes.len(), 5);

                let fast = parse_timestamp(bytes);
                let fall = parse_timestamp_fallback(bytes);
                assert_eq!(fast, fall, "快速路径 4 与回退路径不一致: {buf}");
                assert_eq!(fast, Some(s * 1000 + m));
            }
        }
    }

    #[test]
    fn test_fast_path_and_fallback_exhaustive_consistency_ss_mmm() {
        // 快速路径 3：SS.mmm（6 字节，如 10.254）
        // 全空间覆盖：SS (00..=99) * mmm (000..=999) = 100,000 种组合全量穷举
        for s in 0..=99 {
            for m in 0..=999 {
                let buf = format!("{s:02}.{m:03}");
                let bytes = buf.as_bytes();
                assert_eq!(bytes.len(), 6);

                let fast = parse_timestamp(bytes);
                let fall = parse_timestamp_fallback(bytes);
                assert_eq!(fast, fall, "快速路径 3 与回退路径不一致: {buf}");
                assert_eq!(fast, Some(s * 1000 + m));
            }
        }
    }

    #[test]
    fn test_fast_path_and_fallback_exhaustive_consistency_m_ss_mmm() {
        // 快速路径 1：M:SS.mmm（8 字节，如 3:36.120）
        // 全空间覆盖：M (0..=9) * SS (00..=99) * mmm (000..=999) = 1,000,000 种组合全量穷举
        for m in 0..=9 {
            for s in 0..=99 {
                for ms in 0..=999 {
                    let buf = format!("{m}:{s:02}.{ms:03}");
                    let bytes = buf.as_bytes();
                    assert_eq!(bytes.len(), 8);

                    let fast = parse_timestamp(bytes);
                    let fall = parse_timestamp_fallback(bytes);
                    assert_eq!(fast, fall, "快速路径 1 与回退路径不一致: {buf}");
                    assert_eq!(fast, Some((m * 60 + s) * 1000 + ms));
                }
            }
        }
    }

    #[test]
    fn test_fast_path_and_fallback_dense_consistency_mm_ss_mmm() {
        // 快速路径 2：MM:SS.mmm（9 字节，如 03:36.120）
        // 密集抽样覆盖：对 MM (00..=99), SS (00..=99), mmm (边界及步进) 进行 100,000+ 次验证
        for mm in 0..=99 {
            for ss in 0..=99 {
                // 包含 0, 1, 9, 10, 50, 120, 500, 990, 998, 999 等边界毫秒值以及步进值
                for ms in [0, 1, 9, 10, 50, 120, 500, 990, 998, 999] {
                    let buf = format!("{mm:02}:{ss:02}.{ms:03}");
                    let bytes = buf.as_bytes();
                    assert_eq!(bytes.len(), 9);

                    let fast = parse_timestamp(bytes);
                    let fall = parse_timestamp_fallback(bytes);
                    assert_eq!(fast, fall, "快速路径 2 与回退路径不一致: {buf}");
                    assert_eq!(fast, Some((mm * 60 + ss) * 1000 + ms));
                }
            }
        }
    }

    #[test]
    fn test_fast_path_adversarial_tampering() {
        // 篡改测试：针对 4 种快速路径的每个字节位置，注入非数字、非法分隔符及控制字符
        // 验证快速路径在不匹配时能安全回退，且两者产生完全相同的 None 判定
        let templates = [
            "3:36.120",  // 8 bytes
            "03:36.120", // 9 bytes
            "10.254",    // 6 bytes
            "1.152",     // 5 bytes
        ];

        let bad_chars: [u8; 11] = [
            b'/', b':', b';', b'.', b'a', b'Z', b' ', b'\0', b'-', b'+', 0xFF,
        ];

        for t in templates {
            let original = t.as_bytes();
            for idx in 0..original.len() {
                for &bad in &bad_chars {
                    let mut mutated = original.to_vec();
                    mutated[idx] = bad;

                    let fast = parse_timestamp(&mutated);
                    let fall = parse_timestamp_fallback(&mutated);
                    assert_eq!(
                        fast,
                        fall,
                        "篡改样本快速路径与回退路径表现不一致: {:?}",
                        String::from_utf8_lossy(&mutated)
                    );
                }
            }
        }
    }

    #[test]
    fn test_pseudo_random_fuzzing_robustness_and_equivalence() {
        // 线性同余发生器（LCG），保证测试完全确定性可复现
        let mut state = 0x123456789abcdef0u64;
        let mut next_u64 = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            state
        };

        let sample_chars = b"0123456789:.-+ aZ\0\n\t\xFF";

        // 运行 100,000 次模糊迭代
        for _ in 0..100_000 {
            let rand_val = next_u64();
            let len = (rand_val % 32) as usize;
            let mut buf = Vec::with_capacity(len);

            for _ in 0..len {
                let byte_idx = (next_u64() as usize) % sample_chars.len();
                buf.push(sample_chars[byte_idx]);
            }

            // 验证 1：绝不 Panic，且快速路径与回退路径产出 100% 绝对一致
            let fast = parse_timestamp(&buf);
            let fall = parse_timestamp_fallback(&buf);
            assert_eq!(
                fast,
                fall,
                "模糊测试发现快速路径与回退路径不一致！样本: {:?}",
                String::from_utf8_lossy(&buf)
            );

            // 验证 2：如果解析成功，数值绝不超过 u32::MAX
            if let Some(ms) = fast {
                assert!(ms <= u32::MAX);
            }

            // 验证 3：parse_decimal_seconds 绝不 Panic
            let dec = parse_decimal_seconds(&buf);
            if let Some(ms) = dec {
                assert!(ms.unsigned_abs() <= u32::MAX as u64);
            }
        }
    }
}
