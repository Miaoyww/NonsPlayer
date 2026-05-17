<script lang="ts">
  import { Heart, Ellipsis } from "@lucide/svelte";
  import Button from "../ui/button/button.svelte";

  interface Props {
    songName: string;
    artist: string;
    alias?: string;
    coverUrl?: string;
    duration?: string;
    album?: string;
    liked?: boolean;
    onplay?: () => void;
    onlike?: () => void;
  }

  let {
    songName,
    artist,
    alias = "",
    coverUrl = "",
    duration = "",
    album = "",
    liked = $bindable(false),
    onplay,
    onlike,
  }: Props = $props();

  function handleLike(e: MouseEvent) {
    e.stopPropagation();
    liked = !liked;
    onlike?.();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="group flex items-center gap-4 w-full h-full rounded-lg hover:bg-muted/50 transition-colors cursor-pointer text-left"
  onclick={() => onplay?.()}
  onkeydown={(e) => e.key === "Enter" && onplay?.()}
  role="button"
  tabindex="0"
>
  <!-- 封面 44x44 -->
  <div class="size-12 shrink-0 rounded-lg bg-muted overflow-hidden">
    {#if coverUrl}
      <img src={coverUrl} alt="" class="size-full object-cover" />
    {/if}
  </div>

  <!-- 歌曲名 + 别名 + 歌手 -->
  <div class="flex flex-col justify-center w-80 ">
    <!-- 歌名行：歌名 + 别名 -->
    <div class="flex items-center gap-1 min-w-0">
      <p class="text-base font-medium truncate">{songName}</p>
      {#if alias}
        <p class="text-xs text-gray-400 truncate shrink-0">{alias}</p>
      {/if}
    </div>
    <!-- 歌手行 -->
    <p class="text-sm font-medium text-[#747474] truncate">{artist}</p>
  </div>

  <!-- 右侧：时长 + 操作按钮 -->
  <div class="flex items-center w-30 gap-5 shrink-0 m-2 ml-auto">
    {#if duration}
      <span class="text-sm font-medium text-foreground">{duration}</span>
    {/if}
    <Button
      class="shrink-0 p-0 cursor-pointer"
      variant="ghost"
      size="icon"
      aria-label={liked ? "取消收藏" : "收藏"}
      onclick={handleLike}
    >
      <Heart class="size-5 fill-blue-500 stroke-blue-500" />
    </Button>
    <Button
      class="shrink-0 p-0 size-3 cursor-pointer"
      variant="ghost"
      size="icon"
      aria-label="更多"
    >
      <Ellipsis class="size-5" />
    </Button>
  </div>
</div>
