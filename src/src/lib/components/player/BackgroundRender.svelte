<script lang="ts">
  import { BackgroundRender, MeshGradientRenderer } from "@applemusic-like-lyrics/core";
  import { onMount } from "svelte";

  interface Props {
    album?: string;
    playing?: boolean;
    fps?: number;
    flowSpeed?: number;
    hasLyric?: boolean;
    lowFreqVolume?: number;
    renderScale?: number;
    staticMode?: boolean;
  }

  let {
    album = "",
    playing = false,
    fps = 30,
    flowSpeed = 4,
    hasLyric = true,
    lowFreqVolume = 1,
    renderScale = 0.5,
    staticMode = false,
  }: Props = $props();

  let containerEl = $state<HTMLDivElement>();
  let bgRender: BackgroundRender | null = $state(null);

  onMount(() => {
    if (!containerEl) return;

    bgRender = BackgroundRender.new(MeshGradientRenderer);

    const el = bgRender.getElement();
    el.style.cssText = "position:absolute;inset:0;width:100%;height:100%;display:block;";
    containerEl.appendChild(el);

    // Initial config
    bgRender.setFPS(fps);
    bgRender.setFlowSpeed(flowSpeed);
    bgRender.setRenderScale(renderScale);
    bgRender.setHasLyric(hasLyric);
    bgRender.setLowFreqVolume(lowFreqVolume);
    if (album) {
      bgRender.setAlbum(album).catch(() => {});
    }
    if (playing) bgRender.resume();
    else bgRender.pause();

    return () => {
      bgRender?.dispose();
      bgRender = null;
    };
  });

  // Sync each prop independently (mirrors Vue's per-prop watch pattern)
  $effect(() => {
    if (bgRender && album) {
      bgRender.setAlbum(album).catch(() => {});
    }
  });

  $effect(() => {
    if (!bgRender) return;
    if (playing) bgRender.resume();
    else bgRender.pause();
  });

  $effect(() => { bgRender?.setFPS(fps); });
  $effect(() => { bgRender?.setFlowSpeed(flowSpeed); });
  $effect(() => { bgRender?.setRenderScale(renderScale); });
  $effect(() => { bgRender?.setHasLyric(hasLyric); });
  $effect(() => { bgRender?.setLowFreqVolume(lowFreqVolume); });
  $effect(() => { bgRender?.setStaticMode(staticMode); });
</script>

<div bind:this={containerEl} class="absolute inset-0 w-full h-full overflow-hidden"></div>
