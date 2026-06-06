/**
 * Netease API request orchestrator — ported from `D:/Projects/Web/api-enhanced/util/request.js`.
 *
 * Handles cookie processing, URL construction, encryption, header building,
 * and response decryption. Rust (`netease_tunnel`) only does the raw HTTP POST.
 */
import { invoke } from "@tauri-apps/api/core";
import {
  weapi,
  eapi,
  eapiResDecrypt,
  encodeWeapiForm,
  encodeEapiForm,
} from "./crypto";
import {
  getCookieJar,
  setCookieJar,
  processCookieObject,
  createHeaderCookie,
  generateRequestId,
  jarToString,
} from "./cookies";

// ── Constants ───────────────────────────────────────────────────────────

const WEAPI_BASE = "https://music.163.com";
const EAPI_BASE = "https://interface.music.163.com";

/** Special status codes that should be treated as 200 (from request.js). */
const SPECIAL_STATUS_CODES = new Set([201, 302, 400, 502, 800, 801, 802, 803]);

// ── User-Agent map (mirrors request.js userAgentMap) ────────────────────

const userAgentMap: Record<string, Record<string, string>> = {
  weapi: {
    pc: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edg/124.0.0.0",
  },
  linuxapi: {
    linux:
      "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
  },
  api: {
    pc: "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/3.0.18.203152",
    android:
      "NeteaseMusic/9.1.65.240927161425(9001065);Dalvik/2.1.0 (Linux; U; Android 14; 23013RK75C Build/UKQ1.230804.001)",
    iphone: "NeteaseMusic 9.0.90/5038 (iPhone; iOS 16.2; zh_CN)",
  },
};

/** Choose a User-Agent string based on crypto method and device type. */
function chooseUserAgent(crypto: string, uaType = "pc"): string {
  return (userAgentMap[crypto] && userAgentMap[crypto][uaType]) || "";
}

// ── Request options ─────────────────────────────────────────────────────

export interface NeteaseRequestOptions {
  /** Real IP to forward (sets X-Real-IP and X-Forwarded-For headers). */
  realIP?: string;
  /** Alias for realIP. */
  ip?: string;
  /** Extra headers to merge into the request. */
  headers?: Record<string, string>;
  /** Cookie override — can be a JSON object or a "key=val; key=val" string. */
  cookie?: Record<string, string> | string;
  /** Crypto method override (defaults to method parameter). */
  crypto?: string;
  /** Custom domain for the API endpoint (overrides WEAPI_BASE / EAPI_BASE). */
  domain?: string;
  /** Custom User-Agent string. */
  ua?: string;
  /** Whether to request encrypted response. */
  e_r?: boolean;
  /** Anti-cheat token for checkToken requests. */
  checkToken?: string;
}

// ── Tunnel response type ────────────────────────────────────────────────

interface TunnelResponse {
  status: number;
  /** Base64-encoded response body. */
  body: string;
  /** JSON-serialized cookie jar from Rust. */
  cookies: string;
}

// ── Internal helpers ────────────────────────────────────────────────────

/**
 * Parse param values: JSON strings → parsed values, plain strings stay as-is.
 * Mirrors the Rust behavior: `serde_json::from_str(v).unwrap_or(Value::String(v))`.
 */
function parseParams(params: Record<string, string>): Record<string, unknown> {
  const obj: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(params)) {
    try {
      obj[k] = JSON.parse(v);
    } catch {
      obj[k] = v;
    }
  }
  return obj;
}

/** Convert base64 string to Uint8Array. */
function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/** Convert Uint8Array to uppercase hex string. */
function bytesToHex(bytes: Uint8Array): string {
  let hex = "";
  for (const b of bytes) {
    hex += b.toString(16).padStart(2, "0");
  }
  return hex.toUpperCase();
}

/** Convert a boolean-ish value to boolean (mirrors request.js toBoolean). */
function toBoolean(val: unknown): boolean {
  if (typeof val === "boolean") return val;
  if (typeof val === "string") return val === "true" || val === "1";
  if (typeof val === "number") return val !== 0;
  return !!val;
}

/** Convert a cookie string "key=val; key=val" to a JSON object. */
function cookieStringToJson(s: string): Record<string, string> {
  const jar: Record<string, string> = {};
  if (!s) return jar;
  for (const part of s.split(";")) {
    const trimmed = part.trim();
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx > 0) {
      const key = trimmed.substring(0, eqIdx);
      const val = trimmed.substring(eqIdx + 1);
      if (key && val) jar[key] = val;
    }
  }
  return jar;
}

/** Build a weapi POST URL from an api-enhanced-style path.
 *  Matches request.js: `DOMAIN + '/weapi/' + uri.substr(5)`.
 *  e.g. `/api/song/enhance/player/url` → `https://music.163.com/weapi/song/enhance/player/url`
 */
function buildWeapiUrl(path: string, domain?: string): string {
  const base = domain || WEAPI_BASE;
  const sub = path.startsWith("/api/") ? path.slice(5) : path;
  return `${base}/weapi/${sub}`;
}

/** Build an eapi POST URL from an api-enhanced-style path.
 *  Matches request.js: `API_DOMAIN + '/eapi/' + uri.substr(5)`.
 *  e.g. `/api/song/lyric/v1` → `https://interface.music.163.com/eapi/song/lyric/v1`
 */
function buildEapiUrl(path: string, domain?: string): string {
  const base = domain || EAPI_BASE;
  const sub = path.startsWith("/api/") ? path.slice(5) : path;
  return `${base}/eapi/${sub}`;
}

/** Build a plain (unencrypted) api POST URL. */
function buildApiUrl(path: string, domain?: string): string {
  return (domain || EAPI_BASE) + path;
}

// ── Public API ──────────────────────────────────────────────────────────

/**
 * Send an encrypted / plain request to the Netease Cloud Music API.
 *
 * @param method  `"weapi"` | `"eapi"` | `"api"`
 * @param path    api-enhanced-style path, e.g. `"/api/v3/song/detail"`
 * @param params  key-value parameters (JSON-stringified values for arrays/objects)
 * @param options optional overrides for IP, headers, cookie, domain, UA, e_r, etc.
 * @returns Parsed JSON response
 */
export async function neteaseRequest(
  method: "weapi" | "eapi" | "api",
  path: string,
  params: Record<string, string> = {},
  options: NeteaseRequestOptions = {},
): Promise<any> {
  // ── Merge custom headers ───────────────────────────────────────────
  const headers: Record<string, string> = options.headers
    ? { ...options.headers }
    : {};

  // ── IP header handling (matching request.js) ───────────────────────
  const ip = options.realIP || options.ip || "";
  if (ip) {
    headers["X-Real-IP"] = ip;
    headers["X-Forwarded-For"] = ip;
  }

  // ── Cookie preparation ────────────────────────────────────────────
  let cookie: Record<string, string>;
  if (options.cookie) {
    if (typeof options.cookie === "string") {
      cookie = cookieStringToJson(options.cookie);
    } else {
      cookie = { ...options.cookie };
    }
  } else {
    cookie = getCookieJar();
  }

  const processedCookie = processCookieObject(cookie, path);
  const csrfToken = processedCookie["__csrf"] || "";

  // ── Build data object ────────────────────────────────────────────
  const data = parseParams(params);

  // ── e_r handling (encrypt response, from request.js) ──────────────
  data.e_r = toBoolean(
    options.e_r !== undefined
      ? options.e_r
      : (data as any).e_r !== undefined
        ? (data as any).e_r
        : false, // default: no encrypt response (APP_CONF.encryptResponse)
  );

  // ── Resolve crypto method ─────────────────────────────────────────
  const crypto = options.crypto || method;

  let url: string;
  let body: string;

  // ── Per-method: encrypt + build URL + build headers ──────────────
  switch (crypto) {
    case "weapi": {
      (data as any).csrf_token = csrfToken;

      const encrypted = weapi(data as Record<string, unknown>);

      url = buildWeapiUrl(path, options.domain);
      body = encodeWeapiForm(encrypted);

      headers["Referer"] = options.domain || WEAPI_BASE;
      headers["User-Agent"] =
        options.ua || chooseUserAgent("weapi");
      headers["Content-Type"] = "application/x-www-form-urlencoded";
      // weapi Cookie: simple key=value join matching cookieObjToString in request.js
      headers["Cookie"] = jarToString(processedCookie);
      break;
    }

    case "eapi":
    case "api": {
      // header对象构建（对齐 request.js 的逻辑）
      const header: Record<string, string> = {
        osver: processedCookie.osver,
        deviceId: processedCookie.deviceId,
        os: processedCookie.os,
        appver: processedCookie.appver,
        versioncode: processedCookie.versioncode || "140",
        mobilename: processedCookie.mobilename || "",
        buildver:
          processedCookie.buildver || Date.now().toString().substring(0, 10),
        resolution: processedCookie.resolution || "1920x1080",
        __csrf: csrfToken,
        channel: processedCookie.channel,
        requestId: generateRequestId(),
        ...(options.checkToken
          ? { "X-antiCheatToken": options.checkToken }
          : {}),
      };

      if (processedCookie.MUSIC_U) header["MUSIC_U"] = processedCookie.MUSIC_U;
      if (processedCookie.MUSIC_A) header["MUSIC_A"] = processedCookie.MUSIC_A;

      headers["Cookie"] = createHeaderCookie(header);
      headers["User-Agent"] =
        options.ua || chooseUserAgent("api", "iphone");
      headers["Content-Type"] = "application/x-www-form-urlencoded";

      if (crypto === "eapi") {
        (data as any).header = header;

        url = buildEapiUrl(path, options.domain);
        body = encodeEapiForm(
          eapi(path.slice(5), data as Record<string, unknown>),
        );
      } else {
        // api: 无加密，直接用 URLSearchParams 发送原始 data
        url = buildApiUrl(path, options.domain);
        body = new URLSearchParams(data as Record<string, string>).toString();
      }
      break;
    }

    default:
      throw new Error(`Unsupported encryption method: ${crypto}`);
  }

  // ── HTTP via Rust tunnel ─────────────────────────────────────────
  const raw: TunnelResponse = await invoke("netease_tunnel", {
    url,
    headers,
    body,
  });

  // ── Sync cookies from Rust ───────────────────────────────────────
  try {
    const rustJar: Record<string, string> = JSON.parse(raw.cookies);
    if (Object.keys(rustJar).length > 0) {
      setCookieJar(rustJar);
    }
  } catch {
    // ignore cookie parse failures
  }

  // ── Decode & parse response ──────────────────────────────────────
  const bytes = base64ToBytes(raw.body);
  const use_e_r =
    (crypto === "eapi" || crypto === "weapi") && data.e_r;

  // Try JSON first
  let text: string;
  try {
    text = new TextDecoder().decode(bytes);
  } catch {
    throw new Error(`Failed to decode response as UTF-8`);
  }

  let answer: any;

  if (use_e_r) {
    // Encrypted response — skip JSON parse, go straight to eapi decrypt
    console.log("[netease-api] trying e_r decrypt:", {
      method,
      crypto,
      path,
      status: raw.status,
      hexBody: bytesToHex(bytes).substring(0, 200),
    });
    const hexBody = bytesToHex(bytes);
    const decrypted = eapiResDecrypt(hexBody);
    if (decrypted !== null) {
      answer = decrypted;
    } else {
      throw new Error(
        `Failed to decrypt e_r response: ${text.substring(0, 300)}`,
      );
    }
  } else {
    // Plain response — try JSON first, then eapi decrypt as fallback
    try {
      answer = JSON.parse(text);
      console.log("[netease-api] JSON response:", {
        method,
        crypto,
        path,
        status: raw.status,
        answer,
      });
    } catch {
      // Not valid JSON — try eapi decryption on the hex body
      console.log("[netease-api] non-JSON response:", {
        method,
        crypto,
        path,
        status: raw.status,
        text: text.substring(0, 200),
        hexBody: bytesToHex(bytes).substring(0, 200),
      });
      const hexBody = bytesToHex(bytes);
      const decrypted = eapiResDecrypt(hexBody);
      if (decrypted !== null) {
        answer = decrypted;
      } else {
        throw new Error(
          `Unrecognized response (tried JSON and eapi decrypt): ${text.substring(0, 300)}`,
        );
      }
    }
  }

  // ── Status handling ──────────────────────────────────────────────
  if (answer && answer.code !== undefined) {
    answer.code = Number(answer.code);
  }

  let status = Number(answer?.code ?? raw.status);
  if (SPECIAL_STATUS_CODES.has(status)) {
    status = 200;
  }
  status = status > 100 && status < 600 ? status : 400;

  if (status === 200) {
    return answer;
  }

  console.log("[netease-api] request error:", {
    method,
    crypto,
    path,
    status,
    answer,
  });
  throw answer;
}
