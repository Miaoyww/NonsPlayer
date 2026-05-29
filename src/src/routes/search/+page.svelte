<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { playerService } from "$lib/services/player-service.svelte";
  import { search as doSearch } from "$lib/services/adapter-service";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import SongCard from "$lib/components/cards/song/song-nextup-card.svelte";
  import PlaylistRowCard from "$lib/components/cards/playlist/playlist-row-card.svelte";
  import { Search, Music, SearchX, Disc3 } from "@lucide/svelte";
  import type { Song, Playlist } from "$lib/types";
  import type { SearchResult } from "$lib/types/adapter";
  import { fly } from "svelte/transition";

  let query = $state("");
  let searching = $state(false);
  let result = $state<SearchResult | null>(null);
  let error = $state("");
  let debounceTimer: ReturnType<typeof setTimeout>;

  onMount(async () => {
    await adapterStore.refresh();
  });

  function onInput(e: Event) {
    query = (e.target as HTMLInputElement).value;
    clearTimeout(debounceTimer);
    if (query.trim().length < 1) {
      result = null;
      return;
    }
    debounceTimer = setTimeout(() => performSearch(), 300);
  }

  async function performSearch() {
    const kw = query.trim();
    if (!kw) { result = null; return; }

    const adapters = adapterStore.adapters;
    if (adapters.length === 0) {
      error = "没有可用的音乐服务";
      return;
    }

    searching = true;
    error = "";
    try {
      // Search across all adapters, merge results
      const allResults: SearchResult = { songs: [], albums: [], artists: [], playlists: [] };
      for (const a of adapters) {
        try {
          const r = await doSearch(a.slug, kw);
          allResults.songs.push(...r.songs);
          allResults.albums.push(...r.albums);
          allResults.artists.push(...r.artists);
          allResults.playlists.push(...r.playlists);
        } catch {
          // skip adapters that don't support search
        }
      }
      result = allResults;
    } catch (e) {
      error = String(e);
    } finally {
      searching = false;
    }
  }

  function playSong(song: Song) {
    if (result) {
      playerService.play(result.songs, result.songs.indexOf(song));
    }
  }

  function goPlaylist(playlist: { id: string; adapterSlug: string }) {
    goto(`/adapter/${playlist.adapterSlug}/playlist/${encodeURIComponent(playlist.id)}`);
  }
</script>

<div
  class="flex flex-col gap-6 px-8 py-6 h-full overflow-hidden"
  transition:fly={{ y: -20, duration: 200 }}
>
  <!-- Search input -->
  <div class="shrink-0 max-w-xl">
    <div class="relative">
      <Search class="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
      <Input
        type="text"
        placeholder="搜索歌曲、专辑、艺术家..."
        value={query}
        oninput={onInput}
        class="pl-9"
      />
    </div>
  </div>

  <ScrollArea class="flex-1 min-h-0">
    {#if searching}
      <div class="text-sm text-muted-foreground text-center py-16">搜索中...</div>
    {:else if error}
      <div class="text-sm text-muted-foreground text-center py-16">
        <SearchX class="size-8 mx-auto mb-2 opacity-40" />
        {error}
      </div>
    {:else if result}
      {#if result.songs.length === 0 && result.playlists.length === 0 && result.albums.length === 0}
        <div class="text-sm text-muted-foreground text-center py-16">
          <Music class="size-8 mx-auto mb-2 opacity-40" />
          没有找到 "{query}" 的相关结果
        </div>
      {:else}
        <div class="space-y-8">
          <!-- Songs -->
          {#if result.songs.length > 0}
            <section>
              <h2 class="text-base font-bold text-foreground mb-3 flex items-center gap-2">
                <Music class="size-4" /> 歌曲 <span class="text-xs text-muted-foreground font-normal">{result.songs.length} 首</span>
              </h2>
              <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-2">
                {#each result.songs as song}
                  <SongCard {song} onplay={() => playSong(song)} />
                {/each}
              </div>
            </section>
          {/if}

          <!-- Playlists -->
          {#if result.playlists.length > 0}
            <section>
              <h2 class="text-base font-bold text-foreground mb-3 flex items-center gap-2">
                <Disc3 class="size-4" /> 歌单 <span class="text-xs text-muted-foreground font-normal">{result.playlists.length} 个</span>
              </h2>
              {#each result.playlists as p}
                <div class="mb-2">
                  <PlaylistRowCard playlist={p} onplay={() => goPlaylist(p)} />
                </div>
              {/each}
            </section>
          {/if}
        </div>
      {/if}
    {:else}
      <div class="text-sm text-muted-foreground text-center py-16">
        输入关键词开始搜索
      </div>
    {/if}
  </ScrollArea>
</div>
