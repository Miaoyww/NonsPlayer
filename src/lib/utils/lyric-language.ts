/**
 * Detect the primary language of a lyric line.
 * Mirrors SPlayer's `getLyricLanguage` in `utils/format.ts`.
 *
 * @returns "ja" | "ko" | "zh-CN" | "en"
 */
export function getLyricLanguage(lyric: string): "ja" | "ko" | "zh-CN" | "en" {
  if (!lyric || typeof lyric !== "string") return "en";
  // Japanese — hiragana / katakana
  if (/[぀-ゟ゠-ヿ]/.test(lyric)) return "ja";
  // Korean — Hangul syllables
  if (/[가-힯]/.test(lyric)) return "ko";
  // Simplified Chinese — CJK Unified Ideographs basic block
  if (/[一-鿿]/.test(lyric)) return "zh-CN";
  // Default to English
  return "en";
}

/**
 * Resolve a lyric font setting to a CSS font-family value.
 *
 * @param fontValue   The stored value (e.g. "follow" or a CSS font-family string)
 * @param parentValue The parent font to fall back to when "follow"
 * @returns CSS font-family string, or empty string
 */
export function resolveLyricFont(fontValue: string, parentValue: string): string {
  if (!fontValue || fontValue === "follow") {
    return parentValue === "follow" ? "" : parentValue;
  }
  return fontValue;
}
