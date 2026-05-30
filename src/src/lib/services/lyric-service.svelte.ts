import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { parseLrc } from "@applemusic-like-lyrics/lyric";
import type { Song } from "$lib/types/song";
import type { LyricLine, LyricMap, LyricSource } from "$lib/types/lyric";
import { LyricSourceType } from "$lib/types/lyric";
import { getLyric, search } from "$lib/services/adapter-service";
import { getTtml, parseTtmlLyrics } from "$lib/services/amll-db-service";
import { globalSettings } from "$lib/stores/global-settings-store";

const TAG = "[lyric]";

// ── Matching helpers (mirrors Coriander Player's music_matcher.dart) ─

interface MatchResult {
  adapterSlug: string;
  songId: string;        // full prefixed ID (e.g. "netease_song_1010728767")
  numericId: string;     // raw numeric ID for AMLL DB (e.g. "1010728767")
  score: number;
}

/**
 * Simple character-level matching score.
 * Mirrors `_computeScore` from Coriander Player's music_matcher.dart.
 */
function computeScore(audio: Song, title: string, artists: string, album: string): number {
  const maxScore = audio.name.length + audio.artistsName.length + (audio.albumName?.length ?? 0);
  if (maxScore === 0) return 0;

  let score = 0;
  const minTitleLen = Math.min(audio.name.length, title.length);
  for (let i = 0; i < minTitleLen; i++) {
    if (audio.name[i] === title[i]) score += 1;
  }
  const minArtistLen = Math.min(audio.artistsName.length, artists.length);
  for (let i = 0; i < minArtistLen; i++) {
    if (audio.artistsName[i] === artists[i]) score += 1;
  }
  const albumName = audio.albumName ?? "";
  const minAlbumLen = Math.min(albumName.length, album.length);
  for (let i = 0; i < minAlbumLen; i++) {
    if (albumName[i] === album[i]) score += 1;
  }
  return score / maxScore;
}

/**
 * Extract the raw numeric song ID from an adapter-prefixed ID.
 * NetEase IDs: "netease_song_1010728767" → "1010728767"
 * QQ IDs: "qqmusic_song_..." → "..."
 * Local file paths are NOT valid — returns null.
 */
function extractNumericId(songId: string, adapterSlug: string): string | null {
  if (adapterSlug === "local") return null;

  const prefix = `${adapterSlug}_song_`;
  if (songId.startsWith(prefix)) {
    const id = songId.slice(prefix.length);
    if (/^\d+$/.test(id)) return id;
  }
  if (/^\d+$/.test(songId)) return songId;
  return null;
}

const ARTIST_SEPARATORS = /[、,/;&，xX]+|\/+/;

/**
 * Search streaming adapters for a match to a local song.
 * Mirrors `uniSearch` + `getMostMatchedLyric` from Coriander Player.
 */
async function matchLocalSong(song: Song): Promise<MatchResult | null> {
  const primaryArtist = song.artistsName.split(ARTIST_SEPARATORS)[0]?.trim() ?? "";
  const query = primaryArtist ? `${song.name} ${primaryArtist}` : song.name;
  console.log(`${TAG} [match] searching: query="${query}"`);

  const adaptersToTry = ["netease"]; // future: "qqmusic"
  const allResults: MatchResult[] = [];

  for (const adapterSlug of adaptersToTry) {
    try {
      const result = await search(adapterSlug, query);
      console.log(`${TAG} [match] ${adapterSlug} returned ${result?.songs?.length ?? 0} songs`);
      if (!result?.songs?.length) continue;

      for (let i = 0; i < Math.min(result.songs.length, 5); i++) {
        const s = result.songs[i];
        if (!s || s.isEmpty) continue;

        const numericId = extractNumericId(s.id, adapterSlug);
        if (!numericId) {
          console.log(`${TAG} [match]   #${i + 1} "${s.name}" id="${s.id}" → no numeric ID, skip`);
          continue;
        }

        const score = computeScore(song, s.name, s.artistsName, s.albumName ?? "");
        console.log(`${TAG} [match]   #${i + 1} "${s.name}" - "${s.artistsName}" id=${numericId} score=${score.toFixed(4)}`);
        allResults.push({ adapterSlug, songId: s.id, numericId, score });
      }
    } catch (err) {
      console.warn(`${TAG} [match] search failed for ${adapterSlug}:`, err);
    }
  }

  allResults.sort((a, b) => b.score - a.score);

  if (allResults.length > 0) {
    const best = allResults[0];
    console.log(`${TAG} [match] WINNER: "${song.name}" → ${best.adapterSlug}/${best.numericId} (score=${best.score.toFixed(4)}, candidates=${allResults.length})`);
    return best;
  }

  console.log(`${TAG} [match] NO MATCH for "${song.name}"`);
  return null;
}

// ── LyricService ────────────────────────────────────────────────────

class LyricService {
  // ── Reactive state (read by components) ──

  currentLyricLines = $state<LyricLine[]>([]);
  loadingLyric = $state(false);
  lyricSource = $state<LyricSource | null>(null);

  // ── Internal state ──

  private lyricMap: LyricMap = {};
  private currentSong: Song | null = null;
  private requestId = 0;

  constructor() {
    this._loadLyricMap();
  }

  // ── Main entry point ──

  async updateLyric(song: Song | null): Promise<void> {
    if (!song) {
      console.log(`${TAG} updateLyric(null) — clearing lyrics`);
      this.currentLyricLines = [];
      this.lyricSource = null;
      this.currentSong = null;
      return;
    }

    console.log(
      `${TAG} updateLyric: name="${song.name}" artist="${song.artistsName}" adapter=${song.adapterSlug} id=${song.id}`
    );

    this.currentSong = song;
    this.requestId += 1;
    const reqId = this.requestId;
    this.loadingLyric = true;

    try {
      const lines = await this._getLyric(song);
      if (this.requestId !== reqId) {
        console.log(`${TAG} updateLyric: stale request #${reqId}, discarded (current=#${this.requestId})`);
        return;
      }
      console.log(`${TAG} updateLyric: got ${lines.length} lines`);
      this.currentLyricLines = lines;
    } catch (err) {
      if (this.requestId !== reqId) return;
      console.error(`${TAG} updateLyric: error:`, err);
      this.currentLyricLines = [];
    } finally {
      if (this.requestId === reqId) {
        this.loadingLyric = false;
      }
    }
  }

  // ── Priority logic ──────────────────────────────────────────────

  private async _getLyric(song: Song): Promise<LyricLine[]> {
    const mapKey = `${song.adapterSlug}:${song.id}`;

    // 1. Check persisted preference in lyric map
    const persisted = this.lyricMap[mapKey];
    if (persisted) {
      console.log(`${TAG} _getLyric: persisted source=${persisted.source} ttmlId=${persisted.ttmlId ?? "-"} adapterSongId=${persisted.adapterSongId ?? "-"}`);
      const lines = await this._fetchFromSource(persisted, song);
      if (lines && lines.length > 0) {
        console.log(`${TAG} _getLyric: persisted source returned ${lines.length} lines`);
        return lines;
      }
      console.log(`${TAG} _getLyric: persisted source failed, removing stale entry`);
      delete this.lyricMap[mapKey];
    }

    // 2. No persisted preference — use settings-based priority
    const settings = get(globalSettings);
    const localFirst = settings.localLyricFirst;
    const enableAmll = settings.enableAmllDb;
    const isLocalSong = song.adapterSlug === "local";

    console.log(
      `${TAG} _getLyric: isLocal=${isLocalSong} localFirst=${localFirst} enableAmll=${enableAmll}`
    );

    if (isLocalSong) {
      return await this._getLyricForLocal(song, localFirst, enableAmll);
    }

    // Online song: AMLL DB → adapter fallback
    console.log(`${TAG} _getLyric: online song, trying AMLL DB → adapter`);
    const onlineResult = await this._tryOnlineMatched(song, song.adapterSlug, song.id, enableAmll);
    if (onlineResult) {
      console.log(`${TAG} _getLyric: online result OK (${onlineResult.length} lines)`);
      return onlineResult;
    }

    console.log(`${TAG} _getLyric: no lyrics found for online song`);
    return [];
  }

  private async _getLyricForLocal(
    song: Song,
    localFirst: boolean,
    enableAmll: boolean,
  ): Promise<LyricLine[]> {
    console.log(`${TAG} _getLyricForLocal: localFirst=${localFirst}`);

    if (localFirst) {
      const localResult = await this._tryLocal(song);
      if (localResult) {
        console.log(`${TAG} _getLyricForLocal: local LRC found (${localResult.length} lines)`);
        return localResult;
      }
      console.log(`${TAG} _getLyricForLocal: no local LRC, trying online match`);
      const onlineResult = await this._tryOnlineForLocal(song, enableAmll);
      console.log(`${TAG} _getLyricForLocal: online result = ${onlineResult ? onlineResult.length + " lines" : "null"}`);
      return onlineResult ?? [];
    }

    // online-first for local
    console.log(`${TAG} _getLyricForLocal: online-first, trying match`);
    const onlineResult = await this._tryOnlineForLocal(song, enableAmll);
    if (onlineResult) {
      console.log(`${TAG} _getLyricForLocal: online match OK (${onlineResult.length} lines)`);
      return onlineResult;
    }
    console.log(`${TAG} _getLyricForLocal: online match failed, falling back to local LRC`);
    const localResult = await this._tryLocal(song);
    console.log(`${TAG} _getLyricForLocal: local fallback = ${localResult ? localResult.length + " lines" : "null"}`);
    return localResult ?? [];
  }

  // ── Source fetchers ─────────────────────────────────────────────

  private async _tryLocal(song: Song): Promise<LyricLine[] | null> {
    console.log(`${TAG} _tryLocal: adapter=${song.adapterSlug} id=${song.id}`);
    try {
      const raw = await getLyric(song.adapterSlug, song.id);
      if (!raw) {
        console.log(`${TAG} _tryLocal: getLyric returned empty`);
        return null;
      }
      console.log(`${TAG} _tryLocal: raw LRC length=${raw.length}`);

      const lines = parseLrc(raw);
      if (!Array.isArray(lines) || lines.length === 0) {
        console.log(`${TAG} _tryLocal: parseLrc returned ${Array.isArray(lines) ? "empty array" : typeof lines}`);
        return null;
      }

      const formatted = this._mapLrcLines(lines);
      if (formatted.length === 0) {
        console.log(`${TAG} _tryLocal: _mapLrcLines returned empty`);
        return null;
      }

      console.log(`${TAG} _tryLocal: SUCCESS, ${formatted.length} parsed lines`);
      const mapKey = `${song.adapterSlug}:${song.id}`;
      this.lyricSource = { source: LyricSourceType.local, adapterSlug: song.adapterSlug };
      await this._saveLyricSource(mapKey, this.lyricSource);
      return formatted;
    } catch (err) {
      console.warn(`${TAG} _tryLocal: exception:`, err);
      return null;
    }
  }

  private async _tryOnlineForLocal(
    song: Song,
    enableAmll: boolean,
  ): Promise<LyricLine[] | null> {
    console.log(`${TAG} _tryOnlineForLocal: matching local song to platform...`);
    const match = await matchLocalSong(song);
    if (!match) {
      console.log(`${TAG} _tryOnlineForLocal: no platform match found`);
      return null;
    }

    console.log(`${TAG} _tryOnlineForLocal: matched → adapter=${match.adapterSlug} numericId=${match.numericId}`);
    return await this._tryOnlineMatched(
      song,
      match.adapterSlug,
      match.songId,
      enableAmll,
      match.numericId,
    );
  }

  private async _tryOnlineMatched(
    originalSong: Song,
    adapterSlug: string,
    songId: string,
    enableAmll: boolean,
    numericIdOverride?: string,
  ): Promise<LyricLine[] | null> {
    const numericId = numericIdOverride ?? extractNumericId(songId, adapterSlug);
    const mapKey = `${originalSong.adapterSlug}:${originalSong.id}`;
    console.log(`${TAG} _tryOnlineMatched: adapter=${adapterSlug} songId=${songId} numericId=${numericId ?? "null"} enableAmll=${enableAmll}`);

    // 1. Try AMLL TTML DB first
    if (enableAmll && numericId) {
      console.log(`${TAG} _tryOnlineMatched: trying AMLL DB for id=${numericId}`);
      const ttmlResult = await this._fetchAmll(numericId);
      if (ttmlResult && ttmlResult.length > 0) {
        console.log(`${TAG} _tryOnlineMatched: AMLL DB SUCCESS (${ttmlResult.length} lines)`);
        this.lyricSource = {
          source: LyricSourceType.amll,
          ttmlId: numericId,
          adapterSlug,
          adapterSongId: songId,
        };
        await this._saveLyricSource(mapKey, this.lyricSource);
        return ttmlResult;
      }
      console.log(`${TAG} _tryOnlineMatched: AMLL DB miss or empty`);
    } else if (!numericId) {
      console.log(`${TAG} _tryOnlineMatched: no numeric ID, skipping AMLL DB`);
    } else {
      console.log(`${TAG} _tryOnlineMatched: AMLL DB disabled, skipping`);
    }

    // 2. Fall back to adapter platform lyric
    console.log(`${TAG} _tryOnlineMatched: falling back to adapter getLyric(${adapterSlug}, ${songId})`);
    try {
      const raw = await getLyric(adapterSlug, songId);
      if (!raw) {
        console.log(`${TAG} _tryOnlineMatched: adapter getLyric returned empty`);
        return null;
      }
      console.log(`${TAG} _tryOnlineMatched: adapter raw LRC length=${raw.length}`);

      const lines = parseLrc(raw);
      if (!Array.isArray(lines) || lines.length === 0) {
        console.log(`${TAG} _tryOnlineMatched: parseLrc returned ${Array.isArray(lines) ? "empty array" : typeof lines}`);
        return null;
      }

      const formatted = this._mapLrcLines(lines);
      if (formatted.length === 0) {
        console.log(`${TAG} _tryOnlineMatched: _mapLrcLines returned empty`);
        return null;
      }

      console.log(`${TAG} _tryOnlineMatched: adapter SUCCESS (${formatted.length} lines)`);
      this.lyricSource = {
        source: LyricSourceType.platform,
        adapterSlug,
        adapterSongId: songId,
      };
      await this._saveLyricSource(mapKey, this.lyricSource);
      return formatted;
    } catch (err) {
      console.warn(`${TAG} _tryOnlineMatched: adapter exception:`, err);
      return null;
    }
  }

  private async _fetchAmll(numericId: string): Promise<LyricLine[] | null> {
    console.log(`${TAG} _fetchAmll: id=${numericId}`);
    try {
      const ttml = await getTtml(numericId);
      if (!ttml) {
        console.log(`${TAG} _fetchAmll: getTtml returned null (cache miss + fetch failed or 404)`);
        return null;
      }
      console.log(`${TAG} _fetchAmll: got TTML, length=${ttml.length}`);

      const lines = parseTtmlLyrics(ttml);
      if (lines.length === 0) {
        console.log(`${TAG} _fetchAmll: parseTtmlLyrics returned empty`);
        return null;
      }

      console.log(`${TAG} _fetchAmll: SUCCESS, ${lines.length} parsed lines`);
      return lines;
    } catch (err) {
      console.warn(`${TAG} _fetchAmll: exception:`, err);
      return null;
    }
  }

  // ── Persisted source dispatch ──────────────────────────────────

  private async _fetchFromSource(source: LyricSource, song: Song): Promise<LyricLine[] | null> {
    console.log(`${TAG} _fetchFromSource: source=${source.source} ttmlId=${source.ttmlId ?? "-"} adapterSlug=${source.adapterSlug ?? "-"} adapterSongId=${source.adapterSongId ?? "-"}`);
    switch (source.source) {
      case LyricSourceType.local:
        return await this._tryLocal(song);
      case LyricSourceType.amll: {
        const id = source.ttmlId ?? extractNumericId(source.adapterSongId ?? song.id, source.adapterSlug ?? song.adapterSlug);
        if (!id) {
          console.log(`${TAG} _fetchFromSource: amll source has no numeric ID, skip`);
          return null;
        }
        return await this._fetchAmll(id);
      }
      case LyricSourceType.platform: {
        const adapterSlug = source.adapterSlug ?? song.adapterSlug;
        const songId = source.adapterSongId ?? song.id;
        console.log(`${TAG} _fetchFromSource: platform fallback getLyric(${adapterSlug}, ${songId})`);
        try {
          const raw = await getLyric(adapterSlug, songId);
          if (!raw) return null;
          const lines = parseLrc(raw);
          if (!Array.isArray(lines) || lines.length === 0) return null;
          return this._mapLrcLines(lines);
        } catch (err) {
          console.warn(`${TAG} _fetchFromSource: platform exception:`, err);
          return null;
        }
      }
      default:
        return null;
    }
  }

  // ── Helpers ─────────────────────────────────────────────────────

  private _mapLrcLines(lines: any[]): LyricLine[] {
    return lines.map((line: any) => ({
      words: (line.words ?? []).map((w: any) => ({
        startTime: w.startTime ?? 0,
        endTime: w.endTime ?? 0,
        word: w.word ?? "",
      })),
      translatedLyric: line.translatedLyric ?? "",
      romanLyric: line.romanLyric ?? "",
      startTime: line.startTime ?? 0,
      endTime: line.endTime ?? 0,
      isBG: line.isBG ?? false,
      isDuet: line.isDuet ?? false,
    }));
  }

  // ── Lyric map persistence ───────────────────────────────────────

  private async _loadLyricMap(): Promise<void> {
    try {
      const json: string = await invoke("get_lyric_map");
      if (json && json !== "{}") {
        this.lyricMap = JSON.parse(json);
        const count = Object.keys(this.lyricMap).length;
        console.log(`${TAG} _loadLyricMap: loaded ${count} entries`);
      } else {
        console.log(`${TAG} _loadLyricMap: empty or missing, starting fresh`);
      }
    } catch (err) {
      console.warn(`${TAG} _loadLyricMap: failed:`, err);
      this.lyricMap = {};
    }
  }

  private async _saveLyricSource(key: string, source: LyricSource): Promise<void> {
    this.lyricMap[key] = source;
    console.log(`${TAG} _saveLyricSource: key="${key}" source=${source.source} ttmlId=${source.ttmlId ?? "-"}`);
    try {
      await invoke("save_lyric_map", { json: JSON.stringify(this.lyricMap) });
    } catch (err) {
      console.warn(`${TAG} _saveLyricSource: failed:`, err);
    }
  }

  // ── Public helpers ──────────────────────────────────────────────

  async useLocalLyric(): Promise<void> {
    if (!this.currentSong) return;
    console.log(`${TAG} useLocalLyric: manual switch to local`);
    this.loadingLyric = true;
    try {
      const lines = await this._tryLocal(this.currentSong);
      if (lines && lines.length > 0) {
        this.currentLyricLines = lines;
      }
    } finally {
      this.loadingLyric = false;
    }
  }

  async useOnlineLyric(): Promise<void> {
    if (!this.currentSong) return;
    const enableAmll = get(globalSettings).enableAmllDb;
    const isLocalSong = this.currentSong.adapterSlug === "local";
    console.log(`${TAG} useOnlineLyric: manual switch to online (isLocal=${isLocalSong})`);
    this.loadingLyric = true;
    try {
      let lines: LyricLine[] | null = null;
      if (isLocalSong) {
        lines = await this._tryOnlineForLocal(this.currentSong, enableAmll);
      } else {
        lines = await this._tryOnlineMatched(
          this.currentSong,
          this.currentSong.adapterSlug,
          this.currentSong.id,
          enableAmll,
        );
      }
      if (lines && lines.length > 0) {
        this.currentLyricLines = lines;
      } else {
        console.log(`${TAG} useOnlineLyric: no online lyrics found`);
      }
    } finally {
      this.loadingLyric = false;
    }
  }

  async refresh(): Promise<void> {
    if (this.currentSong) {
      console.log(`${TAG} refresh: clearing persisted source and re-fetching`);
      const mapKey = `${this.currentSong.adapterSlug}:${this.currentSong.id}`;
      delete this.lyricMap[mapKey];
      await this.updateLyric(this.currentSong);
    }
  }
}

export const lyricService = new LyricService();
