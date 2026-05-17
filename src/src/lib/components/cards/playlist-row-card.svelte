<script lang="ts">
  import { Play, Heart, Ellipsis } from "@lucide/svelte";
  import Button from "../ui/button/button.svelte";

  interface Props {
    playlistName: string;
    creator: string;
    coverUrl?: string;
    playCount?: string;
    trackCount?: string;
    liked?: boolean;
    onplay?: () => void;
    onlike?: () => void;
  }

  let {
    playlistName,
    creator,
    coverUrl = "",
    playCount = "",
    trackCount = "",
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
  class="group flex items-center gap-2 w-full rounded-lg hover:bg-muted/50 transition-colors cursor-pointer text-left"
  onclick={() => onplay?.()}
  onkeydown={(e) => e.key === "Enter" && onplay?.()}
  role="button"
  tabindex="0"
>
  <!-- 封面 93x93 -->
  <div class=" size-20 shrink-0 rounded-lg bg-muted overflow-hidden">
    {#if coverUrl}
      <img src={coverUrl} alt="" class="size-full object-cover" />
    {/if}
  </div>

  <!-- 歌单名 + 创建者 -->
  <div class="flex flex-col justify-center gap-2 min-w-0 w-52 shrink-0">
    <p class="text-sm font-medium truncate">{playlistName}</p>
    <p class="text-xs font-medium text-gray-500 truncate">{creator}</p>
  </div>

  <!-- 右侧：播放数 + 曲目数 + 操作按钮 -->
  <div class="flex-1 flex items-center justify-end gap-2 h-full m-2 ml-auto">
    <!-- 播放数 -->
    <div class="flex items-center gap-px w-10">
      <Play class="size-3 text-gray-500 fill-blue-500 stroke-blue-500"  />
      <span class="text-xs font-medium text-gray-500">{playCount}</span>
    </div>

    <!-- 曲目数 -->
    <span class="text-xs font-medium text-gray-500 w-16">{trackCount}</span>

    <!-- 收藏按钮 -->
    <Button
      class="shrink-0 p-0 cursor-pointer"
      variant="ghost"
      size="icon"
      aria-label={liked ? "取消收藏" : "收藏"}
      onclick={handleLike}
    >
      <Heart class="size-5 fill-blue-500 stroke-blue-500" />
    </Button>

    <!-- 更多按钮 -->
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
