<script lang="ts">
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import SongCard from "$lib/components/cards/song/song-nextup-card.svelte";
  import type { Song } from "$lib/types";

  interface Props {
    songs: Song[];
    loading?: boolean;
    onplay?: (song: Song, index: number) => void;
    class?: string;
  }

  let { songs, loading = false, onplay, class: className = "" }: Props = $props();
</script>

<ScrollArea class="flex gap-2 flex-1 min-h-0 {className}">
  {#if loading}
    <div class="text-sm text-muted-foreground p-4">加载中...</div>
  {:else if songs.length === 0}
    <div class="text-sm text-muted-foreground p-4">暂无歌曲</div>
  {:else}
    {#each songs as song, i}
      <div class="shrink-0 mb-2">
        <SongCard {song} onplay={() => onplay?.(song, i)} />
      </div>
    {/each}
  {/if}
  <div class="h-24"></div>
</ScrollArea>
