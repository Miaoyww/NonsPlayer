import type { LyricLine } from "$lib/types/lyric";

const ALIGN_TOLERANCE_MS = 300;

/**
 * Align translation or romanized lyrics to main lyric lines by time.
 * Dual-pointer O(N) algorithm. Returns a new array with sub text
 * written into `translatedLyric` or `romanLyric` on matching lines.
 *
 * @param mainLines  Parsed main lyric lines (from parseLrc / parseYrc)
 * @param subLines   Parsed translation/romaji lines (from parseLrc)
 * @param field      Which field to write: "translatedLyric" or "romanLyric"
 */
export function alignLyrics(
  mainLines: readonly LyricLine[],
  subLines: readonly LyricLine[],
  field: "translatedLyric" | "romanLyric",
): LyricLine[] {
  if (!mainLines.length || !subLines.length) {
    return mainLines.map((l) => ({ ...l, words: l.words.map((w) => ({ ...w })) }));
  }

  // Clone to avoid mutating the input
  const result: LyricLine[] = mainLines.map((l) => ({
    ...l,
    words: l.words.map((w) => ({ ...w })),
  }));

  let i = 0;
  let j = 0;

  while (i < result.length && j < subLines.length) {
    const diff = result[i].startTime - subLines[j].startTime;

    if (Math.abs(diff) <= ALIGN_TOLERANCE_MS) {
      // Time match → attach sub text to main line
      result[i][field] = subLines[j].words.map((w) => w.word).join("");
      i++;
      j++;
    } else if (diff < 0) {
      i++;
    } else {
      j++;
    }
  }

  return result;
}
