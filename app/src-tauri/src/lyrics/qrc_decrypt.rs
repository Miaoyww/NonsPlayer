//! QRC block cipher adapted from lyrics-crypto 0.5.0 (ChouChiu), Apache-2.0.
//! See notices/lyrics-crypto-LICENSE.txt. Decompression is bounded by Nons.
use crate::{model::AppResult, storage::MAX_LYRIC_BYTES};
use std::io::Read;
const QRC_KEY: &[u8; 24] = b"!@#)(*$%123ZXC!@!@#)(NHL";

const SBOX: [[u8; 64]; 8] = [
    [
        14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7, 0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12,
        11, 9, 5, 3, 8, 4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0, 15, 12, 8, 2, 4, 9,
        1, 7, 5, 11, 3, 14, 10, 0, 6, 13,
    ],
    [
        15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10, 3, 13, 4, 7, 15, 2, 8, 15, 12, 0, 1,
        10, 6, 9, 11, 5, 0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15, 13, 8, 10, 1, 3, 15,
        4, 2, 11, 6, 7, 12, 0, 5, 14, 9,
    ],
    [
        10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8, 13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5,
        14, 12, 11, 15, 1, 13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7, 1, 10, 13, 0, 6,
        9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12,
    ],
    [
        7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15, 13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2,
        12, 1, 10, 14, 9, 10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4, 3, 15, 0, 6, 10,
        10, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14,
    ],
    [
        2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9, 14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15,
        10, 3, 9, 8, 6, 4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14, 11, 8, 12, 7, 1, 14,
        2, 13, 6, 15, 0, 9, 10, 4, 5, 3,
    ],
    [
        12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11, 10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13,
        14, 0, 11, 3, 8, 9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6, 4, 3, 2, 12, 9, 5,
        15, 10, 11, 14, 1, 7, 6, 0, 8, 13,
    ],
    [
        4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1, 13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5,
        12, 2, 15, 8, 6, 1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2, 6, 11, 13, 8, 1, 4,
        10, 7, 9, 5, 0, 15, 14, 2, 3, 12,
    ],
    [
        13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7, 1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6,
        11, 0, 14, 9, 2, 7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8, 2, 1, 14, 7, 4, 10,
        8, 13, 15, 12, 9, 0, 3, 5, 6, 11,
    ],
];

const KEY_RND_SHIFT: [u32; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];
const KEY_PERM_C: [u32; 28] = [
    56, 48, 40, 32, 24, 16, 8, 0, 57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2, 59,
    51, 43, 35,
];
const KEY_PERM_D: [u32; 28] = [
    62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 60, 52, 44, 36, 28, 20, 12, 4,
    27, 19, 11, 3,
];
const KEY_COMPRESSION: [u32; 48] = [
    13, 16, 10, 23, 0, 4, 2, 27, 14, 5, 20, 9, 22, 18, 11, 3, 25, 7, 15, 6, 26, 19, 12, 1, 40, 51,
    30, 36, 46, 54, 29, 39, 50, 44, 32, 47, 43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31,
];

/// 子密钥的使用方向：解密时 16 轮子密钥倒序排列。
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Encrypt,
    Decrypt,
}

fn bitnum(a: &[u8], b: usize, c: usize) -> u32 {
    let byte_idx = (b / 32) * 4 + 3 - (b % 32) / 8;
    (((a[byte_idx] >> (7 - (b % 8))) & 0x01) as u32) << c
}

fn bitnum_intr(a: u32, b: usize, c: usize) -> u8 {
    (((a >> (31 - b)) & 0x00000001) << c) as u8
}

fn bitnum_intl(a: u32, b: usize, c: usize) -> u32 {
    ((a << b) & 0x80000000) >> c
}

fn sbox_bit(a: u8) -> usize {
    ((a & 0x20) | ((a & 0x1f) >> 1) | ((a & 0x01) << 4)) as usize
}

fn initial_permutation(input: &[u8]) -> (u32, u32) {
    let s0 = bitnum(input, 57, 31)
        | bitnum(input, 49, 30)
        | bitnum(input, 41, 29)
        | bitnum(input, 33, 28)
        | bitnum(input, 25, 27)
        | bitnum(input, 17, 26)
        | bitnum(input, 9, 25)
        | bitnum(input, 1, 24)
        | bitnum(input, 59, 23)
        | bitnum(input, 51, 22)
        | bitnum(input, 43, 21)
        | bitnum(input, 35, 20)
        | bitnum(input, 27, 19)
        | bitnum(input, 19, 18)
        | bitnum(input, 11, 17)
        | bitnum(input, 3, 16)
        | bitnum(input, 61, 15)
        | bitnum(input, 53, 14)
        | bitnum(input, 45, 13)
        | bitnum(input, 37, 12)
        | bitnum(input, 29, 11)
        | bitnum(input, 21, 10)
        | bitnum(input, 13, 9)
        | bitnum(input, 5, 8)
        | bitnum(input, 63, 7)
        | bitnum(input, 55, 6)
        | bitnum(input, 47, 5)
        | bitnum(input, 39, 4)
        | bitnum(input, 31, 3)
        | bitnum(input, 23, 2)
        | bitnum(input, 15, 1)
        | bitnum(input, 7, 0);

    let s1 = bitnum(input, 56, 31)
        | bitnum(input, 48, 30)
        | bitnum(input, 40, 29)
        | bitnum(input, 32, 28)
        | bitnum(input, 24, 27)
        | bitnum(input, 16, 26)
        | bitnum(input, 8, 25)
        | bitnum(input, 0, 24)
        | bitnum(input, 58, 23)
        | bitnum(input, 50, 22)
        | bitnum(input, 42, 21)
        | bitnum(input, 34, 20)
        | bitnum(input, 26, 19)
        | bitnum(input, 18, 18)
        | bitnum(input, 10, 17)
        | bitnum(input, 2, 16)
        | bitnum(input, 60, 15)
        | bitnum(input, 52, 14)
        | bitnum(input, 44, 13)
        | bitnum(input, 36, 12)
        | bitnum(input, 28, 11)
        | bitnum(input, 20, 10)
        | bitnum(input, 12, 9)
        | bitnum(input, 4, 8)
        | bitnum(input, 62, 7)
        | bitnum(input, 54, 6)
        | bitnum(input, 46, 5)
        | bitnum(input, 38, 4)
        | bitnum(input, 30, 3)
        | bitnum(input, 22, 2)
        | bitnum(input, 14, 1)
        | bitnum(input, 6, 0);

    (s0, s1)
}

fn inverse_permutation(s0: u32, s1: u32) -> [u8; 8] {
    let mut data = [0u8; 8];
    data[3] = bitnum_intr(s1, 7, 7)
        | bitnum_intr(s0, 7, 6)
        | bitnum_intr(s1, 15, 5)
        | bitnum_intr(s0, 15, 4)
        | bitnum_intr(s1, 23, 3)
        | bitnum_intr(s0, 23, 2)
        | bitnum_intr(s1, 31, 1)
        | bitnum_intr(s0, 31, 0);

    data[2] = bitnum_intr(s1, 6, 7)
        | bitnum_intr(s0, 6, 6)
        | bitnum_intr(s1, 14, 5)
        | bitnum_intr(s0, 14, 4)
        | bitnum_intr(s1, 22, 3)
        | bitnum_intr(s0, 22, 2)
        | bitnum_intr(s1, 30, 1)
        | bitnum_intr(s0, 30, 0);

    data[1] = bitnum_intr(s1, 5, 7)
        | bitnum_intr(s0, 5, 6)
        | bitnum_intr(s1, 13, 5)
        | bitnum_intr(s0, 13, 4)
        | bitnum_intr(s1, 21, 3)
        | bitnum_intr(s0, 21, 2)
        | bitnum_intr(s1, 29, 1)
        | bitnum_intr(s0, 29, 0);

    data[0] = bitnum_intr(s1, 4, 7)
        | bitnum_intr(s0, 4, 6)
        | bitnum_intr(s1, 12, 5)
        | bitnum_intr(s0, 12, 4)
        | bitnum_intr(s1, 20, 3)
        | bitnum_intr(s0, 20, 2)
        | bitnum_intr(s1, 28, 1)
        | bitnum_intr(s0, 28, 0);

    data[7] = bitnum_intr(s1, 3, 7)
        | bitnum_intr(s0, 3, 6)
        | bitnum_intr(s1, 11, 5)
        | bitnum_intr(s0, 11, 4)
        | bitnum_intr(s1, 19, 3)
        | bitnum_intr(s0, 19, 2)
        | bitnum_intr(s1, 27, 1)
        | bitnum_intr(s0, 27, 0);

    data[6] = bitnum_intr(s1, 2, 7)
        | bitnum_intr(s0, 2, 6)
        | bitnum_intr(s1, 10, 5)
        | bitnum_intr(s0, 10, 4)
        | bitnum_intr(s1, 18, 3)
        | bitnum_intr(s0, 18, 2)
        | bitnum_intr(s1, 26, 1)
        | bitnum_intr(s0, 26, 0);

    data[5] = bitnum_intr(s1, 1, 7)
        | bitnum_intr(s0, 1, 6)
        | bitnum_intr(s1, 9, 5)
        | bitnum_intr(s0, 9, 4)
        | bitnum_intr(s1, 17, 3)
        | bitnum_intr(s0, 17, 2)
        | bitnum_intr(s1, 25, 1)
        | bitnum_intr(s0, 25, 0);

    data[4] = bitnum_intr(s1, 0, 7)
        | bitnum_intr(s0, 0, 6)
        | bitnum_intr(s1, 8, 5)
        | bitnum_intr(s0, 8, 4)
        | bitnum_intr(s1, 16, 3)
        | bitnum_intr(s0, 16, 2)
        | bitnum_intr(s1, 24, 1)
        | bitnum_intr(s0, 24, 0);

    data
}

fn des_f(state: u32, key: &[u8]) -> u32 {
    let t1 = bitnum_intl(state, 31, 0)
        | ((state & 0xf0000000) >> 1)
        | bitnum_intl(state, 4, 5)
        | bitnum_intl(state, 3, 6)
        | ((state & 0x0f000000) >> 3)
        | bitnum_intl(state, 8, 11)
        | bitnum_intl(state, 7, 12)
        | ((state & 0x00f00000) >> 5)
        | bitnum_intl(state, 12, 17)
        | bitnum_intl(state, 11, 18)
        | ((state & 0x000f0000) >> 7)
        | bitnum_intl(state, 16, 23);

    let t2 = bitnum_intl(state, 15, 0)
        | ((state & 0x0000f000) << 15)
        | bitnum_intl(state, 20, 5)
        | bitnum_intl(state, 19, 6)
        | ((state & 0x00000f00) << 13)
        | bitnum_intl(state, 24, 11)
        | bitnum_intl(state, 23, 12)
        | ((state & 0x000000f0) << 11)
        | bitnum_intl(state, 28, 17)
        | bitnum_intl(state, 27, 18)
        | ((state & 0x0000000f) << 9)
        | bitnum_intl(state, 0, 23);

    let lrgstate: [u8; 6] = [
        ((t1 >> 24) & 0xff) as u8 ^ key[0],
        ((t1 >> 16) & 0xff) as u8 ^ key[1],
        ((t1 >> 8) & 0xff) as u8 ^ key[2],
        ((t2 >> 24) & 0xff) as u8 ^ key[3],
        ((t2 >> 16) & 0xff) as u8 ^ key[4],
        ((t2 >> 8) & 0xff) as u8 ^ key[5],
    ];

    let state = (SBOX[0][sbox_bit(lrgstate[0] >> 2)] as u32) << 28
        | (SBOX[1][sbox_bit(((lrgstate[0] & 0x03) << 4) | (lrgstate[1] >> 4))] as u32) << 24
        | (SBOX[2][sbox_bit(((lrgstate[1] & 0x0f) << 2) | (lrgstate[2] >> 6))] as u32) << 20
        | (SBOX[3][sbox_bit(lrgstate[2] & 0x3f)] as u32) << 16
        | (SBOX[4][sbox_bit(lrgstate[3] >> 2)] as u32) << 12
        | (SBOX[5][sbox_bit(((lrgstate[3] & 0x03) << 4) | (lrgstate[4] >> 4))] as u32) << 8
        | (SBOX[6][sbox_bit(((lrgstate[4] & 0x0f) << 2) | (lrgstate[5] >> 6))] as u32) << 4
        | SBOX[7][sbox_bit(lrgstate[5] & 0x3f)] as u32;

    bitnum_intl(state, 15, 0)
        | bitnum_intl(state, 6, 1)
        | bitnum_intl(state, 19, 2)
        | bitnum_intl(state, 20, 3)
        | bitnum_intl(state, 28, 4)
        | bitnum_intl(state, 11, 5)
        | bitnum_intl(state, 27, 6)
        | bitnum_intl(state, 16, 7)
        | bitnum_intl(state, 0, 8)
        | bitnum_intl(state, 14, 9)
        | bitnum_intl(state, 22, 10)
        | bitnum_intl(state, 25, 11)
        | bitnum_intl(state, 4, 12)
        | bitnum_intl(state, 17, 13)
        | bitnum_intl(state, 30, 14)
        | bitnum_intl(state, 9, 15)
        | bitnum_intl(state, 1, 16)
        | bitnum_intl(state, 7, 17)
        | bitnum_intl(state, 23, 18)
        | bitnum_intl(state, 13, 19)
        | bitnum_intl(state, 31, 20)
        | bitnum_intl(state, 26, 21)
        | bitnum_intl(state, 2, 22)
        | bitnum_intl(state, 8, 23)
        | bitnum_intl(state, 18, 24)
        | bitnum_intl(state, 12, 25)
        | bitnum_intl(state, 29, 26)
        | bitnum_intl(state, 5, 27)
        | bitnum_intl(state, 21, 28)
        | bitnum_intl(state, 10, 29)
        | bitnum_intl(state, 3, 30)
        | bitnum_intl(state, 24, 31)
}

fn des_crypt(input: &[u8], output: &mut [u8], key: &[[u8; 6]]) {
    let (mut s0, mut s1) = initial_permutation(input);

    for k in key.iter().take(15) {
        let t = s1;
        s1 = des_f(s1, k) ^ s0;
        s0 = t;
    }
    s0 ^= des_f(s1, &key[15]);

    let result = inverse_permutation(s0, s1);
    output.copy_from_slice(&result);
}

fn key_schedule(key: &[u8], schedule: &mut [[u8; 6]], mode: Mode) {
    let mut c: u32 = 0;
    let mut d: u32 = 0;

    for (i, &perm) in KEY_PERM_C.iter().enumerate() {
        c |= bitnum(key, perm as usize, 31 - i);
    }
    for (i, &perm) in KEY_PERM_D.iter().enumerate() {
        d |= bitnum(key, perm as usize, 31 - i);
    }

    for (i, &shift) in KEY_RND_SHIFT.iter().enumerate() {
        c = ((c << shift) | (c >> (28 - shift))) & 0xfffffff0;
        d = ((d << shift) | (d >> (28 - shift))) & 0xfffffff0;

        let togen = if mode == Mode::Decrypt { 15 - i } else { i };
        schedule[togen] = [0u8; 6];

        for j in 0..24 {
            schedule[togen][j / 8] |= bitnum_intr(c, KEY_COMPRESSION[j] as usize, 7 - (j % 8));
        }
        for j in 24..48 {
            schedule[togen][j / 8] |= bitnum_intr(d, KEY_COMPRESSION[j] as usize - 27, 7 - (j % 8));
        }
    }
}

type DesSchedule = [[u8; 6]; 16];

/// 3DES-EDE 解密的子密钥：按 K3 解密 → K2 加密 → K1 解密的顺序排列。
///
/// QRC 只需要解密方向，因此不保留上游的加密分支。
fn triple_des_decrypt_schedule(key: &[u8]) -> [DesSchedule; 3] {
    let mut schedule = [[[0u8; 6]; 16]; 3];
    key_schedule(&key[16..], &mut schedule[0], Mode::Decrypt);
    key_schedule(&key[8..], &mut schedule[1], Mode::Encrypt);
    key_schedule(&key[0..], &mut schedule[2], Mode::Decrypt);
    schedule
}

fn triple_des_crypt(input: &[u8], output: &mut [u8], key: &[DesSchedule; 3]) {
    let mut buf = [0u8; 8];
    des_crypt(input, &mut buf, &key[0]);
    des_crypt(&buf, output, &key[1]);
    des_crypt(output, &mut buf, &key[2]);
    output.copy_from_slice(&buf);
}

pub fn decrypt(hex_text: &str) -> AppResult<String> {
    if hex_text.len() > MAX_LYRIC_BYTES {
        return Err("QRC 密文过大".into());
    }
    let hex_text: String = hex_text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut data = hex::decode(hex_text)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "QRC 密文无效")?;
    if data.is_empty() || data.len() % 8 != 0 {
        return Err("QRC 密文长度无效".into());
    }
    let schedule = triple_des_decrypt_schedule(QRC_KEY);
    for block in data.chunks_exact_mut(8) {
        let mut output = [0; 8];
        triple_des_crypt(block, &mut output, &schedule);
        block.copy_from_slice(&output);
    }
    let mut decoded = Vec::new();
    flate2::read::ZlibDecoder::new(data.as_slice())
        .take(MAX_LYRIC_BYTES as u64 + 1)
        .read_to_end(&mut decoded)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "QRC 解压失败")?;
    if decoded.len() > MAX_LYRIC_BYTES {
        return Err("QRC 解压内容过大".into());
    }
    let text = String::from_utf8(decoded)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "QRC 编码无效")?;
    Ok(text.trim_start_matches('\u{feff}').to_owned())
}
