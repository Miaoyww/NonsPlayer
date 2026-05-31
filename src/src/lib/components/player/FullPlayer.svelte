<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { playerUI } from "$lib/stores/player-ui-store.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { lyricService } from "$lib/services/lyric-service.svelte";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import PlayerBackground from "./PlayerBackground.svelte";
  import PlayerMenu from "./PlayerMenu.svelte";
  import PlayerCover from "./PlayerCover.svelte";
  import PlayerData from "./PlayerData.svelte";
  import PlayerControl from "./PlayerControl.svelte";
  import LyricPlayer from "$lib/components/lyrics/LyricPlayer.svelte";

  // ── Reactive state from services ──
  let song = $derived(playerService.currentSong);
  let isPlaying = $derived(playerService.isPlaying);
  let coverUrl = $derived(song?.avatarUrl ?? "");
  let hasLyrics = $derived(lyricService.currentLyricLines.length > 0);

  // ── Derived layout state ──
  let isFullscreenType = $derived($globalSettings.playerType === "fullscreen");
  let leftWidth = $derived(`${$globalSettings.playerStyleRatio}%`);

  // ── Auto-hide timer ──
  let autoHideTimer: ReturnType<typeof setTimeout> | null = null;

  function resetAutoHide() {
    playerUI.playerMetaShow = true;
    if (autoHideTimer) clearTimeout(autoHideTimer);
    if ($globalSettings.autoHidePlayerMeta) {
      autoHideTimer = setTimeout(() => {
        playerUI.playerMetaShow = false;
      }, 3000);
    }
  }

  function onPlayerMouseMove() {
    resetAutoHide();
  }

  function onPlayerMouseLeave() {
    if ($globalSettings.autoHidePlayerMeta) {
      playerUI.playerMetaShow = false;
      if (autoHideTimer) clearTimeout(autoHideTimer);
    }
  }

  function close() {
    playerUI.showFullPlayer = false;
  }

  // ── Keyboard shortcuts ──
  function onKeyDown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    switch (e.key) {
      case "Escape":
        e.preventDefault();
        close();
        break;
      case " ":
        e.preventDefault();
        playerService.togglePlayback();
        break;
      case "ArrowLeft":
        e.preventDefault();
        playerService.seek(Math.max(0, playerService.position - 5));
        break;
      case "ArrowRight":
        e.preventDefault();
        playerService.seek(Math.min(playerService.duration, playerService.position + 5));
        break;
    }
  }

  // ── Lyrics: update when song changes ──
  let lastLyricSongId = $state("");
  $effect(() => {
    const s = song;
    if (!s) {
      lyricService.updateLyric(null);
      return;
    }
    if (s.id !== lastLyricSongId) {
      lastLyricSongId = s.id;
      lyricService.updateLyric(s);
      const next = playerService.getNextSong();
      if (next) lyricService.prefetch(next);
    }
  });

  // ── Lyric line click → seek ──
  function handleLineClick(_index: number, line: { startTime: number }) {
    playerService.seek(line.startTime / 1000);
  }

  // ── Lifecycle ──
  onMount(() => {
    resetAutoHide();
    return () => {
      if (autoHideTimer) clearTimeout(autoHideTimer);
    };
  });
</script>

<svelte:window onkeydown={onKeyDown} />

{#if playerUI.showFullPlayer}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="fixed inset-0 z-[1000] flex flex-col overflow-hidden text-white"
    style="background-color: #00000060; backdrop-filter: blur(80px);"
    role="dialog"
    aria-label="全屏播放器"
    onmousemove={onPlayerMouseMove}
    onmouseleave={onPlayerMouseLeave}
    in:fly={{ y: 16, duration: 300, opacity: 0 }}
    out:fly={{ y: 16, duration: 300, opacity: 0 }}

  >
    <!-- 背景层 -->
    <PlayerBackground />

    <!-- 顶部菜单栏 -->
    <PlayerMenu
      show={playerUI.playerMetaShow}
      onClose={close}
    />

    <!-- 主内容区 -->
    <div class="flex-1 flex items-center justify-center">
      <!-- 左右分栏 -->
      <div class="flex w-full h-full">
        <!-- 左侧：封面 + 数据 + 控制栏 -->
        {#if !isFullscreenType}
          <div
            class="flex flex-col items-center justify-center gap-6 px-8 h-full"
            style="width: {leftWidth}"
            transition:fly|local={{ duration: 300 }}
          >
            {#key song?.id}
              <PlayerCover
                playerType={$globalSettings.playerType}
                {isPlaying}
                {coverUrl}
              />
            {/key}

            <div transition:fly|local={{ y: 20, duration: 300 }}>
              <PlayerData
                songName={song?.name ?? ""}
                artists={song?.artistsName ?? ""}
                album={song?.albumName ?? ""}
                center={true}
                light={true}
              />
            </div>

            <!-- 控制栏（左侧面板内） -->
            <div class="w-full max-w-[70vh]">
              <PlayerControl
                light={true}
                onMouseEnter={() => {
                  if (autoHideTimer) clearTimeout(autoHideTimer);
                  playerUI.playerMetaShow = true;
                }}
                onMouseLeave={() => resetAutoHide()}
              />
            </div>
          </div>
        {/if}

        <!-- 右侧：Lyrics / 空状态 -->
        <div
          class="flex flex-col items-center justify-center h-full px-8"
          style="width: {isFullscreenType ? '100%' : `${100 - $globalSettings.playerStyleRatio}%`}"
          class:items-center={!hasLyrics}
        >
          {#if hasLyrics}
            <div class="flex-1 w-full min-h-0">
              <LyricPlayer
                lyricLines={lyricService.currentLyricLines}
                currentTime={playerService.position * 1000}
                {isPlaying}
                enableBlur={$globalSettings.lyricEnableBlur}
                enableScale={$globalSettings.lyricEnableScale}
                enableSpring={$globalSettings.lyricEnableSpring}
                wordFadeWidth={$globalSettings.lyricWordFadeWidth}
                hidePassedLines={$globalSettings.lyricHidePassedLines}
                alignAnchor={$globalSettings.lyricAlignAnchor}
                alignPosition={$globalSettings.lyricAlignPosition}
                onLineClick={handleLineClick}
              />
            </div>
          {:else}
            <!-- 无歌词状态 -->
            <div
              class="flex flex-col items-center gap-4"
              class:translate-y-[30vh]={isFullscreenType}
            >
              <PlayerData
                songName={song?.name ?? ""}
                artists={song?.artistsName ?? ""}
                album={song?.albumName ?? ""}
                center={true}
                light={true}
              />
              <p class="text-white/40 text-sm">暂无歌词</p>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- 全屏封面模式：控制栏保持底部 -->
    {#if isFullscreenType}
      <div
        class="transition-opacity duration-300"
        class:opacity-100={playerUI.playerMetaShow}
        class:opacity-0={!playerUI.playerMetaShow}
        class:pointer-events-none={!playerUI.playerMetaShow}
      >
        <PlayerControl
          light={true}
          onMouseEnter={() => {
            if (autoHideTimer) clearTimeout(autoHideTimer);
            playerUI.playerMetaShow = true;
          }}
          onMouseLeave={() => resetAutoHide()}
        />
      </div>
    {/if}
  </div>
{/if}
