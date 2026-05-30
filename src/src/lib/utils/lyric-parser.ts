// 来自SPlayer的歌词解释器 https://github.com/imsyy/SPlayer

import { parseLrc } from "@applemusic-like-lyrics/lyric";
import type { LyricLine, LyricWord } from "$lib/types/lyric";

// ── Types ───────────────────────────────────────────────────────────

export enum LrcFormat {
  /** 普通逐行 LRC */
  Line = "line",
  /** 逐字 LRC：[00:28.850]曲[00:32.455]：[00:36.060]钱 */
  WordByWord = "word-by-word",
  /** 增强型 LRC (ESLyric)：[01:37.305]<01:37.624>怕<01:37.943>你 */
  Enhanced = "enhanced",
}

// ── Compiled regex ──────────────────────────────────────────────────

const META_TAG_REGEX = /^\[[a-z]+:/i;
const ENHANCED_TIME_TAG_REGEX = /<(\d{2}):(\d{2})\.(\d{1,})>/;
const LINE_TIME_REGEX = /^\[(\d{2}):(\d{2})\.(\d{1,})\]/;

const DEFAULT_WORD_DURATION = 1000;
const ALIGN_TOLERANCE_MS = 300;

// ── Helpers ─────────────────────────────────────────────────────────

function parseTimeToMs(min: string, sec: string, ms: string): number {
  const minutes = parseInt(min, 10);
  const seconds = parseInt(sec, 10);
  const msNormalized = ms.padEnd(3, "0").slice(0, 3);
  return minutes * 60 * 1000 + seconds * 1000 + milliseconds(msNormalized);
}

function milliseconds(s: string): number {
  return parseInt(s, 10);
}

function createWord(word: string, startTime: number, endTime: number = startTime): LyricWord {
  return { word, startTime, endTime };
}

function createLine(words: LyricWord[], startTime: number, endTime = 0): LyricLine {
  return { words, startTime, endTime, translatedLyric: "", romanLyric: "", isBG: false, isDuet: false };
}

function toText(line: LyricLine): string {
  return (line.words ?? []).map((w) => w.word).join("").trim();
}

function cloneLines(lines: readonly LyricLine[]): LyricLine[] {
  return structuredClone(lines) as LyricLine[];
}

// ── Format detection ────────────────────────────────────────────────

export function detectLrcFormat(content: string): LrcFormat {
  const rawLines = content.split(/\r?\n/);
  for (const rawLine of rawLines) {
    const line = rawLine.trim();
    if (!line || META_TAG_REGEX.test(line)) continue;
    if (ENHANCED_TIME_TAG_REGEX.test(line)) return LrcFormat.Enhanced;

    const timeTagCount = line.match(/\[(\d{2}):(\d{2})\.(\d{1,})\]/g);
    if (timeTagCount && timeTagCount.length > 1) return LrcFormat.WordByWord;
  }
  return LrcFormat.Line;
}

// ── Word-by-word LRC parser ─────────────────────────────────────────

export function parseWordByWordLrc(content: string): LyricLine[] {
  const result: LyricLine[] = [];
  let prevLine: LyricLine | null = null;
  const WORD_BY_WORD_PATTERN = /\[(\d{2}):(\d{2})\.(\d{1,})\]([^[\]]*)/g;

  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || META_TAG_REGEX.test(line)) continue;

    const words: LyricWord[] = [];
    let lineStartTime = Infinity;
    let prevWord: LyricWord | null = null;

    const matches = line.matchAll(WORD_BY_WORD_PATTERN);
    for (const match of matches) {
      const startTime = parseTimeToMs(match[1], match[2], match[3]);
      const wordText = match[4];
      if (!wordText && words.length === 0) continue;

      lineStartTime = Math.min(lineStartTime, startTime);
      if (prevWord) prevWord.endTime = startTime;

      if (wordText) {
        const newWord = createWord(wordText, startTime);
        words.push(newWord);
        prevWord = newWord;
      }
    }

    if (prevWord) prevWord.endTime = prevWord.startTime + DEFAULT_WORD_DURATION;

    if (words.length > 0) {
      const lineObj = createLine(words, lineStartTime === Infinity ? 0 : lineStartTime);
      lineObj.endTime = words[words.length - 1].endTime;

      if (prevLine) {
        const prevLastWord = prevLine.words[prevLine.words.length - 1];
        if (lineObj.startTime > prevLastWord.startTime) {
          prevLastWord.endTime = Math.min(prevLastWord.endTime, lineObj.startTime);
          prevLine.endTime = prevLastWord.endTime;
        }
      }
      result.push(lineObj);
      prevLine = lineObj;
    }
  }
  return result;
}

// ── Enhanced LRC (ESLyric) parser ───────────────────────────────────

export function parseEnhancedLrc(content: string): LyricLine[] {
  const result: LyricLine[] = [];
  let prevLine: LyricLine | null = null;
  const ENHANCED_WORD_PATTERN = /<(\d{2}):(\d{2})\.(\d{1,})>([^<]*)/g;

  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || META_TAG_REGEX.test(line)) continue;

    const lineTimeMatch = LINE_TIME_REGEX.exec(line);
    if (!lineTimeMatch) continue;

    const lineStartTime = parseTimeToMs(lineTimeMatch[1], lineTimeMatch[2], lineTimeMatch[3]);
    const contentAfterTime = line.slice(lineTimeMatch[0].length);
    const words: LyricWord[] = [];

    if (ENHANCED_TIME_TAG_REGEX.test(contentAfterTime)) {
      let prevWord: LyricWord | null = null;
      const matches = contentAfterTime.matchAll(ENHANCED_WORD_PATTERN);
      for (const match of matches) {
        const startTime = parseTimeToMs(match[1], match[2], match[3]);
        const wordText = match[4];
        if (prevWord) prevWord.endTime = startTime;
        if (wordText) {
          const newWord = createWord(wordText, startTime);
          words.push(newWord);
          prevWord = newWord;
        }
      }
      if (prevWord) prevWord.endTime = prevWord.startTime + DEFAULT_WORD_DURATION;
    } else {
      const text = contentAfterTime.trim();
      if (text) words.push(createWord(text, lineStartTime, lineStartTime + DEFAULT_WORD_DURATION));
    }

    if (words.length > 0) {
      const lineObj = createLine(words, lineStartTime);
      lineObj.endTime = words[words.length - 1].endTime;

      if (prevLine) {
        const prevLastWord = prevLine.words[prevLine.words.length - 1];
        if (lineObj.startTime > prevLastWord.startTime) {
          prevLastWord.endTime = Math.min(prevLastWord.endTime, lineObj.startTime);
          prevLine.endTime = prevLastWord.endTime;
        }
      }
      result.push(lineObj);
      prevLine = lineObj;
    }
  }
  return result;
}

// ── Smart parser (auto-detect) ──────────────────────────────────────

export function parseSmartLrc(content: string): { format: LrcFormat; lines: LyricLine[] } {
  const format = detectLrcFormat(content);
  let lines: LyricLine[];

  switch (format) {
    case LrcFormat.WordByWord:
      lines = parseWordByWordLrc(content);
      break;
    case LrcFormat.Enhanced:
      lines = parseEnhancedLrc(content);
      break;
    default:
      lines = (parseLrc(content) as LyricLine[]) || [];
  }

  console.log(`[LyricParser] format: ${format}, ${lines.length} lines`);
  return { format, lines };
}

// ── Time alignment (dual-pointer O(N), 300ms tolerance) ─────────────

export function alignLyrics(
  lyrics: readonly LyricLine[],
  otherLyrics: readonly LyricLine[],
  key: "translatedLyric" | "romanLyric",
): LyricLine[] {
  if (!lyrics.length || !otherLyrics.length) {
    console.log("[alignLyrics] early return — lyrics:", lyrics.length, "otherLyrics:", otherLyrics.length);
    return cloneLines(lyrics);
  }

  const result = cloneLines(lyrics);
  let i = 0;
  let j = 0;
  let matched = 0;

  while (i < result.length && j < otherLyrics.length) {
    const diff = result[i].startTime - otherLyrics[j].startTime;

    if (Math.abs(diff) <= ALIGN_TOLERANCE_MS) {
      result[i][key] = otherLyrics[j].words.map((w) => w.word).join("");
      matched++;
      i++;
      j++;
    } else if (diff < 0) {
      i++;
    } else {
      j++;
    }
  }

  console.log("[alignLyrics]", key, "—", lyrics.length, "main,", otherLyrics.length, "sub →", matched, "matched, tolerance:", ALIGN_TOLERANCE_MS, "ms");
  if (matched === 0 && lyrics.length > 0 && otherLyrics.length > 0) {
    console.log("[alignLyrics] WARNING: zero matches! first main startTime:", lyrics[0].startTime,
      "first sub startTime:", otherLyrics[0].startTime,
      "diff:", lyrics[0].startTime - otherLyrics[0].startTime);
  }

  return result;
}

// ── Same-timestamp grouping (for local LRC with trans/roma on adjacent lines) ─

export function alignLyricLines(
  lyrics: readonly LyricLine[],
  opts: Partial<{
    endTime: "ignore" | "match" | "set";
    maxTimeDiff: number;
    skipSort: boolean;
  }> = {},
): LyricLine[] {
  const { endTime = "set", maxTimeDiff = 0, skipSort = false } = opts;
  if (!lyrics.length) return [];

  const toStartTime = (l: LyricLine) => l.startTime ?? l.words?.[0]?.startTime ?? 0;
  const toEndTime = (l: LyricLine) => l.endTime ?? l.words?.[l.words.length - 1]?.endTime ?? 0;

  const isTimeMatch = (a: LyricLine | undefined, b: LyricLine | undefined): boolean => {
    if (!a || !b) return false;
    const diff = Math.abs(toStartTime(a) - toStartTime(b));
    if (!Number.isFinite(diff) || diff > maxTimeDiff) return false;
    if (endTime === "match") {
      const eDiff = Math.abs(toEndTime(a) - toEndTime(b));
      if (!Number.isFinite(eDiff) || eDiff > maxTimeDiff) return false;
    }
    return true;
  };

  const sorted = skipSort
    ? lyrics
    : [...lyrics].sort((a, b) => toStartTime(a) - toStartTime(b));

  const groups: LyricLine[][] = [];
  for (const line of sorted) {
    const last = groups[groups.length - 1]?.[0];
    if (isTimeMatch(last, line)) {
      groups[groups.length - 1].push(line);
    } else {
      groups.push([line]);
    }
  }

  return groups.map((group) => {
    const base = structuredClone(group[0]) as LyricLine;
    const merge = (add: LyricLine | undefined, key: "translatedLyric" | "romanLyric") => {
      if (base[key] || !add) return;
      const text = toText(add);
      if (!text) return;
      base[key] = text;
      if (endTime !== "set") return;
      const addEnd = toEndTime(add);
      const baseEnd = toEndTime(base);
      if (!Number.isFinite(addEnd) || addEnd <= baseEnd) return;
      base.endTime = addEnd;
      const lastWord = base.words?.[base.words.length - 1];
      if (lastWord && lastWord.endTime === baseEnd) lastWord.endTime = addEnd;
    };
    merge(group[1], "translatedLyric");
    merge(group[2], "romanLyric");
    return base;
  });
}

// ── TTML translation cleaner ─────────────────────────────────────────

export function cleanTTMLTranslations(ttml: string): string {
  if (!ttml || ttml.trim().length === 0) return ttml;

  const lineRegex = /<p\b[^>]*\bbegin="([^"]*)"[^>]*\bend="([^"]*)"[^>]*>/gi;
  const lines: { match: string; begin: number; end: number; isTrans: boolean }[] = [];
  let m: RegExpExecArray | null;

  while ((m = lineRegex.exec(ttml)) !== null) {
    const begin = parseFloat(m[1]);
    const end = parseFloat(m[2]);
    const isTrans = /xml:lang\s*=\s*"(?!zh\b)[^"]*"/i.test(m[0]);
    lines.push({ match: m[0], begin, end, isTrans });
  }

  if (lines.length <= 1) return ttml;

  const sorted = [...lines].sort((a, b) => {
    if (a.isTrans !== b.isTrans) return a.isTrans ? 1 : -1;
    return a.begin - b.begin;
  });

  let result = ttml;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].match !== sorted[i].match) {
      result = result.replace(lines[i].match, sorted[i].match);
    }
  }

  return result;
}
