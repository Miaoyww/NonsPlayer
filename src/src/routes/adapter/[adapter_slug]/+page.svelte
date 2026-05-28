<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { getUserPlaylists, getAccount } from "$lib/services/adapter-service";
  import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
  import Button from "$lib/components/ui/button/button.svelte";
  import PlaylistRowCard from "$lib/components/cards/playlist-row-card.svelte";
  import { ArrowLeft, Disc3, User, Library, Heart, Music, Search, Album, Users, List, Zap } from "@lucide/svelte";
  import { fly } from "svelte/transition";
  import { goto } from "$app/navigation";
  import type { AdapterMetadata } from "$lib/types/adapter";
  import type { Playlist } from "$lib/types";
  import type { Account } from "$lib/types/account";

  const slug = $derived(page.params.adapter_slug ?? "");
  let adapter = $state<AdapterMetadata | undefined>();
  let account = $state<Account | null>(null);
  let playlists = $state<Playlist[]>([]);
  let loadingAccount = $state(false);
  let loadingPlaylists = $state(false);

  const capabilityLabels: Record<string, { label: string; icon: typeof Music }> = {
    Music: { label: "音乐播放", icon: Music },
    Search: { label: "搜索", icon: Search },
    Album: { label: "专辑", icon: Album },
    Artist: { label: "艺术家", icon: Users },
    Playlist: { label: "歌单", icon: List },
    Account: { label: "账号", icon: User },
    Recommend: { label: "推荐", icon: Zap },
  };

  onMount(async () => {
    await adapterStore.refresh();
    adapter = adapterStore.get(slug);
    if (!adapter) return;

    // Try to load account and playlists
    loadingAccount = true;
    try { account = await getAccount(slug); } catch { account = null; }
    loadingAccount = false;

    loadingPlaylists = true;
    try { playlists = await getUserPlaylists(slug); } catch { playlists = []; }
    loadingPlaylists = false;
  });

  function goPlaylist(playlist: { id: string; adapterSlug: string }) {
    goto(`/adapter/${playlist.adapterSlug}/playlist/${encodeURIComponent(playlist.id)}`);
  }
</script>

<div
  class="flex flex-col gap-6 px-8 py-6 h-full overflow-hidden"
  transition:fly={{ y: -20, duration: 200 }}
>
  <!-- Header -->
  <div class="flex items-center gap-3 shrink-0">
    <Button variant="ghost" size="icon" class="cursor-pointer" onclick={() => goto("/library")}>
      <ArrowLeft class="size-5" />
    </Button>
    <div class="size-10 rounded-xl bg-linear-to-br from-primary/20 to-primary/5 flex items-center justify-center">
      <Disc3 class="size-5 text-primary/60" />
    </div>
    <div>
      <h1 class="text-xl font-bold text-foreground">{adapter?.displayPlatform ?? slug}</h1>
      <p class="text-sm text-muted-foreground">{adapter?.description ?? "加载中..."}</p>
    </div>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 flex-1 min-h-0 overflow-hidden">
    <!-- Left: Info + Capabilities -->
    <div class="flex flex-col gap-4 lg:col-span-1 min-h-0 overflow-auto">
      <!-- Adapter meta -->
      {#if adapter}
        <div class="rounded-xl border border-border/50 bg-card p-4 space-y-2">
          <h3 class="text-sm font-semibold text-foreground">适配器信息</h3>
          <div class="text-xs text-muted-foreground space-y-1">
            <p>平台: {adapter.platform}</p>
            <p>版本: {adapter.version}</p>
            <p>作者: {adapter.author}</p>
          </div>
        </div>
      {/if}

      <!-- Account -->
      <div class="rounded-xl border border-border/50 bg-card p-4 space-y-3">
        <h3 class="text-sm font-semibold text-foreground flex items-center gap-2">
          <User class="size-4" />
          账号
        </h3>
        {#if loadingAccount}
          <p class="text-xs text-muted-foreground">加载中...</p>
        {:else if account?.isLoggedIn}
          <div class="flex items-center gap-3">
            <div class="size-10 rounded-full bg-muted overflow-hidden">
              {#if account.avatarUrl}
                <img src={account.avatarUrl} alt="" class="size-full object-cover" />
              {/if}
            </div>
            <div>
              <p class="text-sm font-medium">{account.name}</p>
              <p class="text-xs text-muted-foreground">已登录</p>
            </div>
          </div>
        {:else}
          <p class="text-xs text-muted-foreground">未登录</p>
          <Button variant="outline" size="sm" class="cursor-pointer text-xs" disabled>
            登录（即将推出）
          </Button>
        {/if}
      </div>

      <!-- Capabilities -->
      <div class="rounded-xl border border-border/50 bg-card p-4 space-y-2">
        <h3 class="text-sm font-semibold text-foreground">支持的功能</h3>
        <div class="flex flex-wrap gap-2">
          {#each (["Music", "Search", "Album", "Artist", "Playlist", "Account", "Recommend"] as const) as cap}
            {@const info = capabilityLabels[cap]}
            {#if info}
              <span
                class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium bg-muted text-muted-foreground"
                class:opacity-30={!adapter}
              >
                <info.icon class="size-3" />
                {info.label}
              </span>
            {/if}
          {/each}
        </div>
      </div>
    </div>

    <!-- Right: Playlists -->
    <div class="flex flex-col min-h-0 overflow-hidden lg:col-span-2">
      <div class="flex items-center gap-2 shrink-0 mb-3">
        <Library class="size-5 text-foreground" />
        <h2 class="text-lg font-bold text-foreground">歌单</h2>
        {#if playlists.length > 0}
          <span class="text-xs text-muted-foreground">{playlists.length} 个</span>
        {/if}
      </div>

      <ScrollArea class="flex-1 min-h-0">
        {#if loadingPlaylists}
          <div class="text-sm text-muted-foreground text-center py-16">加载歌单中...</div>
        {:else if playlists.length === 0}
          <div class="text-sm text-muted-foreground text-center py-16">
            {#if account?.isLoggedIn}
              暂无歌单
            {:else}
              请先登录以查看歌单
            {/if}
          </div>
        {:else}
          {#each playlists as p}
            <div class="mb-2">
              <PlaylistRowCard playlist={p} onplay={() => goPlaylist(p)} />
            </div>
          {/each}
        {/if}
      </ScrollArea>
    </div>
  </div>
</div>
