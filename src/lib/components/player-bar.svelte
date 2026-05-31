<script lang="ts">
  import { Play, Pause, SkipBack, SkipForward, Volume2, VolumeX, ListMusic, Shuffle, Repeat, Repeat1 } from "@lucide/svelte";
import { playerUI } from "$lib/stores/player-ui-store.svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { fly } from "svelte/transition";
  import { lyricService } from "$lib/services/lyric-service.svelte";
  import { coverSrc } from "$lib/utils";

  let showVolumeSlider = $state(false);
  let volumeTimer: ReturnType<typeof setTimeout> | null = null;

  function scheduleHideVolume() {
    volumeTimer = setTimeout(() => { showVolumeSlider = false; }, 200);
  }
  function cancelHideVolume() {
    if (volumeTimer) { clearTimeout(volumeTimer); volumeTimer = null; }
  }
  function showVolume() {
    cancelHideVolume();
    showVolumeSlider = true;
  }

  // Progress bar drag
  let progressBar = $state<HTMLDivElement>();
  let dragging = $state(false);
  let dragPercent = $state(0);

  const progress = $derived(
    playerService.duration > 0
      ? (dragging ? dragPercent : playerService.position / playerService.duration * 100)
      : 0
  );

  let lastSongId = $state("");

  $effect(() => {
    const song = playerService.currentSong;
    if (!song) {
      lyricService.updateLyric(null);
      return;
    }
    // Only update when song actually changes
    if (song.id !== lastSongId) {
      lastSongId = song.id;
      lyricService.updateLyric(song);
      // Pre-fetch next song's lyrics
      const nextSong = playerService.getNextSong();
      if (nextSong) {
        lyricService.prefetch(nextSong);
      }
    }
  });

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

  function togglePlayMode() {
    const modes: Array<"sequential" | "shuffle" | "single_loop" | "list_loop"> = ["sequential", "shuffle", "single_loop", "list_loop"];
    const idx = modes.indexOf(playerService.playMode);
    playerService.setPlayMode(modes[(idx + 1) % modes.length]);
  }

  const playModeIcon = $derived({
    sequential: Repeat,
    shuffle: Shuffle,
    single_loop: Repeat1,
    list_loop: Repeat,
  }[playerService.playMode]);

  const song = $derived(playerService.currentSong);
  const hasSong = $derived(song != null);
</script>

<svelte:window onmousemove={onProgressMouseMove} onmouseup={onProgressMouseUp} />

<!-- Player bar (floating card, fixed to viewport) -->
{#if hasSong}
  <div
    class="fixed bottom-3 left-3 right-3 h-20 rounded-xl border border-border/50 bg-background/95 backdrop-blur shadow-lg flex items-center gap-4 px-4 z-40"
    in:fly={{ y: 16, duration: 300, opacity: 0 }}
  >
  <!-- Left: song info (clickable → opens full player) -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="flex items-center gap-3 w-56 shrink-0 cursor-pointer rounded-lg hover:bg-muted/50 transition-colors -ml-2 pl-2 py-1"
    onclick={() => (playerUI.showFullPlayer = true)}
    onkeydown={() => {}}
    role="button"
    tabindex="0"
    aria-label="打开全屏播放器"
  >
    <div class="size-12 shrink-0 rounded-md bg-muted overflow-hidden">
      {#if song?.avatarUrl}
        <img src={coverSrc(song.avatarUrl)} alt="" class="size-full object-cover" />
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
          <!-- Playhead progress -->
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

  <!-- Right: play mode + volume + queue -->
  <div class="flex items-center gap-1 w-54 justify-end shrink-0">
    <Button variant="ghost" size="icon" class="cursor-pointer" disabled={!hasSong} onclick={togglePlayMode}>
      <svelte:component this={playModeIcon} class="size-4 text-muted-foreground" />
    </Button>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="relative flex items-center" onmouseenter={showVolume} onmouseleave={scheduleHideVolume}>
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
        <!-- Transparent hit-area buffer -->
        <div class="absolute bottom-full left-1/2 -translate-x-1/2 pb-2 pt-6 px-8" onmouseenter={showVolume} onmouseleave={scheduleHideVolume}>
          <div class="p-2 bg-background border border-border rounded-lg shadow-lg" in:fly={{ y: -8, duration: 200, opacity: 0 }}>
            <input
              type="range"
              min="0" max="100" value={playerService.volume * 100}
              oninput={(e) => playerService.setVolume(Number(e.currentTarget.value) / 100)}
              class="h-20 w-6 cursor-pointer appearance-none bg-muted rounded-full [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
              style="-webkit-appearance: slider-vertical; writing-mode: vertical-lr; direction: rtl;"
            />
          </div>
        </div>
      {/if}
    </div>

    <Button variant="ghost" size="icon" class="cursor-pointer" href="/playqueue">
      <ListMusic class="size-4 text-muted-foreground" />
    </Button>
  </div>
</div>
{/if}
