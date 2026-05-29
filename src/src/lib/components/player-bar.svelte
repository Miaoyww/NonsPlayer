<script lang="ts">
  import { Play, Pause, SkipBack, SkipForward, Volume2, VolumeX, ListMusic, MicVocal, ChevronUp } from "@lucide/svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { onMount } from "svelte";
  import { fly, slide } from "svelte/transition";
  import LyricPlayer from "$lib/components/lyrics/LyricPlayer.svelte";
  import { parseLyric } from "$lib/components/lyrics/lyric-parser";
  import { getLyric } from "$lib/services/adapter-service";
  import type { LyricLine } from "$lib/types/lyric";

  let showLyrics = $state(false);
  let showVolumeSlider = $state(false);
  let lyricLines = $state<LyricLine[]>([]);
  let loadingLyric = $state(false);

  // Progress bar drag
  let progressBar = $state<HTMLDivElement>();
  let dragging = $state(false);
  let dragPercent = $state(0);

  const progress = $derived(
    playerService.duration > 0
      ? (dragging ? dragPercent : playerService.position / playerService.duration * 100)
      : 0
  );

  $effect(() => {
    if (lyricLines.length > 0) return;
    const song = playerService.currentSong;
    if (!song) return;
    loadLyricFor(song.adapterSlug, song.id);
  });

  async function loadLyricFor(adapterSlug: string, songId: string) {
    loadingLyric = true;
    try {
      const raw = await getLyric(adapterSlug, songId);
      if (raw) lyricLines = parseLyric(raw);
    } catch {
      lyricLines = [];
    } finally {
      loadingLyric = false;
    }
  }

  function formatTime(seconds: number): string {
    if (!isFinite(seconds) || seconds < 0) return "0:00";
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  function onProgressMouseDown(e: MouseEvent) {
    dragging = true;
    updateDrag(e);
  }

  function onProgressMouseMove(e: MouseEvent) {
    if (!dragging) return;
    updateDrag(e);
  }

  function onProgressMouseUp(e: MouseEvent) {
    if (!dragging) return;
    dragging = false;
    updateDrag(e);
    const seekTo = (dragPercent / 100) * playerService.duration;
    playerService.seek(seekTo);
  }

  function updateDrag(e: MouseEvent) {
    if (!progressBar) return;
    const rect = progressBar.getBoundingClientRect();
    const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width));
    dragPercent = (x / rect.width) * 100;
  }

  function handleLineClick(index: number, line: LyricLine) {
    playerService.seek(line.startTime / 1000);
  }

  function togglePlayMode() {
    const modes: Array<"sequential" | "shuffle" | "single_loop" | "list_loop"> = ["sequential", "shuffle", "single_loop", "list_loop"];
    const idx = modes.indexOf(playerService.playMode);
    playerService.setPlayMode(modes[(idx + 1) % modes.length]);
  }

  const playModeLabel = $derived(
    { sequential: "顺序", shuffle: "随机", single_loop: "单曲", list_loop: "列表" }[playerService.playMode] ?? "顺序"
  );

  const song = $derived(playerService.currentSong);
  const hasSong = $derived(song != null);
</script>

<svelte:window onmousemove={onProgressMouseMove} onmouseup={onProgressMouseUp} />

<!-- Lyrics panel overlay -->
{#if song && showLyrics}
  <div
    class="absolute left-1/2 -translate-x-1/2 bottom-20 z-50 w-full max-w-3xl h-96 rounded-t-xl bg-background/95 backdrop-blur border border-border/50 shadow-2xl overflow-hidden"
    transition:slide={{ axis: "y", duration: 300 }}
  >
    <div class="flex items-center justify-between px-4 py-2 border-b border-border/30">
      <span class="text-sm font-medium text-muted-foreground">歌词</span>
      <Button variant="ghost" size="icon" class="cursor-pointer" onclick={() => showLyrics = false}>
        <ChevronUp class="size-4" />
      </Button>
    </div>
    <div class="h-[calc(100%-2.5rem)]">
      {#if loadingLyric}
        <div class="flex items-center justify-center h-full text-sm text-muted-foreground">加载歌词中...</div>
      {:else if lyricLines.length > 0}
        <LyricPlayer
          lyricLines={lyricLines}
          currentTime={playerService.position * 1000}
          isPlaying={playerService.isPlaying}
          onLineClick={handleLineClick}
        />
      {:else}
        <div class="flex items-center justify-center h-full text-sm text-muted-foreground">暂无歌词</div>
      {/if}
    </div>
  </div>
{/if}

<!-- Player bar (floating card, fixed to viewport) -->
<div
  class="fixed bottom-3 left-3 right-3 h-20 rounded-xl border border-border/50 bg-background/95 backdrop-blur shadow-lg flex items-center gap-4 px-4 z-40"
  transition:fly={{ y: 20, duration: 250 }}
>
  <!-- Left: song info -->
  <div class="flex items-center gap-3 w-56 shrink-0">
    <div class="size-12 shrink-0 rounded-md bg-muted overflow-hidden">
      {#if song?.avatarUrl}
        <img src={song.avatarUrl} alt="" class="size-full object-cover" />
      {:else}
        <div class="size-full bg-linear-to-br from-primary/30 to-primary/10"></div>
      {/if}
    </div>
    <div class="min-w-0">
      {#if song}
        <p class="text-sm font-medium truncate">{song.name}</p>
        <p class="text-xs text-muted-foreground truncate">{song.artistsName}</p>
      {:else}
        <p class="text-sm text-muted-foreground">未在播放</p>
      {/if}
    </div>
  </div>

  <!-- Center: controls + progress -->
  <div class="flex-1 flex flex-col items-center gap-1 max-w-xl mx-auto">
    <!-- Buttons -->
    <div class="flex items-center gap-1">
      <Button variant="ghost" size="icon" class="cursor-pointer" disabled={!hasSong} onclick={togglePlayMode} title={playModeLabel}>
        <span class="text-[10px] font-bold text-muted-foreground">{playModeLabel}</span>
      </Button>
      <Button variant="ghost" size="icon" class="cursor-pointer" disabled={!hasSong} onclick={() => playerService.prev()}>
        <SkipBack class="size-5" />
      </Button>
      <Button
        variant="ghost" size="icon" class="cursor-pointer rounded-full bg-primary text-primary-foreground hover:bg-primary/90"
        disabled={!hasSong}
        onclick={() => playerService.togglePlayback()}
      >
        {#if playerService.isPlaying}
          <Pause class="size-5 fill-current" />
        {:else}
          <Play class="size-5 fill-current ml-0.5" />
        {/if}
      </Button>
      <Button variant="ghost" size="icon" class="cursor-pointer" disabled={!hasSong} onclick={() => playerService.next()}>
        <SkipForward class="size-5" />
      </Button>
      <Button variant="ghost" size="icon" class="cursor-pointer" disabled={!hasSong} onclick={() => showLyrics = !showLyrics}>
        <MicVocal class={`size-4 text-muted-foreground ${lyricLines.length === 0 ? 'opacity-50' : ''}`} />
      </Button>
    </div>

    <!-- Progress -->
    <div class="flex items-center gap-2 w-full text-xs text-muted-foreground">
      <span class="tabular-nums w-10 text-right">{formatTime(playerService.position)}</span>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        bind:this={progressBar}
        class="flex-1 h-1.5 bg-muted rounded-full cursor-pointer relative group"
        class:pointer-events-none={!hasSong}
        onmousedown={onProgressMouseDown}
        onkeydown={() => {}}
        role="slider"
        tabindex="0"
        aria-label="播放进度"
        aria-valuenow={progress}
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <div class="absolute inset-y-0 -top-1 left-0 right-0 group-hover:h-3 transition-all">
          <div
            class="h-full bg-primary rounded-full transition-[width] duration-75"
            style="width: {progress}%"
          ></div>
          <div
            class="absolute top-1/2 -translate-y-1/2 size-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity shadow"
            style="left: calc({progress}% - 6px)"
          ></div>
        </div>
      </div>
      <span class="tabular-nums w-10">{formatTime(playerService.duration)}</span>
    </div>
  </div>

  <!-- Right: volume + queue -->
  <div class="flex items-center gap-1 w-48 justify-end shrink-0">
    <div class="relative flex items-center" onmouseenter={() => showVolumeSlider = true} onmouseleave={() => showVolumeSlider = false}>
      <Button
        variant="ghost" size="icon" class="cursor-pointer"
        onclick={() => playerService.setVolume(playerService.volume > 0 ? 0 : 0.8)}
      >
        {#if playerService.volume === 0}
          <VolumeX class="size-4 text-muted-foreground" />
        {:else}
          <Volume2 class="size-4 text-muted-foreground" />
        {/if}
      </Button>
      {#if showVolumeSlider}
        <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 p-2 bg-background border border-border rounded-lg shadow-lg" transition:fly={{ y: 5, duration: 150 }}>
          <input
            type="range"
            min="0" max="100" value={playerService.volume * 100}
            oninput={(e) => playerService.setVolume(Number(e.currentTarget.value) / 100)}
            class="h-20 w-6 cursor-pointer appearance-none bg-muted rounded-full [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
            style="-webkit-appearance: slider-vertical; writing-mode: vertical-lr; direction: rtl;"
          />
        </div>
      {/if}
    </div>

    <Button variant="ghost" size="icon" class="cursor-pointer" href="/playqueue">
      <ListMusic class="size-4 text-muted-foreground" />
    </Button>
  </div>
</div>
