import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { parseLrc } from "@applemusic-like-lyrics/lyric";
import type { Song } from "$lib/types/song";
import type { LyricLine, LyricMap, LyricSource } from "$lib/types/lyric";
import { LyricSourceType } from "$lib/types/lyric";
import { getLyric, matchSongAcrossAdapters, listAdapters } from "$lib/services/adapter-service";
import type { MatchResult } from "$lib/services/adapter-service";
import { getTtml, parseTtmlLyrics } from "$lib/services/amll-db-service";
import { globalSettings } from "$lib/stores/global-settings-store";

const TAG = "[lyric]";

// ── Adapter → AMLL DB tag mapping ────────────────────────────────────
// Lazily populated from adapter metadata on first use.
let _adapterAmllTags: Record<string, string> | null = null;

async function _loadAmllTags(): Promise<Record<string, string>> {
  if (_adapterAmllTags) return _adapterAmllTags;
  try {
    const adapters = await listAdapters();
    _adapterAmllTags = {};
    for (const a of adapters) {
      if (a.amllDbTag) _adapterAmllTags[a.slug] = a.amllDbTag;
    }
  } catch {
    _adapterAmllTags = {};
  }
  return _adapterAmllTags;
}

/** Map adapter slug → AMLL DB tag (e.g. "netease" → "ncm"). */
async function getAmllDbTag(adapterSlug: string): Promise<string> {
  const tags = await _loadAmllTags();
  return tags[adapterSlug] ?? "";
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
    const onlineAmllTag = await getAmllDbTag(song.adapterSlug);
    const onlineResult = await this._tryOnlineMatched(song, song.adapterSlug, song.id, enableAmll, undefined, onlineAmllTag);
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
    const mapKey = `${song.adapterSlug}:${song.id}`;

    if (localFirst) {
      // Run local LRC and cross-adapter match CONCURRENTLY.
      // Local LRC is near-instant; match takes ~500ms-2s (HTTP search).
      // We return local immediately if found, but also try AMLL DB
      // in the background so it's available for manual switch later.
      const [localResult, matchResults] = await Promise.all([
        this._tryLocal(song),
        matchSongAcrossAdapters(song.name, song.artistsName),
      ]);

      const bestMatch = matchResults.length > 0 ? matchResults[0] : null;
      if (bestMatch) {
        console.log(`${TAG} _getLyricForLocal: best match → ${bestMatch.adapterSlug}/${bestMatch.numericId} (score=${bestMatch.score.toFixed(4)}) amllDbTag=${bestMatch.amllDbTag}`);
      }
      const matchMeta = bestMatch
        ? { adapterSlug: bestMatch.adapterSlug, songId: bestMatch.songId, numericId: bestMatch.numericId, amllDbTag: bestMatch.amllDbTag }
        : null;

      // Try AMLL DB concurrently with the local result if both are available.
      // Don't block the return — fire background fetch and cache for later use.
      if (matchMeta && enableAmll) {
        console.log(`${TAG} _getLyricForLocal: background AMLL DB fetch for id=${matchMeta.numericId} tag=${matchMeta.amllDbTag}`);
        this._fetchAmll(matchMeta.numericId, matchMeta.amllDbTag).then((result) => {
          if (result && result.length > 0) {
            console.log(`${TAG} _getLyricForLocal: background AMLL DB SUCCESS (${result.length} lines), updating source`);
            this._saveLyricSource(mapKey, {
              source: LyricSourceType.amll,
              ttmlId: matchMeta.numericId,
              adapterSlug: matchMeta.adapterSlug,
              adapterSongId: matchMeta.songId,
              amllDbTag: matchMeta.amllDbTag,
            });
          }
        });
      }

      // Merge: save the match meta so future plays & manual switch can use it.
      if (matchMeta && localResult) {
        await this._saveLyricSource(mapKey, {
          source: LyricSourceType.local,
          ttmlId: matchMeta.numericId,
          adapterSlug: matchMeta.adapterSlug,
          adapterSongId: matchMeta.songId,
          amllDbTag: matchMeta.amllDbTag,
        });
        console.log(`${TAG} _getLyricForLocal: merged local+online (ttmlId=${matchMeta.numericId})`);
      }

      if (localResult) {
        console.log(`${TAG} _getLyricForLocal: local LRC found (${localResult.length} lines)`);
        return localResult;
      }

      // No local LRC — use the matched platform result
      if (matchMeta) {
        const onlineResult = await this._tryOnlineMatched(
          song, matchMeta.adapterSlug, matchMeta.songId, enableAmll, matchMeta.numericId, matchMeta.amllDbTag,
        );
        console.log(`${TAG} _getLyricForLocal: online result = ${onlineResult ? onlineResult.length + " lines" : "null"}`);
        return onlineResult ?? [];
      }

      console.log(`${TAG} _getLyricForLocal: no local LRC and no match`);
      return [];
    }

    // online-first for local
    console.log(`${TAG} _getLyricForLocal: online-first, matching across adapters`);
    const matchResults = await matchSongAcrossAdapters(song.name, song.artistsName);
    if (matchResults.length > 0) {
      const best = matchResults[0];
      console.log(`${TAG} _getLyricForLocal: best match → ${best.adapterSlug}/${best.numericId} (score=${best.score.toFixed(4)})`);
      const onlineResult = await this._tryOnlineMatched(
        song, best.adapterSlug, best.songId, enableAmll, best.numericId, best.amllDbTag,
      );
      if (onlineResult) {
        console.log(`${TAG} _getLyricForLocal: online result OK (${onlineResult.length} lines)`);
        return onlineResult;
      }
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
      return formatted;
    } catch (err) {
      console.warn(`${TAG} _tryLocal: exception:`, err);
      return null;
    }
  }

  private async _tryOnlineMatched(
    originalSong: Song,
    adapterSlug: string,
    songId: string,
    enableAmll: boolean,
    numericIdOverride?: string,
    amllDbTag?: string,
  ): Promise<LyricLine[] | null> {
    const numericId = numericIdOverride ?? this._extractNumericId(songId, adapterSlug);
    const mapKey = `${originalSong.adapterSlug}:${originalSong.id}`;
    console.log(`${TAG} _tryOnlineMatched: adapter=${adapterSlug} songId=${songId} numericId=${numericId ?? "null"} enableAmll=${enableAmll} amllDbTag=${amllDbTag ?? "-"}`);

    // 1. Try AMLL TTML DB first
    if (enableAmll && numericId) {
      console.log(`${TAG} _tryOnlineMatched: trying AMLL DB for id=${numericId} tag=${amllDbTag ?? "(none)"}`);
      const ttmlResult = await this._fetchAmll(numericId, amllDbTag ?? "");
      if (ttmlResult && ttmlResult.length > 0) {
        console.log(`${TAG} _tryOnlineMatched: AMLL DB SUCCESS (${ttmlResult.length} lines)`);
        this.lyricSource = {
          source: LyricSourceType.amll,
          ttmlId: numericId,
          adapterSlug,
          adapterSongId: songId,
          amllDbTag: amllDbTag ?? "",
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

  /** Resolve AMLL DB tag from a source, falling back to adapter metadata. */
  private async _resolveAmllDbTag(source: LyricSource): Promise<string> {
    if (source.amllDbTag) return source.amllDbTag;
    // Fallback for old persisted data: look up from adapter metadata
    if (source.adapterSlug) return await getAmllDbTag(source.adapterSlug);
    return "";
  }

  private async _fetchAmll(numericId: string, platform: string): Promise<LyricLine[] | null> {
    console.log(`${TAG} _fetchAmll: id=${numericId} platform="${platform}"`);
    if (!platform) {
      console.warn(`${TAG} _fetchAmll: empty platform tag, AMLL DB URL will be malformed`);
    }
    try {
      const ttml = await getTtml(numericId, platform);
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

  /**
   * Fire-and-forget background match: populates the lyric map with
   * platform IDs so future plays can try AMLL DB / adapter fallback.
   * Only called when a local entry has no match IDs yet.
   */
  private _backgroundMatch(song: Song): void {
    const mapKey = `${song.adapterSlug}:${song.id}`;
    console.log(`${TAG} _backgroundMatch: starting for "${song.name}"`);
    matchSongAcrossAdapters(song.name, song.artistsName)
      .then(async (results) => {
        if (results.length === 0) {
          console.log(`${TAG} _backgroundMatch: no match found for "${song.name}"`);
          return;
        }
        const best = results[0];
        console.log(`${TAG} _backgroundMatch: best → ${best.adapterSlug}/${best.numericId} (score=${best.score.toFixed(4)})`);
        await this._saveLyricSource(mapKey, {
          source: LyricSourceType.local,
          ttmlId: best.numericId,
          adapterSlug: best.adapterSlug,
          adapterSongId: best.songId,
          amllDbTag: best.amllDbTag,
        });
        console.log(`${TAG} _backgroundMatch: merged for "${song.name}"`);
      })
      .catch((err) => {
        console.warn(`${TAG} _backgroundMatch: error:`, err);
      });
  }

  // ── Persisted source dispatch ──────────────────────────────────

  private async _fetchFromSource(source: LyricSource, song: Song): Promise<LyricLine[] | null> {
    console.log(`${TAG} _fetchFromSource: source=${source.source} ttmlId=${source.ttmlId ?? "-"} adapterSlug=${source.adapterSlug ?? "-"} adapterSongId=${source.adapterSongId ?? "-"}`);
    switch (source.source) {
      case LyricSourceType.local: {
        const enableAmll = get(globalSettings).enableAmllDb;
        const mapKey = `${song.adapterSlug}:${song.id}`;
        const localResult = await this._tryLocal(song);
        if (localResult) {
          // If this entry has no match IDs yet, kick off a background
          // match so future plays can fall back to AMLL DB / adapter.
          if (!source.ttmlId && !source.adapterSongId) {
            this._backgroundMatch(song);
          }
          // If we have a ttmlId from a previous match, try AMLL DB
          // in the background so it's ready for manual switch.
          if (source.ttmlId && enableAmll) {
            const ttmlId = source.ttmlId;
            const resolvedTag = await this._resolveAmllDbTag(source);
            console.log(`${TAG} _fetchFromSource: background AMLL DB fetch for id=${ttmlId} tag=${resolvedTag || "(unknown)"}`);
            this._fetchAmll(ttmlId, resolvedTag).then((result) => {
              if (result && result.length > 0) {
                console.log(`${TAG} _fetchFromSource: background AMLL DB SUCCESS (${result.length} lines)`);
                this._saveLyricSource(mapKey, {
                  source: LyricSourceType.amll,
                  ttmlId,
                  adapterSlug: source.adapterSlug ?? "",
                  adapterSongId: source.adapterSongId ?? "",
                  amllDbTag: resolvedTag,
                });
              }
            });
          }
          return localResult;
        }

        // Local failed — try fallback via stored match IDs
        const numericId = source.ttmlId
          ?? this._extractNumericId(source.adapterSongId ?? "", source.adapterSlug ?? "");
        if (enableAmll && numericId) {
          const fallbackTag = await this._resolveAmllDbTag(source);
          console.log(`${TAG} _fetchFromSource: local failed, trying AMLL DB fallback id=${numericId} tag=${fallbackTag || "(unknown)"}`);
          const amllResult = await this._fetchAmll(numericId, fallbackTag);
          if (amllResult) return amllResult;
        }
        if (source.adapterSlug && source.adapterSongId) {
          console.log(`${TAG} _fetchFromSource: local failed, trying adapter fallback ${source.adapterSlug}/${source.adapterSongId}`);
          try {
            const raw = await getLyric(source.adapterSlug, source.adapterSongId);
            if (raw) {
              const lines = parseLrc(raw);
              if (Array.isArray(lines) && lines.length > 0) return this._mapLrcLines(lines);
            }
          } catch { /* fall through */ }
        }
        return null;
      }
      case LyricSourceType.amll: {
        const id = source.ttmlId ?? this._extractNumericId(source.adapterSongId ?? song.id, source.adapterSlug ?? song.adapterSlug);
        if (!id) {
          console.log(`${TAG} _fetchFromSource: amll source has no numeric ID, skip`);
          return null;
        }
        const resolvedTag = await this._resolveAmllDbTag(source);
        return await this._fetchAmll(id, resolvedTag);
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

  /** Extract numeric song ID from adapter-prefixed ID (e.g. "netease_song_1010728767" → "1010728767"). */
  private _extractNumericId(songId: string, adapterSlug: string): string | null {
    if (!songId || adapterSlug === "local") return null;
    const prefix = `${adapterSlug}_song_`;
    if (songId.startsWith(prefix)) {
      const id = songId.slice(prefix.length);
      if (/^\d+$/.test(id)) return id;
    }
    if (/^\d+$/.test(songId)) return songId;
    return null;
  }

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
    const mapKey = `${this.currentSong.adapterSlug}:${this.currentSong.id}`;
    console.log(`${TAG} useOnlineLyric: manual switch to online (isLocal=${isLocalSong})`);
    this.loadingLyric = true;
    try {
      let lines: LyricLine[] | null = null;
      if (isLocalSong) {
        // Check if we already have a stored match result from a previous
        // background match — avoids re-doing the HTTP search.
        const persisted = this.lyricMap[mapKey];
        if (persisted?.ttmlId && persisted?.adapterSlug && persisted?.adapterSongId) {
          console.log(`${TAG} useOnlineLyric: using stored match → ${persisted.adapterSlug}/${persisted.ttmlId}`);
          lines = await this._tryOnlineMatched(
            this.currentSong,
            persisted.adapterSlug,
            persisted.adapterSongId,
            enableAmll,
            persisted.ttmlId,
            persisted.amllDbTag,
          );
        } else {
          const results = await matchSongAcrossAdapters(this.currentSong.name, this.currentSong.artistsName);
          if (results.length > 0) {
            const best = results[0];
            lines = await this._tryOnlineMatched(
              this.currentSong, best.adapterSlug, best.songId, enableAmll, best.numericId, best.amllDbTag,
            );
          }
        }
      } else {
        const onlineAmllTag = await getAmllDbTag(this.currentSong.adapterSlug);
        lines = await this._tryOnlineMatched(
          this.currentSong,
          this.currentSong.adapterSlug,
          this.currentSong.id,
          enableAmll,
          undefined,
          onlineAmllTag,
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
