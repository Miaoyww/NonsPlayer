import {
  aesGcmEncrypt,
  aesGcmDecrypt,
  aesCbcEncrypt,
  aesCbcDecrypt,
  hkdfSha256,
  sha256,
  hmacSha256
} from 'nons:crypto/primitives@1.0.0'
export function initialize() {}
export function shutdown() {}
const encode = (value) => new TextEncoder().encode(value)
const hex = (value) => Array.from(value, (b) => b.toString(16).padStart(2, '0')).join('')
export function call() {
  const results = {}
  const key = new Uint8Array(32).fill(7)
  const plain = encode('音乐/雪')
  const nonce = new Uint8Array(12).fill(3)
  const iv = new Uint8Array(16).fill(3)
  const gcm = aesGcmEncrypt(key, nonce, plain, new Uint8Array())
  results.gcmVector = hex(gcm) === 'cc6110e7e3b871abe1ea430a2ca4965ce48d58908a82468963c5'
  results.gcmRoundtrip =
    new TextDecoder().decode(aesGcmDecrypt(key, nonce, gcm, new Uint8Array())) === '音乐/雪'
  const corrupted = new Uint8Array(gcm)
  corrupted[0] ^= 1
  try {
    aesGcmDecrypt(key, nonce, corrupted, new Uint8Array())
    results.gcmRejectTamper = false
  } catch {
    results.gcmRejectTamper = true
  }
  const cbc = aesCbcEncrypt(key, iv, plain)
  results.cbcVector = hex(cbc) === 'bc9c9548d11e9668cabe101e61678ef9'
  results.cbcRoundtrip = new TextDecoder().decode(aesCbcDecrypt(key, iv, cbc)) === '音乐/雪'
  results.hkdf =
    hex(
      hkdfSha256(
        new Uint8Array(22).fill(0x0b),
        Uint8Array.from({ length: 13 }, (_, i) => i),
        Uint8Array.from({ length: 10 }, (_, i) => i + 0xf0),
        42
      )
    ) === '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865'
  results.sha256 =
    hex(sha256(encode('abc'))) ===
    'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
  results.hmac =
    hex(hmacSha256(key, plain)) ===
    '050cf187e8064c77310469924a432bbff14fb2570746762e93bdb2599f33989c'
  try {
    aesGcmEncrypt(key, new Uint8Array(11), plain, new Uint8Array())
    results.rejectNonce = false
  } catch {
    results.rejectNonce = true
  }
  try {
    sha256(new Uint8Array(65537))
    results.rejectOversize = false
  } catch {
    results.rejectOversize = true
  }
  return JSON.stringify({ status: Object.values(results).every(Boolean) ? 'ok' : 'error', results })
}
