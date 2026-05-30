<script lang="ts">
  import { LyricPlayer, type LyricLine, LyricLineMouseEvent } from "@applemusic-like-lyrics/core";
  import "@applemusic-like-lyrics/core/style.css";
  import { onMount } from "svelte";

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
    onLineClick = undefined,
  }: Props = $props();

  let containerEl = $state<HTMLDivElement>();
  let player: LyricPlayer | null = $state(null);
  let animFrameId: number | null = null;
  let lastFrameTime = 0;
  let timeRef = $state(0);

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

<div bind:this={containerEl} class="w-full h-full" style="contain: paint layout;"></div>
