<script lang="ts">
  import { SkipForward, ChevronRight } from "@lucide/svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import SongCard from "$lib/components/cards/song/song-nextup-card.svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { search } from "$lib/services/adapter-service";
  import type { Song } from "$lib/types";

  let songs = $state<Song[]>([]);
  let loading = $state(true);
  let loaded = false;

  $effect(() => {
    if (loaded) return;
    const adapters = adapterStore.adapters;
    if (adapters.length === 0) return;

    loaded = true;
    Promise.all(adapters.map((a) => search(a.slug, "").then((r) => r.songs).catch(() => [] as Song[])))
      .then((results) => { songs = results.flat().slice(0, 9); })
      .finally(() => { loading = false; });
  });

  function playSong(song: Song) {
    playerService.play(songs, songs.indexOf(song));
  }
</script>

<div class="flex flex-col gap-3 min-h-0 overflow-hidden">
  <div class="flex items-center gap-1 shrink-0">
    <SkipForward class="size-4 text-foreground" />
    <p class="text-base font-bold text-foreground">下一首播放</p>
    <Button variant="ghost" class="cursor-pointer" size="icon" href="/playqueue">
      <ChevronRight class="size-4 text-foreground" />
    </Button>
  </div>
  <ScrollArea class="flex gap-2 flex-1 min-h-0">
    {#if loading}
      <div class="text-sm text-muted-foreground p-4">加载中...</div>
    {:else if songs.length === 0}
      <div class="text-sm text-muted-foreground p-4">暂无歌曲</div>
    {:else}
      {#each songs as song, i}
        <div class="shrink-0 mb-2">
          <SongCard {song} onplay={() => playSong(song)} />
        </div>
      {/each}
    {/if}
    <div class="h-24"></div>
  </ScrollArea>
</div>
