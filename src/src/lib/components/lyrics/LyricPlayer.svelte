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
    onLineClick = undefined,
  }: Props = $props();

  let containerEl = $state<HTMLDivElement>();
  let player: LyricPlayer | null = $state(null);
  let animFrameId: number | null = null;
  let lastFrameTime = 0;

  onMount(() => {
    player = new LyricPlayer();

    const el = player.getElement();
    el.style.width = "100%";
    el.style.height = "100%";
    containerEl!.appendChild(el);

    // Click handler
    player.addEventListener("click", (evt: Event) => {
      if (evt instanceof LyricLineMouseEvent) {
        onLineClick?.(evt.lineIndex, evt.line.getLine());
      }
    });

    return () => {
      stopLoop();
      player?.dispose();
    };
  });

  // -- Sync lyric data --
  $effect(() => {
    if (player && lyricLines.length > 0) {
      player.setLyricLines(lyricLines);
    }
  });

  // -- Sync config --
  $effect(() => { player?.setAlignAnchor(alignAnchor); });
  $effect(() => { player?.setAlignPosition(alignPosition); });
  $effect(() => { player?.setEnableSpring(enableSpring); });
  $effect(() => { player?.setEnableBlur(enableBlur); });
  $effect(() => { player?.setEnableScale(enableScale); });
  $effect(() => { player?.setHidePassedLines(hidePassedLines); });
  $effect(() => { player?.setWordFadeWidth(wordFadeWidth); });

  // -- Play state -> animation loop --
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

  // -- Seek --
  $effect(() => {
    if (isSeeking && player) {
      player.setCurrentTime(currentTime, true);
    }
  });

  function startLoop() {
    const tick = (now: number) => {
      if (!player) return;
      const dt = now - lastFrameTime;
      lastFrameTime = now;
      player.update(dt);
      player.setCurrentTime(currentTime, false);
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

<div bind:this={containerEl} class="lyric-container w-full h-full overflow-hidden"></div>
