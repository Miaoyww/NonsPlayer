import { parseYrc } from "@applemusic-like-lyrics/lyric";
import { get } from "svelte/store";
import type { Song } from "$lib/types/song";
import type { LyricLine, LyricSource } from "$lib/types/lyric";
import { LyricSourceType } from "$lib/types/lyric";
import { getLyric } from "$lib/services/adapter-service";
import { fetchNeteaseLyric, searchNeteaseSong } from "$lib/services/netease-api-service";
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

  // ── Core priority logic ──────────────────────────────────────────

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

  /** Online song: priority-based multi-source (SPlayer pattern). */
  private async _getLyricForOnline(song: Song): Promise<LyricLine[]> {
    const numericId = this._extractNumericId(song.id, song.adapterSlug);
    if (!numericId) return [];

    const settings = get(globalSettings);
    const amllTag = AMLL_DB_TAGS[song.adapterSlug] ?? "";

    const result: { lrcData: LyricLine[]; yrcData: LyricLine[] } = { lrcData: [], yrcData: [] };
    let ttmlAdopted = false;

    // ── adoptTTML (AMLL DB: word-level, always tried first) ─────────
    const adoptTTML = async () => {
      const ttmlLines = await this._tryAmll(numericId, amllTag, song.adapterSlug);
      if (!ttmlLines) return;

      result.yrcData = ttmlLines;
      ttmlAdopted = true;
    };

    // ── adoptLRC (Netease API: YRC + LRC with translations) ─────────
    const adoptLRC = async () => {
      const apiResult = await fetchNeteaseLyric(numericId);
      if (!apiResult) {
        console.log("[lyric] fetchNeteaseLyric returned null for:", numericId);
        return;
      }

      console.log("[lyric] API response keys:", {
        hasLrc: !!apiResult.lrc,
        lrcLen: apiResult.lrc?.length ?? 0,
        hasYrc: !!apiResult.yrc,
        yrcLen: apiResult.yrc?.length ?? 0,
        hasTlyric: !!apiResult.tlyric,
        tlyricLen: apiResult.tlyric?.length ?? 0,
        hasYtlrc: !!apiResult.ytlrc,
        ytlrcLen: apiResult.ytlrc?.length ?? 0,
        hasRomalrc: !!apiResult.romalrc,
        romalrcLen: apiResult.romalrc?.length ?? 0,
        hasYromalrc: !!apiResult.yromalrc,
        yromalrcLen: apiResult.yromalrc?.length ?? 0,
      });

      let lrcLines: LyricLine[] = [];
      let yrcLines: LyricLine[] = [];

      // ── Line-level (LRC) ──
      if (apiResult.lrc) {
        const { lines: parsed } = parseSmartLrc(apiResult.lrc);
        console.log("[lyric] LRC main parsed:", parsed.length, "lines");
        if (parsed.length > 0) {
          lrcLines = this._mapLines(parsed);
          // Align line-level translations
          if (apiResult.tlyric) {
            const { lines: transParsed } = parseSmartLrc(apiResult.tlyric);
            console.log("[lyric] LRC tlyric parsed:", transParsed.length, "lines, sample:", transParsed[0]?.words?.[0]?.word);
            if (transParsed.length > 0) {
              const beforeTrans = lrcLines.filter((l) => l.translatedLyric).length;
              lrcLines = alignLyrics(lrcLines, this._mapLines(transParsed), "translatedLyric");
              const afterTrans = lrcLines.filter((l) => l.translatedLyric).length;
              console.log("[lyric] LRC aligned trans:", beforeTrans, "→", afterTrans);
            }
          }
          if (apiResult.romalrc) {
            const { lines: romaParsed } = parseSmartLrc(apiResult.romalrc);
            console.log("[lyric] LRC romalrc parsed:", romaParsed.length, "lines");
            if (romaParsed.length > 0) {
              lrcLines = alignLyrics(lrcLines, this._mapLines(romaParsed), "romanLyric");
            }
          }
        }
      }

      // ── Word-level (YRC) ──
      if (apiResult.yrc) {
        const mainParsed = this._tryParseYrc(apiResult.yrc);
        console.log("[lyric] YRC main parsed:", mainParsed ? mainParsed.length : 0, "lines");
        if (mainParsed) {
          yrcLines = this._mapLines(mainParsed);
        }
      }

      // ── Align translations to YRC ──
      const transText = apiResult.ytlrc || apiResult.tlyric;
      const romaText = apiResult.yromalrc || apiResult.romalrc;

      console.log("[lyric] transText source:", apiResult.ytlrc ? "ytlrc" : apiResult.tlyric ? "tlyric" : "none",
        "length:", transText?.length ?? 0);

      if (yrcLines.length > 0) {
        if (transText) {
          const { lines: transParsed } = parseSmartLrc(transText);
          console.log("[lyric] YRC trans parsed:", transParsed.length, "lines, sample:", transParsed[0]?.words?.[0]?.word);
          if (transParsed.length > 0) {
            const beforeTrans = yrcLines.filter((l) => l.translatedLyric).length;
            yrcLines = alignLyrics(yrcLines, this._mapLines(transParsed), "translatedLyric");
            const afterTrans = yrcLines.filter((l) => l.translatedLyric).length;
            console.log("[lyric] YRC aligned trans:", beforeTrans, "→", afterTrans);
          }
        }
        if (romaText) {
          const { lines: romaParsed } = parseSmartLrc(romaText);
          console.log("[lyric] YRC roma parsed:", romaParsed.length, "lines");
          if (romaParsed.length > 0) {
            yrcLines = alignLyrics(yrcLines, this._mapLines(romaParsed), "romanLyric");
          }
        }
      }

      // If TTML already has yrcData, align external translations onto it
      if (result.yrcData.length > 0 && (transText || romaText)) {
        console.log("[lyric] aligning external trans onto existing TTML yrcData (", result.yrcData.length, "lines)");
        if (transText) {
          const { lines: transParsed } = parseSmartLrc(transText);
          console.log("[lyric] TTML+yrc trans parsed:", transParsed.length, "lines, sample:", transParsed[0]?.words?.[0]?.word);
          if (transParsed.length > 0) {
            const beforeTrans = result.yrcData.filter((l) => l.translatedLyric).length;
            result.yrcData = alignLyrics(result.yrcData, this._mapLines(transParsed), "translatedLyric");
            const afterTrans = result.yrcData.filter((l) => l.translatedLyric).length;
            console.log("[lyric] TTML+yrc aligned trans:", beforeTrans, "→", afterTrans);
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
      // Use Netease YRC only if no TTML word-level lyrics yet
      if (!result.yrcData.length && yrcLines.length) {
        console.log("[lyric] using Netease YRC for yrcData:", yrcLines.length, "lines");
        result.yrcData = yrcLines;
      }
    };

    // ── Strategy: AMLL TTML first → Netease API fallback ───────────
    await adoptTTML();
    await adoptLRC();

    // ── Dedupe fake translations ──
    dedupeTranslations(result.yrcData);
    dedupeTranslations(result.lrcData);

    // ── Log final state ──
    const yrcWithTrans = result.yrcData.filter((l) => l.translatedLyric).length;
    const lrcWithTrans = result.lrcData.filter((l) => l.translatedLyric).length;
    console.log("[lyric] final — yrcData:", result.yrcData.length, "lines (", yrcWithTrans, "with trans ), lrcData:",
      result.lrcData.length, "lines (", lrcWithTrans, "with trans ), ttmlAdopted:", ttmlAdopted,
      "showWordLyrics:", settings.showWordLyrics);

    // ── Set source metadata ──
    if (ttmlAdopted || result.yrcData.length > 0) {
      this.lyricSource = { source: LyricSourceType.platform, adapterSlug: song.adapterSlug, adapterSongId: numericId };
    }

    // ── Return preferred data (word-level first if enabled) ──
    if (settings.showWordLyrics && result.yrcData.length > 0) {
      console.log("[lyric] returning yrcData");
      return result.yrcData;
    }
    if (result.lrcData.length > 0) {
      console.log("[lyric] returning lrcData");
      return result.lrcData;
    }
    if (result.yrcData.length > 0) {
      console.log("[lyric] returning yrcData (fallback)");
      return result.yrcData;
    }

    console.log("[lyric] no lyrics found");
    return [];
  }

  /** Local song: AMLL TTML → Netease match → local LRC */
  private async _getLyricForLocal(song: Song): Promise<LyricLine[]> {
    const settings = get(globalSettings);
    const localLines = await this._tryLocalLrc(song.id);
    console.log("[lyric:local] local LRC parsed:", localLines?.length ?? 0, "lines",
      "with trans:", localLines?.filter((l) => l.translatedLyric).length ?? 0);

    // 非本地优先
    if (!settings.localLyricFirst) {
      const query = song.artistsName ? `${song.name} ${song.artistsName}` : song.name;
      const results = await searchNeteaseSong(query, 5);
      console.log("[lyric:local] Netease search for:", query, "→", results.length, "results");
      if (results.length > 0) {
        const best = results[0];
        console.log("[lyric:local] best match:", best.id, best.name, best.artists);
        const amllTag = AMLL_DB_TAGS["netease"] ?? "ncm";
        const ttmlLines = await this._tryAmll(best.id, amllTag, "netease");
        if (ttmlLines) {
          console.log("[lyric:local] TTML hit, returning", ttmlLines.length, "lines");
          return ttmlLines;
        }
        const neteaseLines = await this._tryNeteaseApi(best.id, "netease");
        if (neteaseLines) {
          const withTrans = neteaseLines.filter((l) => l.translatedLyric).length;
          console.log("[lyric:local] Netease API hit, returning", neteaseLines.length, "lines,", withTrans, "with trans");
          return neteaseLines;
        }
        console.log("[lyric:local] Netease API returned null");
      }
    }

    // 本地优先
    if (localLines && localLines.length > 0) {
      this.lyricSource = { source: LyricSourceType.local };
      console.log("[lyric:local] falling back to local LRC:", localLines.length, "lines");
      return localLines;
    }

    console.log("[lyric:local] no lyrics found");
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

    // Prefer YRC (word-level) over LRC
    if (result.yrc) {
      const mainParsed = this._tryParseYrc(result.yrc);
      console.log("[lyric:netease] YRC parsed:", mainParsed ? mainParsed.length : 0, "lines, tlyric:", !!result.tlyric, "ytlrc:", !!result.ytlrc);
      if (mainParsed) {
        let lines = this._mapLines(mainParsed);
        const beforeTrans = lines.filter((l) => l.translatedLyric).length;
        lines = this._alignTrans(lines, result.ytlrc || result.tlyric, result.yromalrc || result.romalrc);
        const afterTrans = lines.filter((l) => l.translatedLyric).length;
        console.log("[lyric:netease] YRC trans aligned:", beforeTrans, "→", afterTrans);
        this.lyricSource = { source: LyricSourceType.platform, adapterSlug, adapterSongId: numericId };
        return lines;
      }
    }

    if (result.lrc) {
      // Use smart parser for main lyrics
      const { lines: mainParsed } = parseSmartLrc(result.lrc);
      console.log("[lyric:netease] LRC parsed:", mainParsed.length, "lines, tlyric:", !!result.tlyric);
      if (mainParsed.length > 0) {
        let lines = this._mapLines(mainParsed);
        const beforeTrans = lines.filter((l) => l.translatedLyric).length;
        lines = this._alignTrans(lines, result.tlyric, result.romalrc);
        const afterTrans = lines.filter((l) => l.translatedLyric).length;
        console.log("[lyric:netease] LRC trans aligned:", beforeTrans, "→", afterTrans);
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
      console.log("[lyric:align] transText length:", transText.length, "parsed:", transParsed.length, "lines, sample:", transParsed[0]?.words?.[0]?.word?.slice(0, 40));
      if (transParsed.length > 0) {
        lines = alignLyrics(lines, this._mapLines(transParsed), "translatedLyric");
      }
    } else {
      console.log("[lyric:align] no transText provided");
    }
    if (romaText) {
      const { lines: romaParsed } = parseSmartLrc(romaText);
      console.log("[lyric:align] romaText length:", romaText.length, "parsed:", romaParsed.length, "lines");
      if (romaParsed.length > 0) {
        lines = alignLyrics(lines, this._mapLines(romaParsed), "romanLyric");
      }
    }
    // Remove fake translations (same text as main)
    dedupeTranslations(lines);
    return lines;
  }

  private async _tryLocalLrc(songId: string): Promise<LyricLine[] | null> {
    try {
      const raw = await getLyric("local", songId);
      if (!raw) return null;

      // Parse with smart parser, then merge same-timestamp lines into trans/roma
      const { lines: parsed } = parseSmartLrc(raw);
      if (parsed.length === 0) return null;

      console.log("[lyric:localLrc] raw parsed:", parsed.length, "lines, sample startTime:", parsed[0]?.startTime);
      const merged = alignLyricLines(parsed);
      const withTrans = merged.filter((l) => l.translatedLyric).length;
      console.log("[lyric:localLrc] after alignLyricLines:", merged.length, "lines,", withTrans, "with trans");
      return this._mapLines(merged);
    } catch {
      return null;
    }
  }

  // ── Format-robust parsers ─────────────────────────────────────────

  /** Parse YRC text. Falls back to smart LRC parser if YRC format fails. */
  private _tryParseYrc(text: string): any[] | null {
    if (!text || text.trim().length === 0) return null;

    try {
      const parsed = parseYrc(text);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    } catch { /* fall through */ }

    // Fall back to smart LRC parser
    try {
      const { lines } = parseSmartLrc(text);
      if (lines.length > 0) return lines;
    } catch { /* fall through */ }

    // Last resort: JSON format
    return this._tryParseJson(text);
  }

  /** Try extracting lyrics from JSON format. */
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
