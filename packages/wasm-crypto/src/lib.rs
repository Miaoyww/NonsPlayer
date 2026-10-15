//! Platform-neutral guest primitives; no Host, network, clock or random imports.
use aes_gcm::{
    aead::{Aead, Payload},
    Aes128Gcm, Aes256Gcm, KeyInit, Nonce,
};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

wit_bindgen::generate!({ world: "crypto" });
struct Crypto;
type Bytes = Result<Vec<u8>, String>;
const MAX_INPUT: usize = 64 * 1024;
fn bounded(values: &[&[u8]]) -> Result<(), String> {
    if values
        .iter()
        .try_fold(0usize, |sum, value| sum.checked_add(value.len()))
        .is_none_or(|size| size > MAX_INPUT)
    {
        return Err("inputTooLarge".into());
    }
    Ok(())
}
fn gcm(key: Vec<u8>, nonce: Vec<u8>, data: Vec<u8>, aad: Vec<u8>, decrypt: bool) -> Bytes {
    let key = Zeroizing::new(key);
    bounded(&[&key, &nonce, &data, &aad])?;
    if nonce.len() != 12 {
        return Err("invalidNonce".into());
    }
    let payload = Payload {
        msg: &data,
        aad: &aad,
    };
    let nonce = Nonce::from_slice(&nonce);
    let result = match key.len() {
        16 => {
            let cipher = Aes128Gcm::new_from_slice(&key).map_err(|_| "invalidKey")?;
            if decrypt {
                cipher.decrypt(nonce, payload)
            } else {
                cipher.encrypt(nonce, payload)
            }
        }
        32 => {
            let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "invalidKey")?;
            if decrypt {
                cipher.decrypt(nonce, payload)
            } else {
                cipher.encrypt(nonce, payload)
            }
        }
        _ => return Err("invalidKey".into()),
    };
    result.map_err(|_| "cryptoFailed".into())
}
fn cbc(key: Vec<u8>, iv: Vec<u8>, data: Vec<u8>, decrypt: bool) -> Bytes {
    let key = Zeroizing::new(key);
    bounded(&[&key, &iv, &data])?;
    if iv.len() != 16 {
        return Err("invalidIv".into());
    }
    macro_rules! operate {
        ($aes:ty) => {
            if decrypt {
                cbc::Decryptor::<$aes>::new_from_slices(&key, &iv)
                    .map_err(|_| "invalidKey")?
                    .decrypt_padded_vec_mut::<Pkcs7>(&data)
                    .map_err(|_| "cryptoFailed".into())
            } else {
                Ok(cbc::Encryptor::<$aes>::new_from_slices(&key, &iv)
                    .map_err(|_| "invalidKey")?
                    .encrypt_padded_vec_mut::<Pkcs7>(&data))
            }
        };
    }
    match key.len() {
        16 => operate!(aes::Aes128),
        32 => operate!(aes::Aes256),
        _ => Err("invalidKey".into()),
    }
}
impl exports::nons::crypto::primitives::Guest for Crypto {
    fn md5(data: Vec<u8>) -> Bytes {
        bounded(&[&data])?;
        Ok(md5::Md5::digest(data).to_vec())
    }
    fn rsaes_pkcs1_encrypt(
        modulus: Vec<u8>,
        exponent: u32,
        data: Vec<u8>,
        entropy: Vec<u8>,
    ) -> Bytes {
        use rand_core::SeedableRng;
        let entropy = Zeroizing::new(entropy);
        let data = Zeroizing::new(data);
        bounded(&[&modulus, &data, &entropy])?;
        if !(128..=512).contains(&modulus.len())
            || entropy.len() != 32
            || data.len() > modulus.len().saturating_sub(11)
        {
            return Err("invalidRsaParameters".into());
        }
        let key = rsa::RsaPublicKey::new(
            rsa::BigUint::from_bytes_be(&modulus),
            rsa::BigUint::from(exponent),
        )
        .map_err(|_| "invalidKey")?;
        let seed: [u8; 32] = entropy
            .as_slice()
            .try_into()
            .map_err(|_| "invalidEntropy")?;
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(seed);
        key.encrypt(&mut rng, rsa::Pkcs1v15Encrypt, &data)
            .map_err(|_| "cryptoFailed".into())
    }
    fn aes_gcm_encrypt(key: Vec<u8>, nonce: Vec<u8>, data: Vec<u8>, aad: Vec<u8>) -> Bytes {
        gcm(key, nonce, data, aad, false)
    }
    fn aes_gcm_decrypt(key: Vec<u8>, nonce: Vec<u8>, data: Vec<u8>, aad: Vec<u8>) -> Bytes {
        gcm(key, nonce, data, aad, true)
    }
    fn aes_cbc_encrypt(key: Vec<u8>, iv: Vec<u8>, data: Vec<u8>) -> Bytes {
        cbc(key, iv, data, false)
    }
    fn aes_cbc_decrypt(key: Vec<u8>, iv: Vec<u8>, data: Vec<u8>) -> Bytes {
        cbc(key, iv, data, true)
    }
    fn hkdf_sha256(key: Vec<u8>, salt: Vec<u8>, info: Vec<u8>, length: u32) -> Bytes {
        let key = Zeroizing::new(key);
        bounded(&[&key, &salt, &info])?;
        if length == 0 || length > 255 * 32 {
            return Err("invalidLength".into());
        }
        let mut output = vec![0; length as usize];
        hkdf::Hkdf::<Sha256>::new(Some(&salt), &key)
            .expand(&info, &mut output)
            .map_err(|_| "invalidLength")?;
        Ok(output)
    }
    fn sha256(data: Vec<u8>) -> Bytes {
        bounded(&[&data])?;
        Ok(Sha256::digest(data).to_vec())
    }
    fn hmac_sha256(key: Vec<u8>, data: Vec<u8>) -> Bytes {
        let key = Zeroizing::new(key);
        bounded(&[&key, &data])?;
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&key).map_err(|_| "invalidKey")?;
        mac.update(&data);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}
export!(Crypto);

#[cfg(test)]
mod tests {
    use super::*;
    use exports::nons::crypto::primitives::Guest;
    fn hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn fixed_webcrypto_vectors_and_authentication() {
        let key = vec![7; 32];
        let plain = "音乐/雪".as_bytes().to_vec();
        let ciphertext =
            Crypto::aes_gcm_encrypt(key.clone(), vec![3; 12], plain.clone(), vec![]).unwrap();
        assert_eq!(
            ciphertext,
            hex("cc6110e7e3b871abe1ea430a2ca4965ce48d58908a82468963c5")
        );
        assert_eq!(
            Crypto::aes_gcm_decrypt(key.clone(), vec![3; 12], ciphertext.clone(), vec![]).unwrap(),
            plain
        );
        let mut corrupt = ciphertext;
        corrupt[0] ^= 1;
        assert!(Crypto::aes_gcm_decrypt(key.clone(), vec![3; 12], corrupt, vec![]).is_err());
        let ciphertext = Crypto::aes_cbc_encrypt(key.clone(), vec![3; 16], plain.clone()).unwrap();
        assert_eq!(ciphertext, hex("bc9c9548d11e9668cabe101e61678ef9"));
        assert_eq!(
            Crypto::aes_cbc_decrypt(key.clone(), vec![3; 16], ciphertext).unwrap(),
            plain
        );
        assert_eq!(
            Crypto::hmac_sha256(key, plain).unwrap(),
            hex("050cf187e8064c77310469924a432bbff14fb2570746762e93bdb2599f33989c")
        );
        assert_eq!(
            Crypto::sha256(b"abc".to_vec()).unwrap(),
            hex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }
    #[test]
    fn rfc5869_case_one() {
        assert_eq!(Crypto::hkdf_sha256(vec![0x0b; 22], (0..13).collect(), (0xf0..=0xf9).collect(), 42).unwrap(), hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"));
    }
    #[test]
    fn md5_and_rsa_public_key_protocol_operations() {
        use rand_core::SeedableRng;
        use rsa::traits::PublicKeyParts;
        assert_eq!(
            Crypto::md5(b"abc".to_vec()).unwrap(),
            hex("900150983cd24fb0d6963f7d28e17f72")
        );
        let mut rng = rand_chacha::ChaCha20Rng::from_seed([11; 32]);
        let key = rsa::RsaPrivateKey::new(&mut rng, 1024).unwrap();
        let data = b"public test vector".to_vec();
        let ciphertext =
            Crypto::rsaes_pkcs1_encrypt(key.n().to_bytes_be(), 65537, data.clone(), vec![12; 32])
                .unwrap();
        assert_eq!(
            key.decrypt(rsa::Pkcs1v15Encrypt, &ciphertext).unwrap(),
            data
        );
        assert!(Crypto::rsaes_pkcs1_encrypt(vec![1; 128], 65537, vec![], vec![1; 31]).is_err());
    }
    #[test]
    fn rejects_invalid_parameters_and_oversized_data() {
        assert!(Crypto::aes_gcm_encrypt(vec![7; 31], vec![3; 12], vec![], vec![]).is_err());
        assert!(Crypto::aes_gcm_encrypt(vec![7; 32], vec![3; 11], vec![], vec![]).is_err());
        assert!(Crypto::aes_cbc_encrypt(vec![7; 32], vec![3; 15], vec![]).is_err());
        assert!(Crypto::aes_cbc_decrypt(vec![7; 32], vec![3; 16], vec![1; 15]).is_err());
        assert!(Crypto::sha256(vec![0; MAX_INPUT + 1]).is_err());
        assert!(Crypto::hkdf_sha256(vec![1], vec![], vec![], 8161).is_err());
    }
}
