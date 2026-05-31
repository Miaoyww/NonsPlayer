import { parseYrc } from "@applemusic-like-lyrics/lyric";
import { get } from "svelte/store";
import type { Song } from "$lib/types/song";
import type { LyricLine, LyricSource } from "$lib/types/lyric";
import { LyricSourceType } from "$lib/types/lyric";
import { getLyric } from "$lib/services/adapter-service";
import { fetchNeteaseLyric, searchNeteaseSong } from "$lib/services/netease-api";
import { getTtml, parseTtmlLyrics } from "$lib/services/amll-db-service";
import { globalSettings } from "$lib/stores/global-settings-store";
import {
  alignLyrics,
  alignLyricLines,
  parseSmartLrc,
  cleanTTMLTranslations,
} from "$lib/utils/lyric-parser";

// ── Platform → AMLL DB tag mapping ──────────────────────────────────

const AMLL_DB_TAGS: Record<string, string> = {
  netease: "ncm",
};

// ── Helpers ─────────────────────────────────────────────────────────

/** Strip translations that are identical to the main lyric text (not real translations). */
function dedupeTranslations(lines: LyricLine[]): void {
  for (const line of lines) {
    const mainText = line.words.map((w) => w.word).join("").trim();
    if (line.translatedLyric && line.translatedLyric.trim() === mainText) {
      line.translatedLyric = "";
    }
    if (line.romanLyric && line.romanLyric.trim() === mainText) {
      line.romanLyric = "";
    }
  }
}

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

  // ── Core logic ────────────────────────────────────────────────────

  private async _getLyric(song: Song): Promise<LyricLine[]> {
    const cacheKey = this._cacheKey(song);
    const cached = this.lyricCache.get(cacheKey);
    if (cached) return cached;

    const isLocal = song.adapterSlug === "local";
    const lines = isLocal
      ? await this._getLyricForLocal(song)
      : await this._getLyricForOnline(song);

    this._addToCache(cacheKey, lines);
    return lines;
  }

  /** Online song: AMLL TTML → Netease API */
  private async _getLyricForOnline(song: Song): Promise<LyricLine[]> {
    const numericId = this._extractNumericId(song.id, song.adapterSlug);
    if (!numericId) return [];

    const settings = get(globalSettings);
    const amllTag = AMLL_DB_TAGS[song.adapterSlug] ?? "";

    const result: { lrcData: LyricLine[]; yrcData: LyricLine[] } = { lrcData: [], yrcData: [] };
    let ttmlAdopted = false;

    // ── adoptTTML (AMLL DB) ──
    const adoptTTML = async () => {
      const ttmlLines = await this._tryAmll(numericId, amllTag, song.adapterSlug);
      if (!ttmlLines) return;
      result.yrcData = ttmlLines;
      ttmlAdopted = true;
    };

    // ── adoptLRC (Netease API: YRC + LRC with translations) ──
    const adoptLRC = async () => {
      const apiResult = await fetchNeteaseLyric(numericId);
      if (!apiResult) return;

      let lrcLines: LyricLine[] = [];
      let yrcLines: LyricLine[] = [];

      // Line-level (LRC)
      if (apiResult.lrc) {
        const { lines: parsed } = parseSmartLrc(apiResult.lrc);
        if (parsed.length > 0) {
          lrcLines = this._mapLines(parsed);
          if (apiResult.tlyric) {
            const { lines: transParsed } = parseSmartLrc(apiResult.tlyric);
            if (transParsed.length > 0) {
              lrcLines = alignLyrics(lrcLines, this._mapLines(transParsed), "translatedLyric");
            }
          }
          if (apiResult.romalrc) {
            const { lines: romaParsed } = parseSmartLrc(apiResult.romalrc);
            if (romaParsed.length > 0) {
              lrcLines = alignLyrics(lrcLines, this._mapLines(romaParsed), "romanLyric");
            }
          }
        }
      }

      // Word-level (YRC)
      if (apiResult.yrc) {
        const mainParsed = this._tryParseYrc(apiResult.yrc);
        if (mainParsed) {
          yrcLines = this._mapLines(mainParsed);
        }
      }

      // Align translations
      const transText = apiResult.ytlrc || apiResult.tlyric;
      const romaText = apiResult.yromalrc || apiResult.romalrc;

      if (yrcLines.length > 0) {
        if (transText) {
          const { lines: transParsed } = parseSmartLrc(transText);
          if (transParsed.length > 0) {
            yrcLines = alignLyrics(yrcLines, this._mapLines(transParsed), "translatedLyric");
          }
        }
        if (romaText) {
          const { lines: romaParsed } = parseSmartLrc(romaText);
          if (romaParsed.length > 0) {
            yrcLines = alignLyrics(yrcLines, this._mapLines(romaParsed), "romanLyric");
          }
        }
      }

      // If TTML already has yrcData, align external translations onto it
      if (result.yrcData.length > 0 && (transText || romaText)) {
        if (transText) {
          const { lines: transParsed } = parseSmartLrc(transText);
          if (transParsed.length > 0) {
            result.yrcData = alignLyrics(result.yrcData, this._mapLines(transParsed), "translatedLyric");
          }
        }
        if (romaText) {
          const { lines: romaParsed } = parseSmartLrc(romaText);
          if (romaParsed.length > 0) {
            result.yrcData = alignLyrics(result.yrcData, this._mapLines(romaParsed), "romanLyric");
          }
        }
      }

      if (lrcLines.length) result.lrcData = lrcLines;
      if (!result.yrcData.length && yrcLines.length) {
        result.yrcData = yrcLines;
      }
    };

    await adoptTTML();
    await adoptLRC();

    dedupeTranslations(result.yrcData);
    dedupeTranslations(result.lrcData);

    if (ttmlAdopted || result.yrcData.length > 0) {
      this.lyricSource = { source: LyricSourceType.platform, adapterSlug: song.adapterSlug, adapterSongId: numericId };
    }

    if (settings.showWordLyrics && result.yrcData.length > 0) {
      return result.yrcData;
    }
    if (result.lrcData.length > 0) return result.lrcData;
    if (result.yrcData.length > 0) return result.yrcData;
    return [];
  }

  /** Local song: online match if !localLyricFirst → local LRC */
  private async _getLyricForLocal(song: Song): Promise<LyricLine[]> {
    const settings = get(globalSettings);
    const localLines = await this._tryLocalLrc(song.id);

    if (!settings.localLyricFirst) {
      const query = song.artistsName ? `${song.name} ${song.artistsName}` : song.name;
      const results = await searchNeteaseSong(query, 5);
      if (results.length > 0) {
        const best = results[0];
        const amllTag = AMLL_DB_TAGS["netease"] ?? "ncm";
        const ttmlLines = await this._tryAmll(best.id, amllTag, "netease");
        if (ttmlLines) return ttmlLines;
        const neteaseLines = await this._tryNeteaseApi(best.id, "netease");
        if (neteaseLines) return neteaseLines;
      }
    }

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

      const cleaned = cleanTTMLTranslations(ttml);
      const lines = parseTtmlLyrics(cleaned);
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

    if (result.yrc) {
      const mainParsed = this._tryParseYrc(result.yrc);
      if (mainParsed) {
        let lines = this._mapLines(mainParsed);
        lines = this._alignTrans(lines, result.ytlrc || result.tlyric, result.yromalrc || result.romalrc);
        this.lyricSource = { source: LyricSourceType.platform, adapterSlug, adapterSongId: numericId };
        return lines;
      }
    }

    if (result.lrc) {
      const { lines: mainParsed } = parseSmartLrc(result.lrc);
      if (mainParsed.length > 0) {
        let lines = this._mapLines(mainParsed);
        lines = this._alignTrans(lines, result.tlyric, result.romalrc);
        this.lyricSource = { source: LyricSourceType.platform, adapterSlug, adapterSongId: numericId };
        return lines;
      }
    }

    return null;
  }

  /** Parse and align translation / romaji into main lines. */
  private _alignTrans(
    lines: LyricLine[],
    transText: string | undefined,
    romaText: string | undefined,
  ): LyricLine[] {
    if (transText) {
      const { lines: transParsed } = parseSmartLrc(transText);
      if (transParsed.length > 0) {
        lines = alignLyrics(lines, this._mapLines(transParsed), "translatedLyric");
      }
    }
    if (romaText) {
      const { lines: romaParsed } = parseSmartLrc(romaText);
      if (romaParsed.length > 0) {
        lines = alignLyrics(lines, this._mapLines(romaParsed), "romanLyric");
      }
    }
    dedupeTranslations(lines);
    return lines;
  }

  private async _tryLocalLrc(songId: string): Promise<LyricLine[] | null> {
    try {
      const raw = await getLyric("local", songId);
      if (!raw) return null;

      const { lines: parsed } = parseSmartLrc(raw);
      if (parsed.length === 0) return null;

      const merged = alignLyricLines(parsed);
      return this._mapLines(merged);
    } catch {
      return null;
    }
  }

  // ── Format-robust parsers ─────────────────────────────────────────

  private _tryParseYrc(text: string): any[] | null {
    if (!text || text.trim().length === 0) return null;

    try {
      const parsed = parseYrc(text);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    } catch { /* fall through */ }

    try {
      const { lines } = parseSmartLrc(text);
      if (lines.length > 0) return lines;
    } catch { /* fall through */ }

    return this._tryParseJson(text);
  }

  private _tryParseJson(text: string): any[] | null {
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
    } catch { /* not JSON */ }
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
