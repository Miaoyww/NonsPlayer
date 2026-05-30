<script lang="ts">
  import { coverSrc } from "$lib/utils";

  interface Props {
    playerType?: "fullscreen" | "cover" | "record";
    isPlaying?: boolean;
    coverUrl?: string;
  }

  let {
    playerType = "cover",
    isPlaying = false,
    coverUrl = "",
  }: Props = $props();
</script>

{#if playerType === "fullscreen"}
  <!-- 全屏封面: 60vw 宽 + 渐变遮罩 -->
  <div
    class="fixed left-0 top-0 h-screen z-0"
    style="width: 60vw; mask-image: linear-gradient(to right, #000 60%, transparent 100%); -webkit-mask-image: linear-gradient(to right, #000 60%, transparent 100%);"
  >
    {#if coverUrl}
      <img
        src={coverSrc(coverUrl)}
        alt=""
        class="w-full h-full object-cover"
      />
    {:else}
      <div class="w-full h-full bg-linear-to-br from-primary/30 to-primary/10"></div>
    {/if}
  </div>
{:else if playerType === "record"}
  <!-- 黑胶唱片模式：旋转唱片 + 指针 -->
  <div class="relative flex items-center justify-center w-[70%] max-w-[46vh] aspect-square mb-[4%]">
    <!-- CSS 指针（模拟唱臂） -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="absolute w-[30%] left-[46%] top-[-22%] z-[2] transition-transform duration-300 rotate-[-20deg] origin-[10%_10%]"
      class:rotate-[-8deg]={isPlaying}
    >
      <div class="relative w-full">
        <!-- 唱臂杆 -->
        <div class="h-1 w-full bg-white/80 rounded-full rotate-45 origin-left shadow-md"></div>
        <!-- 唱头 -->
        <div class="absolute -right-1 -top-0.75 size-2.5 rounded-full bg-white/90 shadow"></div>
      </div>
    </div>
    <!-- 唱片 -->
    <div
      class="relative w-full h-full rounded-full overflow-hidden animate-[playerCoverSpin_30s_linear_infinite]"
      class:paused={!isPlaying}
    >
      <!-- 黑胶纹理 -->
      <div
        class="w-full h-full rounded-full p-[15%]"
        style="background: radial-gradient(circle, #333 0%, #111 50%, #222 51%, #333 52%, #111 53%, #222 54%, #333 55%, #111 56%, #222 57%, #333 58%, #111 59%, #222 60%, #333 61%, #111 62%, #222 63%, #333 64%, #111 65%, #222 66%, #333 67%, #111 68%, #222 69%, #333 70%, #111 71%, #222 72%, #333 73%, #111 74%, #222 75%, #333 76%, #111 77%, #222 78%, #333 79%, #111 80%, #222 81%, #333 82%, #111 83%, #222 84%, #333 85%, #111 86%, #222 87%, #333 88%, #111 89%, #222 90%, #333 91%, #111 92%, #222 93%, #333 94%, #111 95%, #222 96%, #333 97%, #111 98%, #222 99%, #333 100%);"
      >
        <!-- 中心封面图 -->
        <div class="w-full h-full rounded-full overflow-hidden border-[1vh] border-white/30">
          {#if coverUrl}
            <img
              src={coverSrc(coverUrl)}
              alt=""
              class="w-full h-full object-cover rounded-full"
            />
          {:else}
            <div class="w-full h-full rounded-full bg-linear-to-br from-primary/30 to-primary/10"></div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{:else}
  <!-- 普通封面模式: 圆角方形 + 播放缩放动画 -->
  <div
    class="relative w-[70%] max-w-[50vh] aspect-square rounded-[32px] overflow-hidden shadow-[0_0_20px_10px_rgba(0,0,0,0.1)] transition-transform duration-500"
    class:scale-100={!isPlaying}
    class:scale-[1.02]={isPlaying}
  >
    {#if coverUrl}
      <img
        src={coverSrc(coverUrl)}
        alt=""
        class="w-full h-full object-cover"
      />
    {:else}
      <div class="w-full h-full bg-linear-to-br from-primary/30 to-primary/10"></div>
    {/if}
  </div>
{/if}

<style>
  @keyframes playerCoverSpin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
