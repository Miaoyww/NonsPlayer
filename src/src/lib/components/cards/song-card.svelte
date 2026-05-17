<script lang="ts">
  import { Play, Heart, Ellipsis } from "@lucide/svelte";
  import Button from "../ui/button/button.svelte";

  interface Props {
    songName: string;
    artist: string;
    coverUrl?: string;
    duration?: string;
    liked?: boolean;
    album?: string;
    onplay?: () => void;
    onlike?: () => void;
  }

  let {
    songName,
    artist,
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

<div
  class="group flex items-center gap-3 w-full rounded-xl p-3 hover:bg-muted/50 transition-colors cursor-pointer text-left"
>
  <!-- 封面 -->
  <div>
    <img src={coverUrl} alt="" class="size-15 rounded-lg" />
  </div>

  <!-- 歌曲信息 -->
  <div class="flex-1 min-w-0">
    <p class="text-sm font-semibold text-foreground truncate">{songName}</p>
    <p class="text-sm text-muted-foreground truncate">{artist}</p>
  </div>

  <div class="flex-1 min-w-0">
    <p class="text-sm font-semibold text-foreground truncate">{album}</p>
  </div>

  <div class="flex items-center gap-2">
    <!-- 时长 -->
    {#if duration}
      <span class="text-sm text-muted-foreground shrink-0">{duration}</span>
      <!-- 收藏按钮 -->
      <Button
        class="shrink-0 p-1 transition-opacity cursor-pointer"
        variant="ghost"
        size="icon"
        aria-label={liked ? "取消收藏" : "收藏"}
        onclick={handleLike}
      >
        <Heart class="size-4 fill-blue-500 stroke-blue-500" />
      </Button>
      <Button
        class="shrink-0 p-1 transition-opacity cursor-pointer"
        variant="ghost"
        size="icon"
        aria-label={liked ? "取消收藏" : "收藏"}
        onclick={handleLike}
      >
        <Ellipsis class="size-4" />
      </Button>
    {/if}
  </div>
</div>
