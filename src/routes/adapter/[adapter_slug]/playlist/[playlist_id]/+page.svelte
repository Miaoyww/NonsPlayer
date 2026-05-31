<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { Play, Heart, Ellipsis, Copy, Link, Hash } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
  } from "$lib/components/ui/dropdown-menu";
  import Skeleton from "$lib/components/ui/skeleton/skeleton.svelte";
  import { fly } from "svelte/transition";
  import SongList from "$lib/components/song-list.svelte";
  import { getPlaylist } from "$lib/services/adapter-service";
  import { playerService } from "$lib/services/player-service.svelte";
  import type { Song, Playlist } from "$lib/types";

  const adapterSlug = page.params.adapter_slug ?? "";
  const playlistId = page.params.playlist_id ?? "";

  let playlist = $state<Playlist | null>(null);
  let songs = $state<Song[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let playlistLiked = $state(false);

  onMount(async () => {
    if (!adapterSlug || !playlistId) {
      error = "无效的歌单链接";
      loading = false;
      return;
    }
    try {
      const data = await getPlaylist(adapterSlug, decodeURIComponent(playlistId));
      playlist = data;
      songs = data.musics ?? [];
      playlistLiked = false; // TODO: check from user data
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  function handlePlayAll() {
    if (songs.length === 0) return;
    playerService.play(songs, 0);
  }

  function handlePlay(index: number) {
    playerService.play(songs, index);
  }

  function handleLike(index: number) {
    songs = songs.map((sg, i) =>
      i === index ? { ...sg, isLiked: !sg.isLiked } : sg,
    );
  }

  function handleTogglePlaylistLike() {
    playlistLiked = !playlistLiked;
  }

  function handleCopyLink() {
    navigator.clipboard?.writeText(window.location.href);
  }

  function handleCopyId() {
    navigator.clipboard?.writeText(playlistId);
  }

  function handleCopyInfo() {
    if (!playlist) return;
    const text = `${playlist.name}\n${playlist.creator}\n${playlist.musicsCount} 首\n${playlist.description}`;
    navigator.clipboard?.writeText(text);
  }

  function coverGradient(name: string): string {
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    const h1 = Math.abs(hash % 360);
    const h2 = (h1 + 40) % 360;
    return `linear-gradient(135deg, hsl(${h1}, 50%, 45%), hsl(${h2}, 60%, 38%))`;
  }
</script>

<div
  class="h-[calc(100vh-36px)] overflow-y-auto"
  in:fly={{ y: 16, duration: 300, opacity: 0 }}
>
  {#if loading}
    <!-- ===== Skeleton ===== -->
    <div class="flex gap-8 pt-12 pl-8">
      <Skeleton class="w-64 h-64 shrink-0 rounded-xl" />
      <div class="flex flex-col justify-between flex-1 min-w-0 gap-4">
        <div class="flex flex-col gap-3">
          <Skeleton class="h-10 w-80 rounded" />
          <Skeleton class="h-4 w-48 rounded" />
          <Skeleton class="h-16 w-full max-w-150 rounded" />
        </div>
        <div class="flex gap-2.5">
          <Skeleton class="h-12.5 w-30 rounded-lg" />
          <Skeleton class="h-12.5 w-12.5 rounded-full" />
          <Skeleton class="h-12.5 w-12.5 rounded-full" />
        </div>
      </div>
    </div>
    <div class="mt-10 mx-8">
      {#each Array.from({ length: 12 }) as _}
        <div class="flex items-center gap-4 py-3">
          <Skeleton class="w-8 h-4 rounded" />
          <Skeleton class="flex-1 h-5 rounded" />
          <Skeleton class="w-24 h-4 rounded" />
        </div>
      {/each}
    </div>
  {:else if error}
    <div class="flex flex-col items-center justify-center h-full gap-3 text-muted-foreground">
      <p class="text-lg font-medium">加载歌单失败</p>
      <p class="text-sm">{error}</p>
    </div>
  {:else if playlist}
    <!-- ===== Header ===== -->
    <div class="flex gap-8 pt-12 pl-8">
      <!-- Cover -->
      <div
        class="w-64 h-64 shrink-0 rounded-xl border border-border bg-muted overflow-hidden shadow-lg"
      >
        {#if playlist.avatarUrl}
          <img
            src={playlist.avatarUrl}
            alt={playlist.name}
            class="w-full h-full object-cover"
          />
        {:else}
          <div
            class="w-full h-full"
            style="background: {coverGradient(playlist.name)}"
          ></div>
        {/if}
      </div>

      <!-- Info -->
      <div class="flex flex-col justify-between flex-1 min-w-0">
        <div class="flex flex-col gap-1">
          <h1
            class="text-4xl font-extrabold tracking-tight text-foreground truncate"
          >
            {playlist.name}
          </h1>

          <div
            class="flex items-center gap-2 text-sm text-muted-foreground/60 mt-1"
          >
            <span>{playlist.musicsCount} 首</span>
            {#if playlist.creator}
              <span>&middot;</span>
              <span>{playlist.creator}</span>
            {/if}
            {#if playlist.createTime}
              <span>&middot;</span>
              <span>{playlist.createTime}</span>
            {/if}
          </div>

          {#if playlist.description}
            <p
              class="text-sm text-muted-foreground/70 mt-3 line-clamp-3 max-w-150 leading-relaxed"
            >
              {playlist.description}
            </p>
          {/if}
        </div>

        <!-- Action Buttons -->
        <div class="flex items-center gap-2.5">
          <Button
            size="lg"
            class="h-12.5 w-30 rounded-lg font-bold text-sm gap-2 cursor-pointer"
            onclick={handlePlayAll}
          >
            <Play size={16} />
            播放全部
          </Button>

          <Button
            variant="outline"
            size="icon"
            class="h-12.5 w-12.5 rounded-full cursor-pointer {playlistLiked
              ? 'text-red-500 border-red-500'
              : ''}"
            onclick={handleTogglePlaylistLike}
            aria-label={playlistLiked ? "取消收藏" : "收藏"}
          >
            <Heart
              size={18}
              class={playlistLiked ? "fill-red-500 text-red-500" : ""}
            />
          </Button>

          <DropdownMenu>
            <DropdownMenuTrigger>
              <Button
                variant="outline"
                size="icon"
                class="h-12.5 w-12.5 rounded-full cursor-pointer"
                aria-label="更多"
              >
                <Ellipsis size={20} />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onclick={handleCopyLink}>
                <Link size={14} class="mr-2" />
                复制链接
              </DropdownMenuItem>
              <DropdownMenuItem onclick={handleCopyId}>
                <Hash size={14} class="mr-2" />
                复制 ID
              </DropdownMenuItem>
              <DropdownMenuItem onclick={handleCopyInfo}>
                <Copy size={14} class="mr-2" />
                复制信息
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    </div>

    <!-- ===== Song List ===== -->
    {#if songs.length > 0}
      <SongList class="mt-10 mx-8 mb-24" {songs} onplay={handlePlay} onlike={handleLike} />
    {:else}
      <div class="flex flex-col items-center justify-center mt-16 gap-2 text-muted-foreground">
        <p class="text-sm">歌单为空</p>
      </div>
    {/if}
  {/if}
</div>
