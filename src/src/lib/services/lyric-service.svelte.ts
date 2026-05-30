import { parseLrc, parseYrc } from "@applemusic-like-lyrics/lyric";
import { get } from "svelte/store";
import type { Song } from "$lib/types/song";
import type { LyricLine, LyricSource } from "$lib/types/lyric";
import { LyricSourceType } from "$lib/types/lyric";
import { getLyric } from "$lib/services/adapter-service";
import { fetchNeteaseLyric, searchNeteaseSong } from "$lib/services/netease-api-service";
import { getTtml, parseTtmlLyrics } from "$lib/services/amll-db-service";
import { globalSettings } from "$lib/stores/global-settings-store";
import { alignLyrics } from "$lib/utils/lyric-align";

// ── Platform → AMLL DB tag mapping ──────────────────────────────────

const AMLL_DB_TAGS: Record<string, string> = {
  netease: "ncm",
};

// ── LyricService ────────────────────────────────────────────────────

class LyricService {
  // ── Reactive state (read by components) ──

  currentLyricLines = $state<LyricLine[]>([]);
  loadingLyric = $state(false);
  lyricSource = $state<LyricSource | null>(null);

  // ── Internal state ──

  private requestId = 0;
  private lyricCache = new Map<string, LyricLine[]>();
  private prefetchedLyric: { key: string; lines: LyricLine[] } | null = null;
  private static MAX_CACHE_SIZE = 50;

  // ── Main entry point ──

  async updateLyric(song: Song | null): Promise<void> {
    if (!song) {
      this.currentLyricLines = [];
      this.lyricSource = null;
      return;
    }

    const cacheKey = this._cacheKey(song);

    // Check prefetch cache first
    if (this.prefetchedLyric?.key === cacheKey) {
      this.currentLyricLines = this.prefetchedLyric.lines;
      this.prefetchedLyric = null;
      return;
    }
    this.prefetchedLyric = null;

    this.requestId += 1;
    const reqId = this.requestId;
    this.loadingLyric = true;

    try {
      const lines = await this._getLyric(song);
      if (this.requestId !== reqId) return;
      this.currentLyricLines = lines;
    } catch (err) {
      if (this.requestId !== reqId) return;
      console.error("[lyric] error:", err);
      this.currentLyricLines = [];
    } finally {
      if (this.requestId === reqId) {
        this.loadingLyric = false;
      }
    }
  }

  /** Pre-fetch lyrics for the next song. Fire-and-forget. */
  async prefetch(song: Song | null): Promise<void> {
    if (!song) {
      this.prefetchedLyric = null;
      return;
    }
    const cacheKey = this._cacheKey(song);
    if (this.lyricCache.has(cacheKey)) return;

    try {
      const lines = await this._getLyric(song);
      this.prefetchedLyric = { key: cacheKey, lines };
    } catch {
      this.prefetchedLyric = null;
    }
  }

  // ── Core priority logic (参照 SPlayer fetchOnlineLyric) ─────────

  private async _getLyric(song: Song): Promise<LyricLine[]> {
    const cacheKey = this._cacheKey(song);

    // Check in-memory cache
    const cached = this.lyricCache.get(cacheKey);
    if (cached) return cached;

    const isLocal = song.adapterSlug === "local";

    let lines: LyricLine[];
    if (isLocal) {
      lines = await this._getLyricForLocal(song);
    } else {
      lines = await this._getLyricForOnline(song);
    }

    this._addToCache(cacheKey, lines);
    return lines;
  }

  /** Online song: AMLL TTML → Netease YRC → Netease LRC */
  private async _getLyricForOnline(song: Song): Promise<LyricLine[]> {
    const numericId = this._extractNumericId(song.id, song.adapterSlug);
    if (!numericId) return [];

    const settings = get(globalSettings);
    const amllTag = AMLL_DB_TAGS[song.adapterSlug] ?? "";

    // 1. Try AMLL TTML DB first (word-level synced lyrics)
    if (settings.enableAmllDb && amllTag) {
      const ttmlLines = await this._tryAmll(numericId, amllTag, song.adapterSlug);
      if (ttmlLines) return ttmlLines;
    }

    // 2. Try Netease API (YRC word-level → LRC fallback)
    const neteaseLines = await this._tryNeteaseApi(numericId, song.adapterSlug);
    if (neteaseLines) return neteaseLines;

    return [];
  }

  /** Local song: AMLL TTML (if matched) → local LRC → search Netease match → online flow */
  private async _getLyricForLocal(song: Song): Promise<LyricLine[]> {
    const settings = get(globalSettings);
    const localLines = await this._tryLocalLrc(song.id);

    // If AMLL DB is enabled, search for a Netease match so we can try TTML
    if (settings.enableAmllDb) {
      const query = song.artistsName ? `${song.name} ${song.artistsName}` : song.name;
      const results = await searchNeteaseSong(query, 5);
      if (results.length > 0) {
        const best = results[0];
        const amllTag = AMLL_DB_TAGS["netease"] ?? "ncm";
        // Try AMLL TTML — word-level lyrics take priority over local LRC
        const ttmlLines = await this._tryAmll(best.id, amllTag, "netease");
        if (ttmlLines) return ttmlLines;
        // TTML miss — fall back to Netease API for YRC/LRC
        const neteaseLines = await this._tryNeteaseApi(best.id, "netease");
        if (neteaseLines) return neteaseLines;
      }
    }

    // No online match or AMLL disabled — use local LRC
    if (localLines && localLines.length > 0) {
      this.lyricSource = { source: LyricSourceType.local };
      return localLines;
    }

    return [];
  }

  // ── Individual source fetchers ───────────────────────────────────

  private async _tryAmll(
    numericId: string,
    amllTag: string,
    adapterSlug: string,
  ): Promise<LyricLine[] | null> {
    try {
      const ttml = await getTtml(numericId, amllTag);
      if (!ttml) return null;

      const lines = parseTtmlLyrics(ttml);
      if (lines.length === 0) return null;

      this.lyricSource = {
        source: LyricSourceType.platform,
        adapterSlug,
        adapterSongId: numericId,
      };
      return lines;
    } catch {
      return null;
    }
  }

  private async _tryNeteaseApi(
    numericId: string,
    adapterSlug: string,
  ): Promise<LyricLine[] | null> {
    const result = await fetchNeteaseLyric(numericId);
    if (!result) return null;

    const hasTrans = !!(result.tlyric || result.ytlrc);
    const hasRoma = !!(result.romalrc || result.yromalrc);
    if (hasTrans || hasRoma) {
      console.log(`[lyric] translations found for ${numericId}: trans=${hasTrans} roma=${hasRoma}`);
    }

    // Prefer YRC (word-level) over LRC
    if (result.yrc) {
      const mainParsed = this._tryParse(result.yrc, true);
      if (mainParsed) {
        let lines = this._mapLines(mainParsed);
        // Align translations: prefer word-level (ytlrc), fall back to line-level (tlyric)
        const transText = result.ytlrc || result.tlyric;
        if (transText) {
          const transParsed = this._tryParse(transText, false);
          if (transParsed) {
            lines = alignLyrics(lines, this._mapLines(transParsed), "translatedLyric");
          }
        }
        const romaText = result.yromalrc || result.romalrc;
        if (romaText) {
          const romaParsed = this._tryParse(romaText, false);
          if (romaParsed) {
            lines = alignLyrics(lines, this._mapLines(romaParsed), "romanLyric");
          }
        }
        this.lyricSource = { source: LyricSourceType.platform, adapterSlug, adapterSongId: numericId };
        return lines;
      }
    }

    if (result.lrc) {
      const mainParsed = this._tryParse(result.lrc, false);
      if (mainParsed) {
        let lines = this._mapLines(mainParsed);
        // Align line-level translations
        if (result.tlyric) {
          const transParsed = this._tryParse(result.tlyric, false);
          if (transParsed) {
            lines = alignLyrics(lines, this._mapLines(transParsed), "translatedLyric");
          }
        }
        if (result.romalrc) {
          const romaParsed = this._tryParse(result.romalrc, false);
          if (romaParsed) {
            lines = alignLyrics(lines, this._mapLines(romaParsed), "romanLyric");
          }
        }
        this.lyricSource = { source: LyricSourceType.platform, adapterSlug, adapterSongId: numericId };
        return lines;
      }
    }

    return null;
  }

  private async _tryLocalLrc(songId: string): Promise<LyricLine[] | null> {
    try {
      const raw = await getLyric("local", songId);
      if (!raw) return null;
      const parsed = parseLrc(raw);
      if (!Array.isArray(parsed) || parsed.length === 0) return null;
      return this._mapLines(parsed);
    } catch {
      return null;
    }
  }

  // ── Format-robust parser ─────────────────────────────────────────

  /** Try multiple parsing strategies. Returns raw parsed lines or null. */
  private _tryParse(text: string, preferWordLevel: boolean): any[] | null {
    if (!text || text.trim().length === 0) return null;

    // Strategy 1: YRC parser (word-level timestamps)
    if (preferWordLevel) {
      try {
        const parsed = parseYrc(text);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      } catch { /* fall through */ }
    }

    // Strategy 2: Standard LRC parser
    try {
      const parsed = parseLrc(text);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    } catch { /* fall through */ }

    // Strategy 3: JSON format extraction (legacy JSON-format lyrics)
    try {
      if (text.startsWith("{") && text.includes('"c"')) {
        const obj = JSON.parse(text);
        if (Array.isArray(obj.c)) {
          const items = obj.c.filter((item: any) => typeof item.tx === "string");
          if (items.length > 0) {
            return items.map((item: any) => ({
              words: [{ startTime: 0, endTime: 0, word: item.tx }],
              startTime: 0,
              endTime: 0,
            }));
          }
        }
      }
    } catch { /* last resort failed */ }

    return null;
  }

  // ── Helpers ─────────────────────────────────────────────────────

  private _cacheKey(song: Song): string {
    return `${song.adapterSlug}:${song.id}`;
  }

  private _addToCache(key: string, lines: LyricLine[]): void {
    if (this.lyricCache.size >= LyricService.MAX_CACHE_SIZE) {
      const firstKey = this.lyricCache.keys().next().value;
      if (firstKey !== undefined) this.lyricCache.delete(firstKey);
    }
    this.lyricCache.set(key, lines);
  }

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

  private _mapLines(lines: any[]): LyricLine[] {
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
}

export const lyricService = new LyricService();
