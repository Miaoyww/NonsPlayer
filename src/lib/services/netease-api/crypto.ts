/**
 * Netease Cloud Music API encryption — browser-compatible port of
 * the reference `D:/Projects/Web/api-enhanced/util/crypto.js`.
 *
 * Uses `crypto-js` for AES / MD5 and `BigInt` for RSA-1024 raw encryption.
 * No Node.js built-ins — runs in the Tauri webview.
 */
import CryptoJS from "crypto-js";

// ── Constants (identical to crypto.js) ──────────────────────────────────

const IV = "0102030405060708";
const PRESET_KEY = "0CoJUm6Qyw8W8jud";
const EAPI_KEY = "e82ckenh8dichen8";
const BASE62 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

// ── RSA public key (1024-bit, same as crypto.js) ────────────────────────
// Modulus n was extracted from the DER-encoded SPKI PEM.
// Exponent e = 65537 (standard).

const RSA_N = BigInt(
  "0xe0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a56aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddebd2741d546b8e289dc6935b3ece0462db0a22b8e7",
);
const RSA_E = 65537n;

// ── AES helpers (ported 1:1 from crypto.js) ─────────────────────────────

/**
 * AES encrypt — mirrors `aesEncrypt()` in crypto.js.
 *
 * @param text   plaintext
 * @param mode   "cbc" or "ecb"
 * @param key    AES key (UTF-8 string)
 * @param iv     IV (UTF-8 string, ignored in ECB mode but still parsed)
 * @param format "base64" → encrypted.toString(); otherwise → ciphertext hex (uppercase)
 */
function aesEncrypt(
  text: string,
  mode: "cbc" | "ecb",
  key: string,
  iv: string,
  format: "base64" | "hex" = "base64",
): string {
  const encrypted = CryptoJS.AES.encrypt(
    CryptoJS.enc.Utf8.parse(text),
    CryptoJS.enc.Utf8.parse(key),
    {
      iv: CryptoJS.enc.Utf8.parse(iv),
      mode: CryptoJS.mode[mode.toUpperCase() as "CBC" | "ECB"],
      padding: CryptoJS.pad.Pkcs7,
    },
  );

  if (format === "base64") {
    return encrypted.toString();
  }
  // hex: raw ciphertext as uppercase hex
  return encrypted.ciphertext.toString().toUpperCase();
}

/**
 * AES decrypt — mirrors `aesDecrypt()` in crypto.js.
 *
 * @param ciphertext  ciphertext (base64 string or hex string)
 * @param key         AES key (UTF-8 string)
 * @param iv          IV (UTF-8 string, ignored in ECB mode but still parsed)
 * @param format      "base64" or "hex" — format of `ciphertext`
 */
function aesDecrypt(
  ciphertext: string,
  key: string,
  iv: string,
  format: "base64" | "hex" = "base64",
): CryptoJS.lib.WordArray {
  if (format === "base64") {
    return CryptoJS.AES.decrypt(ciphertext, CryptoJS.enc.Utf8.parse(key), {
      iv: CryptoJS.enc.Utf8.parse(iv),
      mode: CryptoJS.mode.ECB,
      padding: CryptoJS.pad.Pkcs7,
    });
  }
  // hex format — cast needed: { ciphertext } is a partial CipherParams
  return CryptoJS.AES.decrypt(
    { ciphertext: CryptoJS.enc.Hex.parse(ciphertext) } as any,
    CryptoJS.enc.Utf8.parse(key),
    {
      iv: CryptoJS.enc.Utf8.parse(iv),
      mode: CryptoJS.mode.ECB,
      padding: CryptoJS.pad.Pkcs7,
    },
  );
}

// ── RSA raw encryption (BigInt — no forge needed) ───────────────────────

/** Modular exponentiation: base^exp % mod */
function modPow(base: bigint, exp: bigint, mod: bigint): bigint {
  let result = 1n;
  base = base % mod;
  while (exp > 0n) {
    if (exp & 1n) result = (result * base) % mod;
    exp >>= 1n;
    base = (base * base) % mod;
  }
  return result;
}

/**
 * Raw RSA-1024 encryption (no padding).
 * Mirrors `rsaEncrypt()` in crypto.js which uses forge with 'NONE' encoding.
 *
 * Forge left-pads the input with zero bytes to reach the key size (128 bytes).
 * We do the same: `data` (any length) is right-aligned in a 128-byte zero array.
 */
function rsaEncrypt(data: Uint8Array): string {
  // Build 128-byte big-endian integer (left zero-padded by right-aligning)
  const buf = new Uint8Array(128);
  buf.set(data, 128 - data.length);

  // Convert bytes → BigInt (big-endian)
  let m = 0n;
  for (let i = 0; i < 128; i++) {
    m = (m << 8n) | BigInt(buf[i]);
  }

  const c = modPow(m, RSA_E, RSA_N);

  // hex output, always 256 chars (128 bytes), matching forge.util.bytesToHex
  return c.toString(16).padStart(256, "0");
}

// ── URL encoding ────────────────────────────────────────────────────────

function urlEncode(s: string): string {
  let result = "";
  for (let i = 0; i < s.length; i++) {
    const c = s.charAt(i);
    if (/[a-zA-Z0-9\-_.~]/.test(c)) {
      result += c;
    } else {
      result += "%" + c.charCodeAt(0).toString(16).toUpperCase().padStart(2, "0");
    }
  }
  return result;
}

// ── Public API ──────────────────────────────────────────────────────────

export interface WeapiResult {
  params: string;
  encSecKey: string;
}

export interface EapiResult {
  params: string;
}

/**
 * Weapi encryption — identical to `weapi()` in crypto.js.
 *
 * 1. AES-128-CBC with fixed key & IV
 * 2. AES-128-CBC with random 16-char base62 key & fixed IV
 * 3. RSA-1024 raw encrypt the reversed random key
 */
export function weapi(object: Record<string, unknown>): WeapiResult {
  const text = JSON.stringify(object);

  // Random 16-char base62 key (match Math.round distribution from original)
  let secretKey = "";
  for (let i = 0; i < 16; i++) {
    secretKey += BASE62.charAt(Math.round(Math.random() * 61));
  }

  const params = aesEncrypt(
    aesEncrypt(text, "cbc", PRESET_KEY, IV, "base64"),
    "cbc",
    secretKey,
    IV,
    "base64",
  );

  const reversed = secretKey.split("").reverse().join("");
  const encSecKey = rsaEncrypt(new TextEncoder().encode(reversed));

  return { params, encSecKey };
}

/**
 * Eapi encryption — identical to `eapi()` in crypto.js.
 */
export function eapi(url: string, object: Record<string, unknown>): EapiResult {
  const text = JSON.stringify(object);
  const message = `nobody${url}use${text}md5forencrypt`;
  const digest = CryptoJS.MD5(message).toString();
  const data = `${url}-36cd479b6b5-${text}-36cd479b6b5-${digest}`;

  return {
    params: aesEncrypt(data, "ecb", EAPI_KEY, "" /* ECB ignores IV */, "hex"),
  };
}

/**
 * Eapi response decryption — identical to `eapiResDecrypt()` in crypto.js.
 *
 * AES-128-ECB decrypt from hex.  If the body is gzip-compressed the sync
 * path returns null (browser can't gunzip synchronously); callers should
 * fall back to `eapiResDecryptAsync`.
 */
export function eapiResDecrypt(hexBody: string): any {
  try {
    const decrypted = aesDecrypt(hexBody, EAPI_KEY, "", "hex");

    // Check for gzip magic bytes (only when x-aeapi header was sent)
    const hexCheck = decrypted.toString(CryptoJS.enc.Hex);
    if (hexCheck.length >= 4 && hexCheck.substring(0, 4).toUpperCase() === "1F8B") {
      // gzip path — can't decompress synchronously in browser, return null
      // so the caller knows to try the async version
      return null;
    }

    // Plain UTF-8
    const text = decrypted.toString(CryptoJS.enc.Utf8);
    console.log("[crypto] eapiResDecrypt decrypted:", text);
    return JSON.parse(text);
  } catch (error) {
    console.log("[crypto] eapiResDecrypt error:", error, "hexBody:", hexBody.substring(0, 200));
    return null;
  }
}

/**
 * Async version that also handles gzip-compressed eapi responses.
 */
export async function eapiResDecryptAsync(hexBody: string): Promise<any> {
  try {
    const decrypted = aesDecrypt(hexBody, EAPI_KEY, "", "hex");

    // Convert WordArray → Uint8Array
    const decHex = decrypted.toString(CryptoJS.enc.Hex);
    const bytes = new Uint8Array(decHex.length / 2);
    for (let i = 0; i < decHex.length; i += 2) {
      bytes[i / 2] = parseInt(decHex.substring(i, i + 2), 16);
    }

    // gzip magic bytes?
    if (bytes.length >= 2 && bytes[0] === 0x1f && bytes[1] === 0x8b) {
      const ds = new DecompressionStream("gzip");
      const writer = ds.writable.getWriter();
      writer.write(bytes);
      writer.close();
      const decompressed = await new Response(ds.readable).arrayBuffer();
      return JSON.parse(new TextDecoder().decode(decompressed));
    }

    return JSON.parse(decrypted.toString(CryptoJS.enc.Utf8));
  } catch (error) {
    console.log("[crypto] eapiResDecryptAsync error:", error);
    return null;
  }
}

// ── Form helpers ────────────────────────────────────────────────────────

export function encodeWeapiForm(p: WeapiResult): string {
  return `params=${urlEncode(p.params)}&encSecKey=${urlEncode(p.encSecKey)}`;
}

export function encodeEapiForm(p: EapiResult): string {
  return `params=${urlEncode(p.params)}`;
}
