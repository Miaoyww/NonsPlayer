<script lang="ts">
  import { onMount } from "svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import SongList from "$lib/components/song-list.svelte";
  import { ListMusic, Trash2 } from "@lucide/svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import type { Song } from "$lib/types";
  import { fly } from "svelte/transition";

  let queue = $state<Song[]>([]);

  onMount(async () => {
    try {
      queue = await playerService.getQueue();
    } catch {
      queue = [];
    }
  });

  // Reactively sync queue when current song changes
  $effect(() => {
    const _ = playerService.currentSong;
    playerService.getQueue().then((q) => { queue = q; }).catch(() => {});
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
      <SongList songs={queue} onplay={playAt} />
    {/if}
  </ScrollArea>
</div>
