/**
 * Netease API service — encryption &amp; orchestration in TypeScript,
 * raw HTTP via Rust `netease_tunnel` command.
 *
 * This folder mirrors the structure of api-enhanced: each module corresponds
 * to one or more Netease API endpoints.
 *
 *     netease-api/
 *       index.ts          — barrel re-exports (this file)
 *       types.ts          — shared type definitions
 *       crypto.ts         — weapi / eapi encryption &amp; decryption
 *       request.ts        — request orchestrator (URL, headers, encrypt, decrypt)
 *       cookies.ts        — cookie jar management + persistence
 *       login.ts          — QR login flow + token refresh
 *       account.ts        — user profile, playlists, favorites
 *       song.ts           — song detail, URL, lyric, like
 *       search.ts         — cloudsearch
 *       album.ts          — album detail
 *       artist.ts         — artist detail + hot songs
 *       playlist.ts       — playlist detail
 *       recommend.ts      — personalized playlists + daily songs
 *       discover.ts       — toplist, categories, playlist square
 *       lyric.ts          — full lyric fetch + song search
 */

// Foundation
export type { QrKeyResult, NeteaseLyricResult } from "./types";
export { neteaseAuth, isLoggedIn, getCookie, setCookieJar, jarToString, initCookies } from "./cookies";

// Login
export { loginQrKey, loginQrCreate, loginQrCheck, loginRefresh, tryAutoLogin } from "./login";

// Account
export { getAccount, getUserPlaylists, getFavoritePlaylist } from "./account";

// Song
export { getSong, getSongs, getSongUrl, getLyric, toggleLike } from "./song";

// Search
export { search } from "./search";

// Album / Artist / Playlist
export { getAlbum } from "./album";
export { getArtist } from "./artist";
export { getPlaylist } from "./playlist";

// Recommend
export { getRecommendedPlaylists, getDailyRecommended } from "./recommend";

// Discover
export { getTopPlaylists, getPlaylistCats, getPlaylistSquare } from "./discover";

// Lyric
export { fetchNeteaseLyric, searchNeteaseSong } from "./lyric";
