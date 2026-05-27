import { parseTTML } from "@applemusic-like-lyrics/lyric";
import type { LyricLine } from "$lib/types/lyric";

/**
 * Parse raw lyric text (TTML or LRC) into AMLL-compatible LyricLine array.
 */
export function parseLyric(raw: string): LyricLine[] {
  if (!raw.trim()) return [];

  // Try TTML first (QRC / Apple-style lyrics)
  try {
    const result = parseTTML(raw);
    return result.lines.map((line) => ({
      words: line.words.map((w) => ({
        startTime: w.startTime,
        endTime: w.endTime,
        word: w.word,
        romanWord: (w as any).romanWord,
        obscene: (w as any).obscene ?? false,
      })),
      translatedLyric: (line as any).translatedLyric ?? "",
      romanLyric: (line as any).romanLyric ?? "",
      startTime: line.startTime,
      endTime: line.endTime,
      isBG: (line as any).isBG ?? false,
      isDuet: (line as any).isDuet ?? false,
    }));
  } catch {
    // Not valid TTML, try LRC
  }

  return parseLRC(raw);
}

/**
 * Parse LRC format into LyricLine array.
 */
function parseLRC(raw: string): LyricLine[] {
  const lines: LyricLine[] = [];
  const lineRe = /\[(\d{2,}):(\d{2})(?:[.:](\d{2,3}))?\](.*)/g;

  // Collect raw matches
  const matches: Array<{
    timeMs: number;
    text: string;
  }> = [];

  for (const line of raw.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // Reset lastIndex since we're reusing the regex
    lineRe.lastIndex = 0;

    let match: RegExpExecArray | null;
    while ((match = lineRe.exec(trimmed)) !== null) {
      const mins = parseInt(match[1], 10);
      const secs = parseInt(match[2], 10);
      let ms = parseInt(match[3] ?? "0", 10);
      // If 2 digits, treat as centiseconds
      if (match[3] && match[3].length === 2) ms *= 10;

      const timeMs = mins * 60000 + secs * 1000 + ms;
      const text = match[4]?.trim() ?? "";
      if (text) {
        matches.push({ timeMs, text });
      }
    }
  }

  // Sort by time
  matches.sort((a, b) => a.timeMs - b.timeMs);

  // Build LyricLine array
  for (let i = 0; i < matches.length; i++) {
    const { timeMs, text } = matches[i];
    const endTimeMs = i + 1 < matches.length ? matches[i + 1].timeMs : timeMs + 5000;

    lines.push({
      words: [
        {
          startTime: timeMs,
          endTime: endTimeMs,
          word: text,
        },
      ],
      translatedLyric: "",
      romanLyric: "",
      startTime: timeMs,
      endTime: endTimeMs,
      isBG: false,
      isDuet: false,
    });
  }

  return lines;
}
