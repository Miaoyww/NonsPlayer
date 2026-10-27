use super::*;
use crate::{Event, Response};
use rsa::traits::PublicKeyParts;

fn expected(value: &str) -> Vec<u8> {
    decode_hex(value).expect("valid fixed hexadecimal fixture")
}

fn assert_error<T>(result: Result<T>, stage: &str) {
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("expected {stage}"),
    };
    assert_eq!(error.code, "invalidData");
    assert_eq!(error.stage, stage);
}

// Shared vectors also exercised by packages/wasm-crypto/src/lib.rs.
#[test]
fn fixed_md5_and_sha256_vectors() {
    assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        sha256(b"abc"),
        expected("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn fixed_hmac_sha256_vector_preserves_utf8_bytes() {
    assert_eq!(
        hmac_sha256(&[7; 32], "音乐/雪".as_bytes()).unwrap(),
        expected("050cf187e8064c77310469924a432bbff14fb2570746762e93bdb2599f33989c")
    );
}

#[test]
fn hkdf_rfc5869_case_one() {
    assert_eq!(
        hkdf_sha256(
            &[0x0b; 22],
            &(0u8..13).collect::<Vec<_>>(),
            &(0xf0u8..=0xf9).collect::<Vec<_>>(),
            42
        )
        .unwrap(),
        expected(
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        )
    );
}

#[test]
fn aes256_gcm_matches_webcrypto_vector_and_verifies_authentication() {
    let key = [7; 32];
    let nonce = [3; 12];
    let plain = "音乐/雪".as_bytes();
    let expected = expected("cc6110e7e3b871abe1ea430a2ca4965ce48d58908a82468963c5");
    assert_eq!(aes_gcm_encrypt(&key, &nonce, plain, &[]).unwrap(), expected);
    assert_eq!(
        aes_gcm_decrypt(&key, &nonce, &expected, &[]).unwrap(),
        plain
    );
    let mut corrupt = expected.clone();
    corrupt[0] ^= 1;
    assert_error(aes_gcm_decrypt(&key, &nonce, &corrupt, &[]), "crypto.gcm");
    assert_error(
        aes_gcm_decrypt(&key, &[4; 12], &expected, &[]),
        "crypto.gcm",
    );
    assert_error(
        aes_gcm_decrypt(&key, &nonce, &expected, b"changed aad"),
        "crypto.gcm",
    );
}

#[test]
fn aes128_gcm_empty_plaintext_matches_nist_tag() {
    let ciphertext = expected("58e2fccefa7e3061367f1d57a4e7455a");
    assert_eq!(
        aes_gcm_encrypt(&[0; 16], &[0; 12], &[], &[]).unwrap(),
        ciphertext
    );
    assert!(aes_gcm_decrypt(&[0; 16], &[0; 12], &ciphertext, &[])
        .unwrap()
        .is_empty());
}

#[test]
fn aes256_cbc_matches_webcrypto_pkcs7_vector() {
    let plain = "音乐/雪".as_bytes();
    let ciphertext = expected("bc9c9548d11e9668cabe101e61678ef9");
    assert_eq!(
        aes_cbc_encrypt(&[7; 32], &[3; 16], plain).unwrap(),
        ciphertext
    );
    assert_eq!(
        aes_cbc_decrypt(&[7; 32], &[3; 16], &ciphertext).unwrap(),
        plain
    );
}

#[test]
fn aes128_cbc_matches_nist_first_block_and_adds_full_padding_block() {
    let key = expected("2b7e151628aed2a6abf7158809cf4f3c");
    let iv = expected("000102030405060708090a0b0c0d0e0f");
    let plain = expected("6bc1bee22e409f96e93d7e117393172a");
    let ciphertext = aes_cbc_encrypt(&key, &iv, &plain).unwrap();
    assert_eq!(
        &ciphertext[..16],
        expected("7649abac8119b246cee98e9b12e9197d")
    );
    assert_eq!(
        ciphertext.len(),
        32,
        "a full aligned block still requires PKCS7 padding"
    );
    assert_eq!(aes_cbc_decrypt(&key, &iv, &ciphertext).unwrap(), plain);
}

#[test]
fn gcm_rejects_invalid_key_nonce_and_truncated_tag() {
    assert_error(
        aes_gcm_encrypt(&[7; 31], &[3; 12], &[], &[]),
        "crypto.gcm-key",
    );
    assert_error(
        aes_gcm_encrypt(&[7; 32], &[3; 11], &[], &[]),
        "crypto.gcm-nonce",
    );
    assert_error(
        aes_gcm_decrypt(&[7; 32], &[3; 13], &[], &[]),
        "crypto.gcm-nonce",
    );
    assert_error(
        aes_gcm_decrypt(&[7; 32], &[3; 12], &[0; 15], &[]),
        "crypto.gcm",
    );
}

#[test]
fn cbc_rejects_invalid_key_iv_and_padding() {
    assert_error(aes_cbc_encrypt(&[7; 24], &[3; 16], &[]), "crypto.cbc-key");
    assert_error(aes_cbc_encrypt(&[7; 32], &[3; 15], &[]), "crypto.cbc-iv");
    assert_error(aes_cbc_decrypt(&[7; 32], &[3; 16], &[1; 15]), "crypto.cbc");
    let mut ciphertext = aes_cbc_encrypt(&[7; 32], &[3; 16], &[0; 16]).unwrap();
    // The last byte of the penultimate CBC block flips the padding length 0x10 to 0x00.
    ciphertext[15] ^= 0x10;
    assert_error(
        aes_cbc_decrypt(&[7; 32], &[3; 16], &ciphertext),
        "crypto.cbc",
    );
}

#[test]
fn bounded_primitives_enforce_combined_input_and_hkdf_output_limits() {
    let oversized = vec![0; MAX_INPUT + 1];
    assert_error(hmac_sha256(&[], &oversized), "crypto.input-size");
    assert_error(hmac_sha256(&[1], &vec![0; MAX_INPUT]), "crypto.input-size");
    assert_error(hkdf_sha256(&[1], &[], &oversized, 32), "crypto.input-size");
    assert_error(hkdf_sha256(&[1], &[], &[], 0), "crypto.hkdf-length");
    assert_error(hkdf_sha256(&[1], &[], &[], 8161), "crypto.hkdf-length");
    assert_eq!(hkdf_sha256(&[1], &[], &[], 8160).unwrap().len(), 8160);
    assert_error(
        aes_cbc_encrypt(&[7; 32], &[3; 16], &oversized),
        "crypto.input-size",
    );
    assert_error(
        aes_gcm_encrypt(&[7; 32], &[3; 12], &[], &vec![0; MAX_INPUT]),
        "crypto.input-size",
    );
}

#[test]
fn protocol_hex_and_base64_encoding_have_fixed_vectors_and_reject_invalid_text() {
    assert_eq!(hex(&[0, 15, 128, 255]), "000f80ff");
    assert_eq!(decode_hex("000F80fF").unwrap(), [0, 15, 128, 255]);
    assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    assert_eq!(decode_b64("Zm9vYmFy").unwrap(), b"foobar");
    for value in ["0", "gg", "é"] {
        assert_error(decode_hex(value), "crypto.hex");
    }
    assert_error(decode_hex(&"00".repeat(MAX_INPUT + 1)), "crypto.hex");
    assert_error(decode_b64("!!!!"), "crypto.base64");
    assert_error(
        decode_b64(&"A".repeat((MAX_INPUT + 2) / 3 * 4 + 1)),
        "crypto.base64-size",
    );
}

struct EntropyHost {
    length: usize,
    fails: bool,
}
impl Host for EntropyHost {
    fn http(
        &self,
        _: &str,
        _: &str,
        _: Vec<(String, String)>,
        _: Vec<u8>,
        _: u32,
    ) -> Result<Response> {
        panic!("crypto must not access HTTP")
    }
    fn now_ms(&self) -> u64 {
        panic!("crypto must not access clocks")
    }
    fn random(&self, length: usize) -> Result<Vec<u8>> {
        assert_eq!(length, 32, "RSA requests one bounded seed from Host");
        if self.fails {
            return Err(Error::new("permissionDenied", "fixture.random"));
        }
        Ok(vec![12; self.length])
    }
    fn state_get(&self, _: &str) -> Result<Option<String>> {
        panic!("crypto must not access state")
    }
    fn state_set(&self, _: &str, _: &str) -> Result<()> {
        panic!("crypto must not access state")
    }
    fn credential_read(&self) -> Result<Vec<u8>> {
        panic!("crypto must not access credentials")
    }
    fn credential_publish(&self, _: &str, _: &[u8]) -> Result<()> {
        panic!("crypto must not access credentials")
    }
    fn credential_replace(&self, _: &[u8]) -> Result<()> {
        panic!("crypto must not access credentials")
    }
    fn credential_invalidate(&self) -> Result<()> {
        panic!("crypto must not access credentials")
    }
    fn websocket_open(&self, _: &str, _: &str, _: u32) -> Result<u32> {
        panic!("crypto must not access sockets")
    }
    fn websocket_send(&self, _: u32, _: &[u8]) -> Result<()> {
        panic!("crypto must not access sockets")
    }
    fn websocket_close(&self, _: u32) -> Result<()> {
        panic!("crypto must not access sockets")
    }
    fn next_event(&self, _: u32) -> Result<Option<Event>> {
        panic!("crypto must not access events")
    }
    fn diagnostic(&self, _: &str, _: &str, _: &str) {
        panic!("crypto must not emit diagnostics")
    }
}

#[test]
fn rsa_pkcs1_encryption_uses_host_entropy_and_private_key_can_decrypt() {
    let mut rng = rand_chacha::ChaCha20Rng::from_seed([11; 32]);
    let key = rsa::RsaPrivateKey::new(&mut rng, 1024).unwrap();
    let host = EntropyHost {
        length: 32,
        fails: false,
    };
    let data = b"public test vector";
    let ciphertext = rsaes_pkcs1_encrypt(&host, &key.n().to_bytes_be(), 65537, data).unwrap();
    assert_eq!(ciphertext.len(), 128);
    assert_eq!(
        key.decrypt(rsa::Pkcs1v15Encrypt, &ciphertext).unwrap(),
        data
    );
    assert_eq!(
        rsaes_pkcs1_encrypt(&host, &key.n().to_bytes_be(), 65537, data).unwrap(),
        ciphertext,
        "the deterministic Host seed makes the fixture reproducible"
    );
    let longest = vec![0x41; 117];
    let ciphertext = rsaes_pkcs1_encrypt(&host, &key.n().to_bytes_be(), 65537, &longest).unwrap();
    assert_eq!(
        key.decrypt(rsa::Pkcs1v15Encrypt, &ciphertext).unwrap(),
        longest
    );
}

#[test]
fn rsa_rejects_bad_parameters_and_preserves_host_entropy_failure() {
    let host = EntropyHost {
        length: 32,
        fails: false,
    };
    assert_error(
        rsaes_pkcs1_encrypt(&host, &[1; 127], 65537, &[]),
        "crypto.rsa-parameters",
    );
    assert_error(
        rsaes_pkcs1_encrypt(&host, &[1; 513], 65537, &[]),
        "crypto.rsa-parameters",
    );
    assert_error(
        rsaes_pkcs1_encrypt(&host, &[1; 128], 65537, &[0; 118]),
        "crypto.rsa-parameters",
    );
    assert_error(
        rsaes_pkcs1_encrypt(&host, &[1; 128], 0, &[]),
        "crypto.rsa-key",
    );
    let short = EntropyHost {
        length: 31,
        fails: false,
    };
    assert_error(
        rsaes_pkcs1_encrypt(&short, &[1; 128], 65537, &[]),
        "crypto.rsa-entropy",
    );
    let denied = EntropyHost {
        length: 32,
        fails: true,
    };
    let error = rsaes_pkcs1_encrypt(&denied, &[1; 128], 65537, &[]).unwrap_err();
    assert_eq!(error.code, "permissionDenied");
    assert_eq!(error.stage, "fixture.random");
}
