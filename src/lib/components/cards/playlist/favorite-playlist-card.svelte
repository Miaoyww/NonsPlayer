<script lang="ts">
  import { Play } from "@lucide/svelte";
  import { goto } from "$app/navigation";
  import Button from "../../ui/button/button.svelte";
  import { adapterStore } from "$lib/stores/adapter-store.svelte";
  import { getFavoritePlaylist, getUserPlaylists } from "$lib/services/adapter-service";
  import { neteaseAuth, fetchNeteaseLyric } from "$lib/services/netease-api";
  import { parseYrc } from "@applemusic-like-lyrics/lyric";
  import { parseSmartLrc } from "$lib/utils/lyric-parser";
  import PlaylistCard from "./playlist-card.svelte";
  import type { Playlist } from "$lib/types";

  let favorite: Playlist | null = $state(null);
  let playlists: Playlist[] = $state([]);
  let loading = $state(true);
  let lyricLines = $state<string[]>([]);

  // ── Lyric helpers ──────────────────────────────────────────────────

  const META_PATTERNS = /^(作曲|作词|编曲|制作人|混音|母带|录音|和声|吉他|贝斯|键盘|鼓|钢琴|弦乐|小提琴|大提琴|监制|出品|发行|厂牌|DJ|Remix|feat\.?|ft\.?|Cover)/i;

  /** Extract plain text lines from YRC or LRC raw lyric string. */
  function lyricsToLines(yrcText: string, lrcText: string): string[] {
    // Prefer YRC (word-level)
    if (yrcText) {
      try {
        const parsed = parseYrc(yrcText);
        return parsed
          .map((l: any) => l.words?.map((w: any) => w.word ?? "").join("") ?? "")
          .filter((t: string) => t.trim() && !META_PATTERNS.test(t));
      } catch { /* fall through to LRC */ }
    }
    // Fallback: LRC
    if (lrcText) {
      try {
        const { lines } = parseSmartLrc(lrcText);
        return lines
          .map((l: any) => l.words?.map((w: any) => w.word ?? "").join("") ?? "")
          .filter((t: string) => t.trim() && !META_PATTERNS.test(t));
      } catch { /* give up */ }
    }
    return [];
  }

  function pick<T>(arr: T[]): T | undefined {
    if (arr.length === 0) return undefined;
    return arr[Math.floor(Math.random() * arr.length)];
  }

  async function fetchLyricLines(songs: { id: string }[], n = 5): Promise<string[]> {
    const ids = songs.slice(0, n).map(s => s.id.replace(/^netease_song_/, ""));
    const results = await Promise.allSettled(ids.map(id => fetchNeteaseLyric(id)));
    const allSongs: string[][] = [];
    for (const r of results) {
      if (r.status === "fulfilled" && r.value) {
        const lines = lyricsToLines(r.value.yrc, r.value.lrc);
        if (lines.length > 0) allSongs.push(lines);
      }
    }
    const chosen = pick(allSongs);
    return chosen ? chosen.slice(0, 3) : [];
  }

  $effect(() => {
    const online = adapterStore.streaming;
    console.log("[fav-card] effect run | adapters:", online.map(a => a.slug));
    if (online.length === 0) return;

    // Re-trigger on auth change
    const authed = $neteaseAuth.loggedIn;
    console.log("[fav-card] auth:", authed);

    if (!authed) { favorite = null; loading = false; console.log("[fav-card] not authed → show needLogin"); return; }
    loading = true;

    (async () => {
      for (const a of online) {
        console.log("[fav-card] trying adapter:", a.slug);
        try {
          const fav = await getFavoritePlaylist(a.slug);
          if (fav) {
            favorite = fav;
            if (fav.musics?.length > 0) {
              lyricLines = await fetchLyricLines(fav.musics);
            }
          }
          // Fetch user playlists (excluding favorite) for the right grid
          const all = await getUserPlaylists(a.slug);
          playlists = all.filter(p => p.id !== fav?.id).slice(0, 12);
          break;
        } catch (e) { console.log("[fav-card] error for", a.slug, ":", e); }
      }
      console.log("[fav-card] done | favorite:", favorite?.name ?? "null", "| playlists:", playlists.length);
      loading = false;
    })();
  });

  const hasNoAdapters = $derived(!loading && !favorite && adapterStore.streaming.length === 0);
  const needLogin = $derived(!loading && !favorite && !hasNoAdapters);
  const fav = $derived(favorite!);
</script>

<div class="flex gap-8 min-h-0 overflow-hidden">
  <div class="min-h-0 overflow-hidden" style="flex: 4;">
    <div class="w-full h-full rounded-lg shadow bg-muted flex flex-col">
      {#if loading}
        <div class="flex items-center justify-center h-full text-sm text-muted-foreground">
          加载中...
        </div>
      {:else if favorite}
        <!-- 随机歌词 -->
        <div class="flex-1 flex flex-col justify-center p-4">
          {#if lyricLines.length > 0}
            {#each lyricLines as line}
              <p class="text-sm text-muted-foreground italic leading-relaxed">{line}</p>
            {/each}
          {:else}
            <p class="text-xs text-muted-foreground">{fav.musicsCount ?? fav.musicTrackIds?.length ?? 0} 首</p>
          {/if}
        </div>
        <!-- 底部 -->
        <div class="flex items-end justify-between p-4 mt-auto">
          <div class="flex flex-col gap-1">
            <p class="text-2xl font-bold text-blue-500">{fav.name}</p>
            <p class="text-sm text-blue-500">{fav.musicsCount ?? fav.musicTrackIds?.length ?? 0} Tracks</p>
          </div>
          <Button
            class="relative shrink-0 rounded-full flex items-center justify-center hover:bg-blue-600 cursor-pointer bg-blue-500"
            size="icon"
            onclick={() => goto(`/adapter/${fav.adapterSlug}/playlist/${encodeURIComponent(fav.id)}`)}
          >
            <Play class="size-4 text-foreground ml-0.5 fill-white stroke-white" />
          </Button>
        </div>
      {:else if hasNoAdapters}
        <div class="flex flex-col items-center justify-center h-full text-sm text-muted-foreground gap-2 p-4">
          <p>尚未连接音乐服务</p>
          <Button variant="outline" size="sm" class="cursor-pointer text-xs" onclick={() => goto("/settings")}>
            去设置中连接
          </Button>
        </div>
      {:else if needLogin}
        <div class="flex flex-col items-center justify-center h-full text-sm text-muted-foreground gap-2 p-4">
          <p>登录后即可同步喜欢的歌单</p>
        </div>
      {:else}
        <div class="flex items-center justify-center h-full text-sm text-muted-foreground">
          暂无收藏歌单
        </div>
      {/if}
    </div>
  </div>
  <div class="min-h-0 overflow-auto" style="flex: 7;">
    {#if loading}
      <div class="h-full flex items-center justify-center text-sm text-muted-foreground">
        加载中...
      </div>
    {:else if playlists.length > 0}
      <div class="grid grid-cols-4 gap-3 p-1">
        {#each playlists as pl}
          <PlaylistCard
            playlist={pl}
            onclick={() => goto(`/adapter/${pl.adapterSlug}/playlist/${encodeURIComponent(pl.id)}`)}
          />
        {/each}
      </div>
    {:else}
      <div class="h-full flex items-center justify-center text-sm text-muted-foreground">
        暂无歌单
      </div>
    {/if}
  </div>
</div>
