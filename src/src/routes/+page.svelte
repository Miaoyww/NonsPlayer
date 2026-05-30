<script lang="ts">
  import { SkipForward, ListMusic, ChevronRight } from "@lucide/svelte";
  import { goto } from "$app/navigation";
  import FavoritePlaylistCard from "$lib/components/cards/playlist/favorite-playlist-card.svelte";
  import NextupSongList from "$lib/components/nextup-song-list.svelte";
  import PlaylistRowCard from "$lib/components/cards/playlist/playlist-row-card.svelte";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import { fly } from "svelte/transition";
  import Button from "$lib/components/ui/button/button.svelte";
  import GreetingQuote from "$lib/components/greeting-quote.svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { search, getRecommendedPlaylists } from "$lib/services/adapter-service";
  import type { Song, Playlist } from "$lib/types";

  let songs = $state<Song[]>([]);
  let recommendedPlaylists = $state<Playlist[]>([]);
  let loadingSongs = $state(true);
  let loadingPlaylists = $state(true);
  let songsLoaded = false;
  let playlistsLoaded = false;

  // Load "next up" songs independently
  $effect(() => {
    if (songsLoaded) return;
    const adapters = adapterStore.adapters;
    if (adapters.length === 0) return;

    songsLoaded = true;
    Promise.all(adapters.map((a) => search(a.slug, "").then((r) => r.songs).catch(() => [] as Song[])))
      .then((results) => { songs = results.flat().slice(0, 9); })
      .finally(() => { loadingSongs = false; });
  });

  // Load recommended playlists independently
  $effect(() => {
    if (playlistsLoaded) return;
    const adapters = adapterStore.adapters;
    if (adapters.length === 0) return;

    playlistsLoaded = true;
    Promise.all(adapters.map((a) => getRecommendedPlaylists(a.slug, 4).catch(() => [] as Playlist[])))
      .then((results) => { recommendedPlaylists = results.flat().slice(0, 4); })
      .finally(() => { loadingPlaylists = false; });
  });

  function goPlaylist(playlist: { id: string; adapterSlug: string }) {
    goto(`/adapter/${playlist.adapterSlug}/playlist/${encodeURIComponent(playlist.id)}`);
  }

  function playSong(song: Song) {
    playerService.play(songs, songs.indexOf(song));
  }
</script>

<div
  class="grid grid-rows-[auto_1fr] gap-4 p-8 h-full overflow-hidden"
  transition:fly={{ y: -20, duration: 200 }}
>
  <!-- ===== 上半部分：问候语 + 最爱歌单 ===== -->
  <div class="flex flex-col gap-4 min-h-0 overflow-hidden">
    <GreetingQuote />

    <div class="flex gap-8 min-h-0 overflow-hidden">
      <div class="min-h-0 overflow-hidden" style="flex: 4;">
        <FavoritePlaylistCard />
      </div>
      <div class="min-h-0 overflow-hidden" style="flex: 7;">
        <div class="h-full flex items-center justify-center text-sm text-muted-foreground">
          暂无收藏歌单
        </div>
      </div>
    </div>
  </div>

  <!-- ===== 下半部分：下一首播放 + 推荐歌单 ===== -->
  <div class="grid grid-cols-2 gap-4 min-h-0 overflow-hidden">
    <!-- 下一首播放 -->
    <div class="flex flex-col gap-3 min-h-0 overflow-hidden">
      <div class="flex items-center gap-1 shrink-0">
        <SkipForward class="size-4 text-foreground" />
        <p class="text-base font-bold text-foreground">下一首播放</p>
        <Button variant="ghost" class="cursor-pointer" size="icon" href="/playqueue">
          <ChevronRight class="size-4 text-foreground" />
        </Button>
      </div>
      <NextupSongList {songs} loading={loadingSongs} onplay={playSong} />
    </div>

    <!-- 推荐歌单 -->
    <div class="flex flex-col gap-3 min-h-0 overflow-hidden">
      <div class="flex items-center gap-1 shrink-0">
        <ListMusic class="size-6 text-foreground" />
        <p class="text-base font-bold text-foreground">推荐歌单</p>
        <Button variant="ghost" class="cursor-pointer" size="icon" href="/explore">
          <ChevronRight class="size-4 text-foreground" />
        </Button>
      </div>
      <ScrollArea class="flex flex-col gap-2 flex-1 min-h-0">
        {#if loadingPlaylists}
          <div class="text-sm text-muted-foreground p-4">加载中...</div>
        {:else if recommendedPlaylists.length === 0}
          <div class="text-sm text-muted-foreground p-4">暂无推荐歌单</div>
        {:else}
          {#each recommendedPlaylists as p}
            <div class="mb-2">
              <PlaylistRowCard playlist={p} onplay={() => goPlaylist(p)} />
            </div>
          {/each}
        {/if}
      </ScrollArea>
    </div>
  </div>
</div>
