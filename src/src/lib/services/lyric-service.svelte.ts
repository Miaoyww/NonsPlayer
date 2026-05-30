import { parseLrc, parseYrc } from "@applemusic-like-lyrics/lyric";
import type { Song } from "$lib/types/song";
import type { LyricLine, LyricSource } from "$lib/types/lyric";
import { LyricSourceType } from "$lib/types/lyric";
import { getLyric } from "$lib/services/adapter-service";
import { fetchNeteaseLyric, searchNeteaseSong } from "$lib/services/netease-api-service";

// ── LyricService ────────────────────────────────────────────────────

class LyricService {
  // ── Reactive state (read by components) ──

  currentLyricLines = $state<LyricLine[]>([]);
  loadingLyric = $state(false);
  lyricSource = $state<LyricSource | null>(null);

  // ── Internal state ──

  private requestId = 0;

  // ── Main entry point ──

  async updateLyric(song: Song | null): Promise<void> {
    if (!song) {
      this.currentLyricLines = [];
      this.lyricSource = null;
      return;
    }

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

  // ── Core logic ──────────────────────────────────────────────────

  private async _getLyric(song: Song): Promise<LyricLine[]> {
    const isLocal = song.adapterSlug === "local";

    if (!isLocal) {
      // Online song: extract numeric ID → fetch from local Netease API
      const numericId = this._extractNumericId(song.id, song.adapterSlug);
      if (!numericId) return [];
      const lines = await this._fetchOnlineLyric(numericId, song.adapterSlug);
      if (lines) return lines;
      return [];
    }

    // Local song: try local LRC first
    const localLines = await this._fetchLocalLrc(song.id);
    if (localLines && localLines.length > 0) {
      this.lyricSource = { source: LyricSourceType.local };
      return localLines;
    }

    // No local LRC → search Netease by name + artist
    const onlineLines = await this._matchAndFetchOnline(song);
    if (onlineLines && onlineLines.length > 0) return onlineLines;

    return [];
  }

  // ── Fetchers ────────────────────────────────────────────────────

  /** Fetch lyrics from the local Netease API server. Prefers YRC (word-level). */
  private async _fetchOnlineLyric(
    numericId: string,
    adapterSlug: string,
  ): Promise<LyricLine[] | null> {
    const result = await fetchNeteaseLyric(numericId);
    if (!result) return null;

    // Prefer YRC (word-level) over LRC
    if (result.yrc) {
      const parsed = parseYrc(result.yrc);
      if (Array.isArray(parsed) && parsed.length > 0) {
        this.lyricSource = {
          source: LyricSourceType.platform,
          adapterSlug,
          adapterSongId: numericId,
        };
        return this._mapLines(parsed);
      }
    }

    if (result.lrc) {
      const parsed = parseLrc(result.lrc);
      if (Array.isArray(parsed) && parsed.length > 0) {
        this.lyricSource = {
          source: LyricSourceType.platform,
          adapterSlug,
          adapterSongId: numericId,
        };
        return this._mapLines(parsed);
      }
    }

    return null;
  }

  /** Read embedded or external .lrc from the local adapter. */
  private async _fetchLocalLrc(songId: string): Promise<LyricLine[] | null> {
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

  /** Search Netease for a match, then fetch its lyrics. */
  private async _matchAndFetchOnline(song: Song): Promise<LyricLine[] | null> {
    const query = song.artistsName ? `${song.name} ${song.artistsName}` : song.name;
    const results = await searchNeteaseSong(query, 5);
    if (results.length === 0) return null;

    const best = results[0];
    return this._fetchOnlineLyric(best.id, "netease");
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

  /** Map raw parsed lines to the app's LyricLine type. */
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
