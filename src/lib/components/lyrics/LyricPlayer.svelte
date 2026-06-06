<script lang="ts">
  import { LyricPlayer, type LyricLine, LyricLineMouseEvent } from "@applemusic-like-lyrics/core";
  import "@applemusic-like-lyrics/core/style.css";
  import { onMount, tick } from "svelte";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { getLyricLanguage, resolveLyricFont } from "$lib/utils/lyric-language";

  interface SpringParams {
    mass?: number;
    resistance?: number;
    stiffness?: number;
    softClamp?: boolean;
  }

  interface Props {
    lyricLines: LyricLine[];
    currentTime: number;
    isPlaying: boolean;
    isSeeking?: boolean;
    alignAnchor?: "top" | "bottom" | "center";
    alignPosition?: number;
    enableSpring?: boolean;
    enableBlur?: boolean;
    enableScale?: boolean;
    hidePassedLines?: boolean;
    wordFadeWidth?: number;
    showLyricTran?: boolean;
    showLyricRoma?: boolean;
    posYSpringParams?: SpringParams;
    scaleSpringParams?: SpringParams;
    onLineClick?: (lineIndex: number, line: LyricLine) => void;
  }

  let {
    lyricLines = [],
    currentTime = 0,
    isPlaying = false,
    isSeeking = false,
    alignAnchor = "center",
    alignPosition = 0.35,
    enableSpring = true,
    enableBlur = true,
    enableScale = true,
    hidePassedLines = false,
    wordFadeWidth = 0.5,
    showLyricTran = true,
    showLyricRoma = true,
    posYSpringParams = undefined,
    scaleSpringParams = undefined,
    onLineClick = undefined,
  }: Props = $props();

  let containerEl = $state<HTMLDivElement>();
  let player: LyricPlayer | null = $state(null);
  let animFrameId: number | null = null;
  let lastFrameTime = 0;
  let timeRef = $state(0);

  // ── Resolved font values ──────────────────────────────────────────
  let resolvedLyricFont = $derived(
    resolveLyricFont($globalSettings.lyricFontFamily, $globalSettings.fontFamily),
  );
  let resolvedEnFont = $derived(
    resolveLyricFont($globalSettings.englishLyricFont, resolvedLyricFont || $globalSettings.fontFamily),
  );
  let resolvedJaFont = $derived(
    resolveLyricFont($globalSettings.japaneseLyricFont, resolvedLyricFont || $globalSettings.fontFamily),
  );
  let resolvedKoFont = $derived(
    resolveLyricFont($globalSettings.koreanLyricFont, resolvedLyricFont || $globalSettings.fontFamily),
  );

  // ── Container style for CSS variables ─────────────────────────────
  let containerStyle = $derived({
    "--lyric-font-family": resolvedLyricFont || "",
    "--en-font-family": resolvedEnFont || "",
    "--ja-font-family": resolvedJaFont || "",
    "--ko-font-family": resolvedKoFont || "",
  });

  // ── Process lyric language & set lang attributes ──────────────────
  function processLyricLanguage() {
    if (!player) return;
    const lyricGroups = (player as any).currentLyricGroups;
    if (!Array.isArray(lyricGroups) || lyricGroups.length === 0) return;

    for (const group of lyricGroups) {
      const lyricLine = group.mainLine?.getLine?.();
      const lyricLineElement = group.mainLine?.getElement?.();
      if (!lyricLine || !lyricLineElement) continue;

      // Build content from word-level lyrics
      const content = lyricLine.words.map((w: { word: string }) => w.word).join("");
      if (!content) continue;

      const lang = getLyricLanguage(content);
      const mainLineEl = lyricLineElement.firstChild as HTMLElement | null;
      if (mainLineEl instanceof HTMLElement) {
        mainLineEl.setAttribute("lang", lang);
      }
    }
  }

  onMount(() => {
    player = new LyricPlayer();

    const el = player.getElement();
    el.style.width = "100%";
    el.style.height = "100%";
    containerEl!.appendChild(el);

    // Click handler — AMLL v0.5 dispatches "line-click" events
    player.addEventListener("line-click", (evt: Event) => {
      if (evt instanceof LyricLineMouseEvent) {
        onLineClick?.(evt.lineIndex, evt.line.getLine());
      }
    });

    return () => {
      stopLoop();
      player?.dispose();
    };
  });

  // Keep timeRef in sync with currentTime prop (so animation loop reads latest)
  $effect(() => { timeRef = currentTime; });

  // Sync lyric data (deep-spread to strip Svelte 5 proxies)
  // Respect showLyricTran/showLyricRoma toggles
  $effect(() => {
    if (player && lyricLines.length > 0) {
      const plain = lyricLines.map((l) => ({
        words: l.words.map((w) => ({ ...w })),
        translatedLyric: showLyricTran ? l.translatedLyric : "",
        romanLyric: showLyricRoma ? l.romanLyric : "",
        startTime: l.startTime,
        endTime: l.endTime,
        isBG: l.isBG,
        isDuet: l.isDuet,
      }));
      player.setLyricLines(plain);

      // Process language after lyrics are rendered
      tick().then(() => processLyricLanguage());
    }
  });

  // Sync config
  $effect(() => { player?.setAlignAnchor(alignAnchor); });
  $effect(() => { player?.setAlignPosition(alignPosition); });
  $effect(() => { player?.setEnableSpring(enableSpring); });
  $effect(() => { player?.setEnableBlur(enableBlur); });
  $effect(() => { player?.setEnableScale(enableScale); });
  $effect(() => { player?.setHidePassedLines(hidePassedLines); });
  $effect(() => { player?.setWordFadeWidth(wordFadeWidth); });
  $effect(() => {
    if (player && posYSpringParams) player.setLinePosYSpringParams(posYSpringParams);
  });
  $effect(() => {
    if (player && scaleSpringParams) player.setLineScaleSpringParams(scaleSpringParams);
  });

  // Play state -> animation loop
  $effect(() => {
    if (!player) return;
    if (isPlaying) {
      lastFrameTime = performance.now();
      startLoop();
    } else {
      stopLoop();
      player.pause();
    }
  });

  // Seek
  $effect(() => {
    if (isSeeking && player) {
      player.setCurrentTime(timeRef, true);
    }
  });

  function startLoop() {
    const tick = (now: number) => {
      if (!player) return;
      const dt = now - lastFrameTime;
      lastFrameTime = now;
      player.update(dt);
      player.setCurrentTime(timeRef | 0);
      animFrameId = requestAnimationFrame(tick);
    };
    animFrameId = requestAnimationFrame(tick);
  }

  function stopLoop() {
    if (animFrameId !== null) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
  }
</script>

<div
  bind:this={containerEl}
  class="lyric-player-container w-full h-full"
  style="contain: paint layout; font-family: var(--lyric-font-family); {Object.entries(containerStyle).map(([k, v]) => v ? `${k}: ${v}` : '').filter(Boolean).join('; ')}"
></div>

<style>
  /* Remove mask-image on AMLL emphasized wrappers — the gradient mask
     clips glyph descenders (g, j, p, q, y). */
  :global(.lyric-player-container) :global([class*="emphasizeWrapper"]) {
    mask-image: none !important;
    -webkit-mask-image: none !important;
  }

  /* Per-language font overrides (mirrors SPlayer AMLyric.vue) */
  :global(.lyric-player-container) :global([lang="ja"]) {
    font-family: var(--ja-font-family);
  }
  :global(.lyric-player-container) :global([lang="en"]) {
    font-family: var(--en-font-family);
  }
  :global(.lyric-player-container) :global([lang="ko"]) {
    font-family: var(--ko-font-family);
  }
</style>
