import {
  aesGcmEncrypt,
  aesGcmDecrypt,
  aesCbcEncrypt,
  aesCbcDecrypt,
  hkdfSha256,
  sha256,
  hmacSha256,
  md5,
  rsaesPkcs1Encrypt as rustRsaEncrypt
} from 'nons:crypto/primitives@1.0.0'
import { Buffer } from 'buffer/'
const nativeCrypto = Reflect.get(globalThis, 'crypto')
export const bytes = (value) =>
  value instanceof ArrayBuffer
    ? new Uint8Array(value)
    : new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
const copy = (value) => new Uint8Array(bytes(value))
const name = (algorithm) =>
  (typeof algorithm === 'string' ? algorithm : algorithm.name).toUpperCase()
const keys = new WeakMap()
const subtle = {
  async importKey(format, data, algorithm, extractable, usages) {
    if (format !== 'raw' || extractable) throw new Error('Unsupported key import')
    const algorithmName = name(algorithm)
    if (!['AES-GCM', 'AES-CBC', 'HKDF', 'HMAC'].includes(algorithmName))
      throw new Error('Unsupported crypto algorithm')
    if (algorithmName === 'HMAC' && name(algorithm.hash) !== 'SHA-256')
      throw new Error('Unsupported HMAC hash')
    const key = Object.freeze({
      algorithm: { name: algorithmName },
      extractable: false,
      usages: [...usages]
    })
    keys.set(key, copy(data))
    return key
  },
  async digest(algorithm, data) {
    if (name(algorithm) !== 'SHA-256') throw new Error('Unsupported digest')
    return sha256(bytes(data)).buffer
  },
  async sign(algorithm, key, data) {
    if (name(algorithm) !== 'HMAC' || !key.usages.includes('sign') || key.algorithm.name !== 'HMAC')
      throw new Error('Invalid crypto usage')
    return hmacSha256(keys.get(key), bytes(data)).buffer
  },
  async encrypt(algorithm, key, data) {
    return transform(algorithm, key, data, false)
  },
  async decrypt(algorithm, key, data) {
    return transform(algorithm, key, data, true)
  },
  async deriveBits(algorithm, key, length) {
    if (
      name(algorithm) !== 'HKDF' ||
      name(algorithm.hash) !== 'SHA-256' ||
      key.algorithm.name !== 'HKDF' ||
      !key.usages.includes('deriveBits') ||
      length % 8
    )
      throw new Error('Invalid crypto derivation')
    return hkdfSha256(keys.get(key), bytes(algorithm.salt), bytes(algorithm.info), length / 8)
      .buffer
  }
}
function transform(algorithm, key, data, decrypt) {
  const algorithmName = name(algorithm)
  if (key.algorithm.name !== algorithmName || !key.usages.includes(decrypt ? 'decrypt' : 'encrypt'))
    throw new Error('Invalid crypto usage')
  if (algorithmName === 'AES-GCM') {
    if (algorithm.tagLength !== undefined && algorithm.tagLength !== 128)
      throw new Error('Unsupported GCM tag length')
    return (decrypt ? aesGcmDecrypt : aesGcmEncrypt)(
      keys.get(key),
      bytes(algorithm.iv),
      bytes(data),
      algorithm.additionalData ? bytes(algorithm.additionalData) : new Uint8Array()
    ).buffer
  }
  if (algorithmName === 'AES-CBC')
    return (decrypt ? aesCbcDecrypt : aesCbcEncrypt)(
      keys.get(key),
      bytes(algorithm.iv),
      bytes(data)
    ).buffer
  throw new Error('Unsupported crypto algorithm')
}
export const runtimeCrypto = {
  subtle,
  getRandomValues: nativeCrypto.getRandomValues.bind(nativeCrypto),
  randomUUID: nativeCrypto.randomUUID.bind(nativeCrypto)
}
export const protocolMd5 = (...values) =>
  Buffer.from(md5(new TextEncoder().encode(values.join('')))).toString('hex')
export function protocolRsaEncrypt(message, modulusHex, exponent) {
  const entropy = nativeCrypto.getRandomValues(new Uint8Array(32))
  try {
    return Buffer.from(
      rustRsaEncrypt(Buffer.from(modulusHex, 'hex'), Number(exponent), bytes(message), entropy)
    )
  } finally {
    entropy.fill(0)
  }
}
