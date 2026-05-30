<script lang="ts">
  import { Play, Heart, Ellipsis } from "@lucide/svelte";
  import Button from "../../ui/button/button.svelte";
  import type { Playlist } from "$lib/types";
  import { coverSrc } from "$lib/utils";

  interface Props {
    playlist: Playlist;
    onplay?: () => void;
    onlike?: () => void;
  }

  let { playlist, onplay, onlike }: Props = $props();

  let liked = $state(false);

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
  <div class=" size-16 shrink-0 rounded-lg bg-muted overflow-hidden">
    {#if playlist.avatarUrl}
      <img
        src={coverSrc(playlist.avatarUrl)}
        alt=""
        class="size-full object-cover"
      />
    {/if}
  </div>

  <!-- 歌单名 + 创建者 -->
  <div class="flex flex-col justify-center gap-2 min-w-0 w-52 shrink-0">
    <p class="text-sm font-medium truncate">{playlist.name}</p>
    <p class="text-xs font-medium text-gray-500 truncate">{playlist.creator}</p>
  </div>

  <!-- 右侧：播放数 + 曲目数 + 操作按钮 -->
  <div class="flex-1 flex items-center justify-end gap-2 h-full m-2 ml-auto">
    {#if playlist.playCount > 0}
      <div class="flex items-center gap-px w-10">
        <Play class="size-3 text-gray-500 fill-blue-500 stroke-blue-500" />
        <span class="text-xs font-medium text-gray-500"
          >{playlist.playCount}</span
        >
      </div>
    {/if}

    {#if playlist.musicsCount > 0}
      <span class="text-xs font-medium text-gray-500 w-16"
        >{playlist.musicsCount} Tracks</span
      >
    {/if}

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
