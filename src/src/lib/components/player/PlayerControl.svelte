<script lang="ts">
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Volume2,
    VolumeX,
    ListMusic,
    Shuffle,
    Repeat,
    Repeat1,
    Settings,
  } from "@lucide/svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { goto } from "$app/navigation";
  import { playerUI } from "$lib/stores/player-ui-store.svelte";
  import { settingsDialogOpen } from "$lib/stores/global-ui-store";
  import { fly } from "svelte/transition";

  interface Props {
    light?: boolean;
    onMouseEnter?: () => void;
    onMouseLeave?: () => void;
  }

  let {
    light = false,
    onMouseEnter = () => {},
    onMouseLeave = () => {},
  }: Props = $props();

  // ── Progress bar drag ──
  let progressBar = $state<HTMLDivElement>();
  let dragging = $state(false);
  let dragPercent = $state(0);

  const progress = $derived(
    playerService.duration > 0
      ? dragging
        ? dragPercent
        : (playerService.position / playerService.duration) * 100
      : 0,
  );

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
    playerService.seek((dragPercent / 100) * playerService.duration);
  }

  function updateDrag(e: MouseEvent) {
    if (!progressBar) return;
    const rect = progressBar.getBoundingClientRect();
    const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width));
    dragPercent = (x / rect.width) * 100;
  }

  // ── Play mode ──
  function cyclePlayMode() {
    const modes: Array<"sequential" | "shuffle" | "single_loop" | "list_loop"> =
      ["sequential", "shuffle", "single_loop", "list_loop"];
    const idx = modes.indexOf(playerService.playMode);
    playerService.setPlayMode(modes[(idx + 1) % modes.length]);
  }

  const playModeIcon = $derived(
    {
      sequential: Repeat,
      shuffle: Shuffle,
      single_loop: Repeat1,
      list_loop: Repeat,
    }[playerService.playMode],
  );

  const hasSong = $derived(playerService.currentSong != null);


  let showVolumeSlider = $state(false);
  let volumeTimer: ReturnType<typeof setTimeout> | null = null;

  function scheduleHideVolume() {
    volumeTimer = setTimeout(() => {
      showVolumeSlider = false;
    }, 200);
  }
  function cancelHideVolume() {
    if (volumeTimer) {
      clearTimeout(volumeTimer);
      volumeTimer = null;
    }
  }
  function showVolume() {
    cancelHideVolume();
    showVolumeSlider = true;
  }

  const textColor = $derived(light ? "text-white/70" : "text-muted-foreground");
  const progressBg = $derived(light ? "bg-white/20" : "bg-muted");
  const progressFill = $derived(light ? "bg-white" : "bg-primary");
</script>

<svelte:window
  onmousemove={onProgressMouseMove}
  onmouseup={onProgressMouseUp}
/>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="flex flex-col gap-4 w-full max-w-2xl mx-auto px-8 pb-6"
  onmouseenter={onMouseEnter}
  onmouseleave={onMouseLeave}
>
  <!-- Progress bar -->
  <div class="flex items-center gap-3 text-xs {textColor}">
    <span class="tabular-nums w-10 text-right"
      >{formatTime(playerService.position)}</span
    >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={progressBar}
      class="flex-1 h-1.5 rounded-full cursor-pointer relative group {progressBg}"
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
      <div
        class="absolute inset-y-0 -top-1 left-0 right-0 group-hover:h-3 transition-all"
      >
        <div
          class="h-full rounded-full transition-[width] duration-75 {progressFill}"
          style="width: {progress}%"
        ></div>
        <div
          class="absolute top-1/2 -translate-y-1/2 size-3 rounded-full opacity-0 group-hover:opacity-100 transition-opacity shadow {progressFill}"
          style="left: calc({progress}% - 6px)"
        ></div>
      </div>
    </div>
    <span class="tabular-nums w-10">{formatTime(playerService.duration)}</span>
  </div>

  <!-- Control buttons -->
  <div class="flex items-center justify-between">
    <!-- Left: settings + play mode -->
    <div class="flex items-center gap-2 w-32">
      <Button
        variant="ghost"
        size="icon"
        class="{textColor} cursor-pointer hover:text-white"
        onclick={() => settingsDialogOpen.set(true)}
      >
        <Settings class="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        class="{textColor} cursor-pointer hover:text-white"
        onclick={cyclePlayMode}
      >
        <svelte:component this={playModeIcon} class="size-4" />
      </Button>
    </div>

    <!-- Center: transport -->
    <div class="flex items-center gap-3">
      <Button
        variant="ghost"
        size="icon"
        class="{textColor} cursor-pointer hover:text-white"
        disabled={!hasSong}
        onclick={() => playerService.prev()}
      >
        <SkipBack class="size-6" />
      </Button>

      <Button
        variant="ghost"
        size="icon"
        class="cursor-pointer rounded-full size-14 {light
          ? 'bg-white/20 hover:bg-white/30'
          : 'bg-primary text-primary-foreground hover:bg-primary/90'}"
        disabled={!hasSong}
        onclick={() => playerService.togglePlayback()}
      >
        {#if playerService.isPlaying}
          <Pause class="size-7 fill-current {light ? 'text-white' : ''}" />
        {:else}
          <Play
            class="size-7 fill-current ml-0.5 {light ? 'text-white' : ''}"
          />
        {/if}
      </Button>

      <Button
        variant="ghost"
        size="icon"
        class="{textColor} cursor-pointer hover:text-white"
        disabled={!hasSong}
        onclick={() => playerService.next()}
      >
        <SkipForward class="size-6" />
      </Button>
    </div>

    <!-- Right: volume + queue -->
    <div class="flex items-center gap-2 w-32 justify-end">
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div
        class="relative flex items-center"
        onmouseenter={showVolume}
        onmouseleave={scheduleHideVolume}
        role="button"
      >
        <Button
          variant="ghost"
          size="icon"
          class="cursor-pointer"
          onclick={() =>
            playerService.setVolume(playerService.volume > 0 ? 0 : 0.8)}
        >
          {#if playerService.volume === 0}
            <VolumeX class="size-4 text-muted-foreground" />
          {:else}
            <Volume2 class="size-4 text-muted-foreground" />
          {/if}
        </Button>
        {#if showVolumeSlider}
          <!-- Transparent hit-area buffer -->
          <div
            class="absolute top-0 left-1/2 -translate-x-1/2 pt-8 pb-8 px-12 -mb-10"
            onmouseenter={showVolume}
            onmouseleave={scheduleHideVolume}
          >
            <div
              class="p-2 bg-background border border-border rounded-lg shadow-lg"
              in:fly={{ y: 8, duration: 200, opacity: 0 }}
            >
              <input
                type="range"
                min="0"
                max="100"
                value={playerService.volume * 100}
                oninput={(e) =>
                  playerService.setVolume(Number(e.currentTarget.value) / 100)}
                class="w-28 h-4 cursor-pointer appearance-none bg-muted rounded-full [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:size-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary"
              />
            </div>
          </div>
        {/if}
      </div>

      <Button
        variant="ghost"
        size="icon"
        class="{textColor} cursor-pointer hover:text-white"
        onclick={() => {
          goto("/playqueue");
          playerUI.showFullPlayer = false;
        }}
      >
        <ListMusic class="size-5" />
      </Button>
    </div>
  </div>
</div>
