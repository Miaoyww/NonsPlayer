<script lang="ts">
  import { onMount } from "svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { lyricService } from "$lib/services/lyric-service.svelte";
  import { globalSettings } from "$lib/stores/global-settings-store";
  import { useAudioManager } from "$lib/player/AudioManager";
  import { coverSrc } from "$lib/utils";
  import BackgroundRender from "./BackgroundRender.svelte";

  const audioManager = useAudioManager();

  let coverUrl = $derived(playerService.currentSong?.avatarUrl ?? "");
  let hasLyric = $derived(lyricService.currentLyricLines.length > 0);

  // 低频音量（对应 Vue: ref(1.0)）
  let lowFreqVolume = $state(1.0);

  // flowSpeed: 暂停时如果开启了"暂停时暂停"则返回0
  let flowSpeed = $derived(
    !playerService.isPlaying && $globalSettings.playerBackgroundPause
      ? 0
      : ($globalSettings.playerBackgroundFlowSpeed ?? 4),
  );

  // RAF 轮询低频音量（对应 Vue: useRafFn）
  let lfRafId: number | null = null;
  function startLowFreqPolling() {
    const tick = () => {
      lowFreqVolume = audioManager.getLowFrequencyVolume();
      lfRafId = requestAnimationFrame(tick);
    };
    lfRafId = requestAnimationFrame(tick);
  }
  function stopLowFreqPolling() {
    if (lfRafId !== null) {
      cancelAnimationFrame(lfRafId);
      lfRafId = null;
    }
  }

  // 启停 RAF（对应 Vue: watch + immediate:true）
  $effect(() => {
    const enabled = $globalSettings.playerBackgroundLowFreqVolume;
    const isAnimation = $globalSettings.playerBackgroundType === "animation";
    const isPlaying = playerService.isPlaying;

    if (enabled && isAnimation && isPlaying) {
      if (lfRafId === null) startLowFreqPolling();
    } else {
      stopLowFreqPolling();
      lowFreqVolume = 1.0;
    }
  });

  onMount(() => {
    return () => stopLowFreqPolling();
  });
</script>

<!-- 对应 Vue: div.background + z-index:-1 -->
<div class="absolute inset-0 w-full h-full overflow-hidden z-[-1]">
  {#if $globalSettings.playerBackgroundType === "color"}
    <!-- 纯色背景（对应 Vue: v-if="color" + .color class） -->
    <div
      class="absolute inset-0"
      style="background-color: rgb(var(--main-cover-color))"
    ></div>
  {:else if $globalSettings.playerBackgroundType === "blur" && coverUrl}
    <!-- 模糊封面（对应 Vue: v-else-if="blur" + s-image.bg-img） -->
    <div class="flex items-center justify-center absolute inset-0">
      <img
        src={coverSrc(coverUrl)}
        alt=""
        class="w-full h-auto scale-150 blur-[80px] contrast-125"
      />
    </div>
  {:else if $globalSettings.playerBackgroundType === "animation"}
    <!-- AMLL 流体背景（对应 Vue: v-else-if="animation" + BackgroundRender） -->
    <BackgroundRender
      album={coverUrl}
      playing={playerService.isPlaying}
      {hasLyric}
      {lowFreqVolume}
      fps={$globalSettings.playerBackgroundFps ?? 30}
      {flowSpeed}
      renderScale={$globalSettings.playerBackgroundRenderScale ?? 0.5}
    />
  {/if}

  <!-- 半透明叠加层（对应 Vue: ::after 伪元素，animation 模式关闭） -->
  {#if $globalSettings.playerBackgroundType !== "animation"}
    <div class="absolute inset-0 bg-black/50 backdrop-blur-[20px]"></div>
  {/if}
</div>
