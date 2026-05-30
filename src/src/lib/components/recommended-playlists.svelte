<script lang="ts">
  import { goto } from "$app/navigation";
  import { ChevronRight, ListMusic } from "@lucide/svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import PlaylistRowCard from "$lib/components/cards/playlist/playlist-row-card.svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { getRecommendedPlaylists } from "$lib/services/adapter-service";
  import type { Playlist } from "$lib/types";
  import { Button } from "./ui/button";

  let playlists = $state<Playlist[]>([]);
  let loading = $state(true);
  let loaded = false;

  $effect(() => {
    if (loaded) return;
    const online = adapterStore.streaming;
    if (online.length === 0) {
      loading = false;
      return;
    }

    loaded = true;
    Promise.all(
      online.map((a) =>
        getRecommendedPlaylists(a.slug, 10).catch(() => [] as Playlist[]),
      ),
    )
      .then((results) => {
        playlists = results.flat().slice(0, 10);
      })
      .finally(() => {
        loading = false;
      });
  });
</script>

<div class="flex flex-col gap-3 min-h-0 overflow-hidden">
  <div class="flex items-center gap-1 shrink-0">
    <ListMusic class="size-6 text-foreground" />
    <p class="text-base font-bold text-foreground">推荐歌单</p>
    <Button variant="ghost" class="cursor-pointer" size="icon" href="/explore">
      <ChevronRight class="size-4 text-foreground" />
    </Button>
  </div>
  <ScrollArea class="flex flex-col gap-2 flex-1 min-h-0">
    {#if loading}
      <div class="text-sm text-muted-foreground p-4">加载中...</div>
    {:else if playlists.length === 0}
      <div class="text-sm text-muted-foreground p-4">暂无推荐歌单</div>
    {:else}
      {#each playlists as p}
        <div class="mb-2">
          <PlaylistRowCard
            playlist={p}
            onplay={() =>
              goto(
                `/adapter/${p.adapterSlug}/playlist/${encodeURIComponent(p.id)}`,
              )}
          />
        </div>
      {/each}
    {/if}
    <div class="h-24"></div>
  </ScrollArea>
</div>
