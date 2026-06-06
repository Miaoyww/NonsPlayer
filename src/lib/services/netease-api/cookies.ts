import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
import CryptoJS from "crypto-js";

// ── Cookie storage ─────────────────────────────────────────────────────

const COOKIE_STORAGE_KEY = "netease_cookie";
const DEVICE_ID_STORAGE_KEY = "netease_device_id";

let _cookieJar: Record<string, string> = tryLoadCookieJar();

// ── OS defaults (matching the pc profile in request.js) ────────────────

export const OS_DEFAULTS = {
  os: "pc",
  appver: "3.1.17.204416",
  osver: "Microsoft-Windows-10-Professional-build-19045-64bit",
  channel: "netease",
  versioncode: "140",
  mobilename: "",
  resolution: "1920x1080",
};

// ── Cached values (computed once at module level) ──────────────────────

const WNMCID = (() => {
  const chars = "abcdefghijklmnopqrstuvwxyz";
  let randomString = "";
  for (let i = 0; i < 6; i++) {
    randomString += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return `${randomString}.${Date.now().toString()}.01.0`;
})();

let _anonymousToken: string | null = null;

/** Reactive store that components can subscribe to for login/logout changes. */
export const neteaseAuth = writable({ loggedIn: isLoggedIn() });

// ── Public helpers ─────────────────────────────────────────────────────

export function isLoggedIn(): boolean {
  return "MUSIC_U" in _cookieJar;
}

/** Get the cookie string (for backwards compat with mapAccount, etc.). */
export function getCookie(): string {
  return jarToString(_cookieJar);
}

/** Get a copy of the current cookie jar (for request building). */
export function getCookieJar(): Record<string, string> {
  return { ..._cookieJar };
}

// ── Device ID ──────────────────────────────────────────────────────────

/** Get or generate a persistent device ID (stored in localStorage). */
export function getDeviceId(): string {
  try {
    if (typeof localStorage !== "undefined") {
      const existing = localStorage.getItem(DEVICE_ID_STORAGE_KEY);
      if (existing) return existing;
    }
  } catch { /* SSR */ }
  // Generate a new device ID (52 uppercase hex chars, matching the Node.js version)
  const hexChars = "0123456789ABCDEF";
  let deviceId = "";
  for (let i = 0; i < 52; i++) {
    deviceId += hexChars.charAt(Math.floor(Math.random() * hexChars.length));
  }
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(DEVICE_ID_STORAGE_KEY, deviceId);
    }
  } catch { /* SSR */ }
  return deviceId;
}

// ── Anonymous token ────────────────────────────────────────────────────

/** Get or generate a cached anonymous token (32-byte hex). */
export function getAnonymousToken(): string {
  if (!_anonymousToken) {
    _anonymousToken = CryptoJS.lib.WordArray.random(32).toString();
  }
  return _anonymousToken;
}

// ── Request ID ─────────────────────────────────────────────────────────

/** Generate a request ID: `{timestamp}_{random4digits}` */
export function generateRequestId(): string {
  return `${Date.now()}_${Math.floor(Math.random() * 1000).toString().padStart(4, "0")}`;
}

// ── Cookie processing (ported from request.js) ─────────────────────────

/**
 * Merge OS defaults and generated fields into a cookie object.
 * Roughly mirrors `processCookieObject()` in the reference request.js.
 */
export function processCookieObject(
  cookie: Record<string, string>,
  uri: string,
): Record<string, string> {
  const _ntes_nuid = CryptoJS.lib.WordArray.random(32).toString();
  const os = OS_DEFAULTS;

  const result: Record<string, string> = {
    ...cookie,
    __remember_me: "true",
    ntes_kaola_ad: "1",
    _ntes_nuid: cookie._ntes_nuid || _ntes_nuid,
    _ntes_nnid: cookie._ntes_nnid || `${_ntes_nuid},${Date.now().toString()}`,
    WNMCID: cookie.WNMCID || WNMCID,
    WEVNSM: cookie.WEVNSM || "1.0.0",
    osver: cookie.osver || os.osver,
    deviceId: cookie.deviceId || getDeviceId(),
    os: cookie.os || os.os,
    channel: cookie.channel || os.channel,
    appver: cookie.appver || os.appver,
  };

  if (uri.indexOf("login") === -1) {
    result["NMTID"] = CryptoJS.lib.WordArray.random(16).toString();
  }

  if (!result.MUSIC_U) {
    result.MUSIC_A = result.MUSIC_A || getAnonymousToken();
  }

  return result;
}

/**
 * Create a Cookie header string from a header object.
 * Uses encodeURIComponent on both keys and values.
 */
export function createHeaderCookie(header: Record<string, string>): string {
  const keys = Object.keys(header);
  const parts: string[] = [];
  for (let i = 0; i < keys.length; i++) {
    const key = keys[i];
    parts[i] = encodeURIComponent(key) + "=" + encodeURIComponent(header[key]);
  }
  return parts.join("; ");
}

/**
 * Build the eapi/api header object from cookie data.
 * This header object is both used for the Cookie header AND embedded in the
 * encrypted eapi request body as `data.header`.
 */
export function buildEapiHeader(
  cookie: Record<string, string>,
  csrfToken: string,
  extra: Record<string, string> = {},
): Record<string, string> {
  const header: Record<string, string> = {
    osver: cookie.osver || OS_DEFAULTS.osver,
    deviceId: cookie.deviceId || getDeviceId(),
    os: cookie.os || OS_DEFAULTS.os,
    appver: cookie.appver || OS_DEFAULTS.appver,
    versioncode: cookie.versioncode || OS_DEFAULTS.versioncode,
    mobilename: cookie.mobilename || OS_DEFAULTS.mobilename,
    buildver: cookie.buildver || Date.now().toString().substring(0, 10),
    resolution: cookie.resolution || OS_DEFAULTS.resolution,
    __csrf: csrfToken,
    channel: cookie.channel || OS_DEFAULTS.channel,
    requestId: generateRequestId(),
    ...extra,
  };

  if (cookie.MUSIC_U) header["MUSIC_U"] = cookie.MUSIC_U;
  if (cookie.MUSIC_A) header["MUSIC_A"] = cookie.MUSIC_A;

  return header;
}

// ── Internal jar ↔ string conversion ──────────────────────────────────

export function jarToString(jar: Record<string, string>): string {
  return Object.entries(jar)
    .map(([k, v]) => `${k}=${v}`)
    .join("; ");
}

function stringToJar(s: string): Record<string, string> {
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

function tryLoadCookieJar(): Record<string, string> {
  try {
    if (typeof localStorage !== "undefined") {
      const raw = localStorage.getItem(COOKIE_STORAGE_KEY);
      if (raw) {
        try {
          const parsed = JSON.parse(raw);
          if (typeof parsed === "object" && parsed !== null) return parsed;
        } catch {
          // Old format: "key=val; key=val" — migrate to JSON
          const jar = stringToJar(raw);
          if (Object.keys(jar).length > 0) {
            localStorage.setItem(COOKIE_STORAGE_KEY, JSON.stringify(jar));
            return jar;
          }
        }
      }
    }
  } catch { /* SSR */ }
  return {};
}

function persistCookieJar(jar: Record<string, string>): void {
  try {
    if (typeof localStorage !== "undefined") {
      if (Object.keys(jar).length > 0) {
        localStorage.setItem(COOKIE_STORAGE_KEY, JSON.stringify(jar));
      } else {
        localStorage.removeItem(COOKIE_STORAGE_KEY);
      }
    }
  } catch { /* SSR */ }
}

export function setCookieJar(jar: Record<string, string>): void {
  const wasLoggedIn = isLoggedIn();
  _cookieJar = jar;
  persistCookieJar(jar);
  const nowLoggedIn = isLoggedIn();
  if (wasLoggedIn !== nowLoggedIn) {
    console.log("[netease-api] auth changed | wasLoggedIn:", wasLoggedIn, "nowLoggedIn:", nowLoggedIn);
    neteaseAuth.set({ loggedIn: nowLoggedIn });
  }
}

// ── Cookie sync ────────────────────────────────────────────────────────

/**
 * Initialize the cookie state. Called at app startup.
 * Syncs cookies from localStorage to Rust, then refreshes the session.
 */
export async function initCookies(): Promise<void> {
  if (Object.keys(_cookieJar).length > 0) {
    try {
      await invoke("netease_set_cookies", { cookies: JSON.stringify(_cookieJar) });
    } catch (e) {
      console.log("[netease-api] initCookies | set_cookies failed:", e);
    }
  }
  try {
    const rustCookies = await invoke<string>("netease_get_cookies");
    if (rustCookies) {
      const jar: Record<string, string> = JSON.parse(rustCookies);
      if (Object.keys(jar).length > 0) {
        setCookieJar(jar);
        console.log("[netease-api] initCookies | synced from Rust, MUSIC_U:", isLoggedIn());
      }
    }
  } catch (e) {
    console.log("[netease-api] initCookies | get_cookies failed:", e);
  }
}
