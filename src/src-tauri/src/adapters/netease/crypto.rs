use aes::cipher::{BlockDecrypt, BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit};
use aes::Aes128;
use base64::Engine;
use num_bigint::BigUint;
use num_traits::Num;
use rand::Rng;
use std::borrow::Cow;

// ── Hardcoded constants ──

const PRESET_KEY: &[u8; 16] = b"0CoJUm6Qyw8W8jud";
const IV: &[u8; 16] = b"0102030405060708";
const EAPI_KEY: &[u8; 16] = b"e82ckenh8dichen8";
const BASE62: &[u8; 62] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// RSA modulus n for 1024-bit key, as hex (big-endian, 128 bytes)
const RSA_N_HEX: &str = concat!(
    "e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725",
    "152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312",
    "ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b4",
    "24d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7",
);

const RSA_E: u32 = 65537;

pub type Aes128Cbc = cbc::Encryptor<Aes128>;

// ── PKCS7 padding ──

fn pkcs7_pad(data: &[u8]) -> Cow<'_, [u8]> {
    let pad_len = 16 - (data.len() % 16);
    let mut padded = Vec::with_capacity(data.len() + pad_len);
    padded.extend_from_slice(data);
    padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    Cow::Owned(padded)
}

fn pkcs7_unpad(data: &[u8]) -> &[u8] {
    if data.is_empty() {
        return data;
    }
    let pad_len = *data.last().unwrap() as usize;
    if pad_len == 0 || pad_len > 16 || pad_len > data.len() {
        return data;
    }
    // Verify all pad bytes are correct
    if data[data.len() - pad_len..].iter().all(|&b| b == pad_len as u8) {
        &data[..data.len() - pad_len]
    } else {
        data
    }
}

// ── AES-128-ECB ──

fn aes128_ecb_encrypt(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).expect("AES key must be 16 bytes");
    let padded = pkcs7_pad(data);
    let mut result = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    result
}

fn aes128_ecb_decrypt(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).expect("AES key must be 16 bytes");
    let mut result = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    pkcs7_unpad(&result).to_vec()
}

// ── AES-128-CBC ──

fn aes128_cbc_encrypt(key: &[u8; 16], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    // Pad manually then encrypt with CBC
    let padded = pkcs7_pad(data);
    // cbc crate uses BlockEncrypt which doesn't auto-pad, so we pad first
    let mut cipher = Aes128Cbc::new_from_slices(key, iv).expect("CBC key/iv must be 16 bytes");
    let buf = padded.to_vec();
    let mut result = Vec::with_capacity(buf.len());
    for chunk in buf.chunks(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block_mut(&mut block);
        result.extend_from_slice(&block);
    }
    result
}

// ── RSA raw encrypt ──
// RSA_NO_PADDING: input must be exactly 128 bytes (1024-bit key).
// Compute c = m^e mod n, output as 128-byte big-endian.

fn rsa_raw_encrypt(data: &[u8; 128]) -> Vec<u8> {
    let n = BigUint::from_str_radix(RSA_N_HEX, 16).expect("hardcoded RSA n is valid hex");
    let e = BigUint::from(RSA_E);
    let m = BigUint::from_bytes_be(data);
    let c = m.modpow(&e, &n);
    let bytes = c.to_bytes_be();
    // Ensure exactly 128 bytes (left-pad with zeros if needed)
    if bytes.len() >= 128 {
        bytes[bytes.len() - 128..].to_vec()
    } else {
        let mut padded = vec![0u8; 128 - bytes.len()];
        padded.extend_from_slice(&bytes);
        padded
    }
}

// ── Random utilities ──

fn random_bytes_16() -> [u8; 16] {
    let mut buf = [0u8; 16];
    rand::thread_rng().fill(&mut buf);
    buf
}

fn base62_map(bytes: &[u8; 16]) -> [u8; 16] {
    let mut result = [0u8; 16];
    for (i, &b) in bytes.iter().enumerate() {
        result[i] = BASE62[(b as usize) % 62];
    }
    result
}

pub fn random_hex(n: usize) -> String {
    let bytes: Vec<u8> = (0..n).map(|_| rand::thread_rng().gen()).collect();
    hex::encode(bytes)
}

// ── Public API ──

/// weapi encrypt: returns (params, enc_sec_key)
pub fn weapi(json_text: &str) -> (String, String) {
    let secret_key = base62_map(&random_bytes_16());

    // Inner: AES-128-CBC with preset key
    let inner_encrypted = aes128_cbc_encrypt(PRESET_KEY, IV, json_text.as_bytes());
    let inner_b64 = base64::engine::general_purpose::STANDARD.encode(&inner_encrypted);

    // Outer: AES-128-CBC with random secret key, encrypting the base64 string
    let outer_encrypted = aes128_cbc_encrypt(&secret_key, IV, inner_b64.as_bytes());
    let params = base64::engine::general_purpose::STANDARD.encode(&outer_encrypted);

    // RSA encrypt the reversed secret key
    let mut reversed_key = secret_key;
    reversed_key.reverse();
    let mut padded_key = [0u8; 128];
    padded_key[128 - reversed_key.len()..].copy_from_slice(&reversed_key);
    let enc_sec_key = hex::encode(rsa_raw_encrypt(&padded_key));

    (params, enc_sec_key)
}

/// eapi encrypt: returns params hex string (uppercase)
pub fn eapi(url_path: &str, json_text: &str) -> String {
    // MD5 digest: nobody{url}use{json}md5forencrypt
    let digest_input = format!("nobody{}use{}md5forencrypt", url_path, json_text);
    use md5::{Digest, Md5};
    let digest = Md5::digest(digest_input.as_bytes());
    let digest_hex = format!("{:x}", digest);

    // Construct: url + "-36cd479b6b5-" + json + "-36cd479b6b5-" + digest
    let data = format!(
        "{}-36cd479b6b5-{}-36cd479b6b5-{}",
        url_path, json_text, digest_hex
    );

    let encrypted = aes128_ecb_encrypt(EAPI_KEY, data.as_bytes());
    hex::encode_upper(encrypted)
}

/// eapi decrypt: decrypt response body (AES-128-ECB with eapi key)
pub fn eapi_decrypt(data: &[u8]) -> Vec<u8> {
    aes128_ecb_decrypt(EAPI_KEY, data)
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkcs7_roundtrip() {
        let input = b"hello world";
        let padded = pkcs7_pad(input);
        let unpadded = pkcs7_unpad(&padded);
        assert_eq!(unpadded, input);
    }

    #[test]
    fn test_ecb_roundtrip() {
        let input = b"hello world test message for ecb mode!";
        let encrypted = aes128_ecb_encrypt(EAPI_KEY, input);
        let decrypted = aes128_ecb_decrypt(EAPI_KEY, &encrypted);
        assert_eq!(&decrypted, input);
    }

    #[test]
    fn test_cbc_roundtrip() {
        let input = b"test data for cbc encryption mode test";
        let encrypted = aes128_cbc_encrypt(PRESET_KEY, IV, input);
        // Verify we got data back that's different from input (encryption worked)
        assert_ne!(&encrypted, input);
        assert_eq!(encrypted.len() % 16, 0);
    }

    #[test]
    fn test_weapi_output_shape() {
        let (params, enc_sec_key) = weapi(r#"{"test":"value"}"#);
        assert!(!params.is_empty());
        assert!(!enc_sec_key.is_empty());
        // encSecKey should be 128 bytes = 256 hex chars
        assert_eq!(enc_sec_key.len(), 256);
    }

    #[test]
    fn test_eapi_output_shape() {
        let params = eapi("/api/test", r#"{"test":"value"}"#);
        assert!(!params.is_empty());
        // Should be uppercase hex
        assert!(params.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_eapi_decrypt_of_encrypted() {
        // eapi_decrypt should reverse aes128_ecb_encrypt
        let input = b"test response data for eapi decrypt";
        let encrypted = aes128_ecb_encrypt(EAPI_KEY, input);
        let decrypted = eapi_decrypt(&encrypted);
        assert_eq!(&decrypted, input);
    }
}
