//! Protocol primitives use the same RustCrypto algorithms as the shared crypto component.
use crate::{Error, Host, Result};
use aes_gcm::{
    aead::{Aead, Payload},
    Aes128Gcm, Aes256Gcm, KeyInit, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use rand_core::SeedableRng;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const MAX_INPUT: usize = 64 * 1024;
fn bounded(values: &[&[u8]]) -> Result<()> {
    if values
        .iter()
        .try_fold(0usize, |sum, value| sum.checked_add(value.len()))
        .is_none_or(|size| size > MAX_INPUT)
    {
        return Err(Error::new("invalidData", "crypto.input-size"));
    }
    Ok(())
}
pub fn hex(data: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(data.len() * 2);
    for byte in data {
        value.push(DIGITS[(byte >> 4) as usize] as char);
        value.push(DIGITS[(byte & 15) as usize] as char);
    }
    value
}
pub fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if value.len() % 2 != 0 || !value.is_ascii() || value.len() > MAX_INPUT * 2 {
        return Err(Error::new("invalidData", "crypto.hex"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            };
            match (digit(pair[0]), digit(pair[1])) {
                (Some(high), Some(low)) => Ok(high * 16 + low),
                _ => Err(Error::new("invalidData", "crypto.hex")),
            }
        })
        .collect()
}
pub fn b64(data: &[u8]) -> String {
    STANDARD.encode(data)
}
pub fn decode_b64(value: &str) -> Result<Vec<u8>> {
    if value.len() > (MAX_INPUT + 2) / 3 * 4 {
        return Err(Error::new("invalidData", "crypto.base64-size"));
    }
    STANDARD
        .decode(value)
        .map_err(|_| Error::new("invalidData", "crypto.base64"))
}
pub fn md5_hex(data: &[u8]) -> String {
    hex(&md5::Md5::digest(data))
}
pub fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    bounded(&[key, data])?;
    let key = Zeroizing::new(key.to_vec());
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&key)
        .map_err(|_| Error::new("invalidData", "crypto.hmac-key"))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}
pub fn hkdf_sha256(key: &[u8], salt: &[u8], info: &[u8], length: usize) -> Result<Vec<u8>> {
    bounded(&[key, salt, info])?;
    if length == 0 || length > 255 * 32 {
        return Err(Error::new("invalidData", "crypto.hkdf-length"));
    }
    let key = Zeroizing::new(key.to_vec());
    let mut output = vec![0; length];
    hkdf::Hkdf::<Sha256>::new(Some(salt), &key)
        .expand(info, &mut output)
        .map_err(|_| Error::new("invalidData", "crypto.hkdf"))?;
    Ok(output)
}
fn gcm(key: &[u8], nonce: &[u8], data: &[u8], aad: &[u8], decrypt: bool) -> Result<Vec<u8>> {
    bounded(&[key, nonce, data, aad])?;
    if nonce.len() != 12 {
        return Err(Error::new("invalidData", "crypto.gcm-nonce"));
    }
    let key = Zeroizing::new(key.to_vec());
    let payload = Payload { msg: data, aad };
    let nonce = Nonce::from_slice(nonce);
    macro_rules! operate {
        ($cipher:ty) => {{
            let cipher = <$cipher>::new_from_slice(&key)
                .map_err(|_| Error::new("invalidData", "crypto.gcm-key"))?;
            if decrypt {
                cipher.decrypt(nonce, payload)
            } else {
                cipher.encrypt(nonce, payload)
            }
        }};
    }
    let result = match key.len() {
        16 => operate!(Aes128Gcm),
        32 => operate!(Aes256Gcm),
        _ => return Err(Error::new("invalidData", "crypto.gcm-key")),
    };
    result.map_err(|_| Error::new("invalidData", "crypto.gcm"))
}
pub fn aes_gcm_encrypt(key: &[u8], nonce: &[u8], data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    gcm(key, nonce, data, aad, false)
}
pub fn aes_gcm_decrypt(key: &[u8], nonce: &[u8], data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    gcm(key, nonce, data, aad, true)
}
fn cbc(key: &[u8], iv: &[u8], data: &[u8], decrypt: bool) -> Result<Vec<u8>> {
    bounded(&[key, iv, data])?;
    if iv.len() != 16 {
        return Err(Error::new("invalidData", "crypto.cbc-iv"));
    }
    let key = Zeroizing::new(key.to_vec());
    macro_rules! operate {
        ($aes:ty) => {
            if decrypt {
                cbc::Decryptor::<$aes>::new_from_slices(&key, iv)
                    .map_err(|_| Error::new("invalidData", "crypto.cbc-key"))?
                    .decrypt_padded_vec_mut::<Pkcs7>(data)
                    .map_err(|_| Error::new("invalidData", "crypto.cbc"))
            } else {
                Ok(cbc::Encryptor::<$aes>::new_from_slices(&key, iv)
                    .map_err(|_| Error::new("invalidData", "crypto.cbc-key"))?
                    .encrypt_padded_vec_mut::<Pkcs7>(data))
            }
        };
    }
    match key.len() {
        16 => operate!(aes::Aes128),
        32 => operate!(aes::Aes256),
        _ => Err(Error::new("invalidData", "crypto.cbc-key")),
    }
}
pub fn aes_cbc_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    cbc(key, iv, data, false)
}
pub fn aes_cbc_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    cbc(key, iv, data, true)
}
pub fn rsaes_pkcs1_encrypt(
    host: &impl Host,
    modulus: &[u8],
    exponent: u32,
    data: &[u8],
) -> Result<Vec<u8>> {
    bounded(&[modulus, data])?;
    if !(128..=512).contains(&modulus.len()) || data.len() > modulus.len().saturating_sub(11) {
        return Err(Error::new("invalidData", "crypto.rsa-parameters"));
    }
    let data = Zeroizing::new(data.to_vec());
    let entropy = Zeroizing::new(host.random(32)?);
    let seed: [u8; 32] = entropy
        .as_slice()
        .try_into()
        .map_err(|_| Error::new("invalidData", "crypto.rsa-entropy"))?;
    let key = rsa::RsaPublicKey::new(
        rsa::BigUint::from_bytes_be(modulus),
        rsa::BigUint::from(exponent),
    )
    .map_err(|_| Error::new("invalidData", "crypto.rsa-key"))?;
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(seed);
    key.encrypt(&mut rng, rsa::Pkcs1v15Encrypt, &data)
        .map_err(|_| Error::new("invalidData", "crypto.rsa"))
}

#[cfg(test)]
mod tests;
