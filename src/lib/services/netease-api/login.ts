/**
 * Login module — QR code login flow + token refresh.
 *
 * Mirror of api-enhanced modules:
 *   login_qr_key, login_qr_create, login_qr_check, login_refresh
 */

import { invoke } from "@tauri-apps/api/core";
import type { Account } from "$lib/types/account";
import { neteaseRequest } from "./request";
import { setCookieJar, jarToString, isLoggedIn, initCookies } from "./cookies";
import type { QrKeyResult } from "./types";

// ── QR code generation (lazy-loaded) ──────────────────────────────────

let _QRCodeToDataURL: ((text: string) => Promise<string>) | null = null;

async function generateQRCode(text: string): Promise<string> {
  if (!_QRCodeToDataURL) {
    try {
      const QRCode = await import("qrcode");
      _QRCodeToDataURL = (t: string) => QRCode.toDataURL(t, { width: 256, margin: 2 });
    } catch {
      console.warn("[netease-api] qrcode package not available, using external URL fallback");
      _QRCodeToDataURL = async () => "";
    }
  }
  return _QRCodeToDataURL(text);
}

// ── Login QR ──────────────────────────────────────────────────────────

export async function loginQrKey(): Promise<QrKeyResult> {
  const data = await neteaseRequest("api", "/api/login/qrcode/unikey", {
    type: "3",
  });
  return { unikey: data.unikey ?? data.data?.unikey };
}

/** Generate a QR code base64 data URL for the given key. */
export async function loginQrCreate(key: string): Promise<{ qrurl: string; qrimg: string }> {
  const qrurl = `https://music.163.com/login?codekey=${key}`;
  const qrimg = await generateQRCode(qrurl);
  return { qrurl, qrimg };
}

/**
 * Poll login status. Returns Netease status codes:
 *   800 – QR expired, 801 – waiting, 802 – scanned, 803 – confirmed
 */
export async function loginQrCheck(key: string): Promise<{ code: number; cookie?: string; message?: string }> {
  const data = await neteaseRequest("api", "/api/login/qrcode/client/login", {
    key,
    type: "3",
  });

  const code = typeof data.code === "number" ? data.code : Number(data.code ?? -1);
  const result: { code: number; cookie?: string; message?: string } = { code, message: data.message };

  if (code === 803) {
    try {
      const rustCookies = await invoke<string>("netease_get_cookies");
      if (rustCookies) {
        const jar: Record<string, string> = JSON.parse(rustCookies);
        setCookieJar(jar);
        result.cookie = jarToString(jar);
      }
    } catch (e) {
      console.log("[netease-api] loginQrCheck | get_cookies failed:", e);
    }
  }

  return result;
}

// ── Token refresh ─────────────────────────────────────────────────────

export async function loginRefresh(): Promise<boolean> {
  await neteaseRequest("api", "/api/login/token/refresh", {});
  try {
    const rustCookies = await invoke<string>("netease_get_cookies");
    if (rustCookies) {
      const jar: Record<string, string> = JSON.parse(rustCookies);
      if (Object.keys(jar).length > 0) {
        setCookieJar(jar);
        return true;
      }
    }
  } catch (e) {
    console.log("[netease-api] loginRefresh | get_cookies failed:", e);
  }
  return false;
}

// ── Auto-login (startup flow) ─────────────────────────────────────────

/**
 * Try to restore the previous login session on app startup.
 * Returns an Account if the stored cookie is still valid, otherwise null.
 */
export async function tryAutoLogin(): Promise<Account | null> {
  await initCookies();

  console.log("[netease-api] tryAutoLogin | has MUSIC_U:", isLoggedIn());
  if (!isLoggedIn()) return null;

  try {
    await loginRefresh();
  } catch (e) {
    console.log("[netease-api] tryAutoLogin | refresh failed:", e);
    setCookieJar({});
    return null;
  }

  // Lazy-import to avoid circular dependency
  const { getAccount } = await import("./account");
  try {
    const acc = await getAccount();
    console.log("[netease-api] tryAutoLogin | account:", acc.name, acc.id);
    return acc;
  } catch (e) {
    console.log("[netease-api] tryAutoLogin | getAccount failed:", e);
    return null;
  }
}
