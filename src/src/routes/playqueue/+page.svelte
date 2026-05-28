<script lang="ts">
  import { onMount } from "svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import SongCard from "$lib/components/cards/song-nextup-card.svelte";
  import { ListMusic, Trash2 } from "@lucide/svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import type { Song } from "$lib/types";
  import { fly } from "svelte/transition";

  let queue = $state<Song[]>([]);
  let currentIndex = $state(-1);

  onMount(async () => {
    try {
      queue = await playerService.getQueue();
    } catch {
      queue = [];
    }
  });

  // Reactively sync queue
  $effect(() => {
    // Re-fetch when current song changes
    const _ = playerService.currentSong;
    playerService.getQueue().then((q) => {
      queue = q;
      currentIndex = q.findIndex((s) => s.id === playerService.currentSong?.id);
    }).catch(() => {});
  });

  function playAt(index: number) {
    playerService.play(queue, index);
  }
</script>

<div
  class="flex flex-col gap-4 px-8 py-6 h-full overflow-hidden"
  transition:fly={{ y: -20, duration: 200 }}
>
  <div class="flex items-center justify-between shrink-0">
    <div class="flex items-center gap-2">
      <ListMusic class="size-5 text-foreground" />
      <h1 class="text-xl font-bold text-foreground">播放队列</h1>
      {#if queue.length > 0}
        <span class="text-xs text-muted-foreground">{queue.length} 首</span>
      {/if}
    </div>
    {#if queue.length > 0}
      <Button variant="ghost" size="sm" class="cursor-pointer text-xs text-muted-foreground">
        <Trash2 class="size-3.5 mr-1" />
        清空
      </Button>
    {/if}
  </div>

  <ScrollArea class="flex-1 min-h-0">
    {#if queue.length === 0}
      <div class="text-sm text-muted-foreground text-center py-16">
        播放队列为空
      </div>
    {:else}
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-2">
        {#each queue as song, i}
          <div
            class="shrink-0"
            class:opacity-50={i < currentIndex}
            class:ring-1={i === currentIndex}
            class:ring-primary={i === currentIndex}
            class:rounded-lg={i === currentIndex}
          >
            <SongCard {song} onplay={() => playAt(i)} />
          </div>
        {/each}
      </div>
    {/if}
  </ScrollArea>
</div>
