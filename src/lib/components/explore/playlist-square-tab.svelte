<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { ListFilter, ChevronDown, Loader2, LayoutGrid } from "@lucide/svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import {
    getPlaylistCats,
    getPlaylistSquare,
  } from "$lib/services/adapter-service";
  import CoverListCard from "$lib/components/cards/playlist/cover-list-card.svelte";
  import CategoryDialog from "./category-dialog.svelte";
  import Skeleton from "$lib/components/ui/skeleton/skeleton.svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import { fly } from "svelte/transition";
  import type { Playlist, PlaylistCategory } from "$lib/types";

  let playlists = $state<Playlist[]>([]);
  let categories = $state<PlaylistCategory[]>([]);
  let selectedCat = $state("全部歌单");
  let highQuality = $state(false);
  let loading = $state(true);
  let loadingMore = $state(false);
  let hasMore = $state(true);
  let total = $state(0);
  let error = $state<string | null>(null);
  let catDialogOpen = $state(false);

  const LIMIT = 30;

  let adapterSlug = $state("");

  async function loadCats() {
    const online = adapterStore.streaming;
    if (online.length === 0) return;
    for (const a of online) {
      try {
        const cats = await getPlaylistCats(a.slug);
        if (cats.length > 0) {
          categories = cats;
          adapterSlug = a.slug;
          break;
        }
      } catch {
        // not supported
      }
    }
  }

  async function loadPlaylists(reset = false) {
    if (!adapterSlug) return;

    if (reset) {
      playlists = [];
      loading = true;
    } else {
      loadingMore = true;
    }

    try {
      const offset = reset ? 0 : playlists.length;
      const catQuery = selectedCat === "全部歌单" ? "全部" : selectedCat;
      const [items, t] = await getPlaylistSquare(
        adapterSlug,
        catQuery,
        "hot",
        LIMIT,
        offset,
        highQuality,
      );

      if (reset) {
        playlists = items;
      } else {
        playlists = [...playlists, ...items];
      }
      total = t;
      hasMore = t > 0 && playlists.length < t;
    } catch (e) {
      if (reset) {
        error = String(e);
      }
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  onMount(async () => {
    await adapterStore.refresh();
    const online = adapterStore.streaming;
    if (online.length === 0) {
      loading = false;
      return;
    }
    adapterSlug = online[0].slug;
    await loadCats();
    await loadPlaylists(true);
  });

  function handleCategorySelect(catName: string) {
    selectedCat = catName;
    catDialogOpen = false;
    loadPlaylists(true);
  }

  function handleQualityToggle(hq: boolean) {
    highQuality = hq;
    loadPlaylists(true);
  }

  function goPlaylist(p: Playlist) {
    goto(`/adapter/${p.adapterSlug}/playlist/${encodeURIComponent(p.id)}`);
  }
</script>

<CategoryDialog
  {categories}
  selectedTag={selectedCat}
  open={catDialogOpen}
  onopenchange={(v) => (catDialogOpen = v)}
  onselect={handleCategorySelect}
/>

<div
  class="flex flex-col gap-4 h-full"
  in:fly={{ y: 16, duration: 300, opacity: 0 }}
>
  <div class="flex items-center gap-2 mb-4">
    <LayoutGrid class="size-5 text-foreground" />
    <h2 class="text-lg font-bold text-foreground">歌单广场</h2>
  </div>
  <!-- Filter bar -->
  <div class="flex items-center gap-3 shrink-0">
    <!-- Category filter button -->
    <button
      class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full border border-border bg-card hover:bg-accent/40 cursor-pointer text-sm transition-colors"
      onclick={() => (catDialogOpen = true)}
    >
      <ListFilter class="size-3.5" />
      <span>{selectedCat}</span>
      <ChevronDown class="size-3.5 text-muted-foreground" />
    </button>

    <!-- Quality toggle -->
    <div
      class="inline-flex items-center rounded-full border border-border overflow-hidden"
    >
      <button
        class="px-3 py-1.5 text-sm cursor-pointer transition-colors
					{!highQuality
          ? 'bg-primary text-primary-foreground'
          : 'bg-card text-foreground hover:bg-accent/40'}"
        onclick={() => handleQualityToggle(false)}
      >
        推荐
      </button>
      <button
        class="px-3 py-1.5 text-sm cursor-pointer transition-colors
					{highQuality
          ? 'bg-primary text-primary-foreground'
          : 'bg-card text-foreground hover:bg-accent/40'}"
        onclick={() => handleQualityToggle(true)}
      >
        精品
      </button>
    </div>
  </div>

  <!-- Content -->
  {#if loading}
    <div
      class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
    >
      {#each Array.from({ length: 24 }) as _}
        <div class="flex flex-col gap-2">
          <Skeleton class="aspect-square rounded-2xl w-full" />
          <Skeleton class="h-4 w-3/4 rounded" />
          <Skeleton class="h-3 w-1/2 rounded" />
        </div>
      {/each}
    </div>
  {:else if error}
    <div
      class="flex flex-col items-center justify-center py-16 gap-2 text-muted-foreground"
    >
      <p class="text-sm">加载歌单失败</p>
      <p class="text-xs">{error}</p>
    </div>
  {:else if playlists.length === 0}
    <div class="flex flex-col items-center justify-center py-16 gap-2">
      <ListFilter class="size-10 text-muted-foreground/50" />
      <p class="text-sm text-muted-foreground">暂无歌单</p>
    </div>
  {:else}
    <div
      class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
    >
      {#each playlists as p}
        <CoverListCard playlist={p} onclick={() => goPlaylist(p)} />
      {/each}
    </div>

    <!-- Load more -->
    <div class="flex justify-center py-4">
      {#if loadingMore}
        <div
          class="inline-flex items-center gap-2 text-sm text-muted-foreground"
        >
          <Loader2 class="size-4 animate-spin" />
          加载中...
        </div>
      {:else if hasMore}
        <Button variant="outline" onclick={() => loadPlaylists(false)}>
          加载更多
        </Button>
      {:else}
        <p class="text-xs text-muted-foreground">- 没有更多了 -</p>
      {/if}
    </div>
  {/if}
</div>
