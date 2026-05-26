<script lang="ts">
  import { Play, Heart, Ellipsis, Copy, Link, Hash } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
  } from "$lib/components/ui/dropdown-menu";
  import { fly } from "svelte/transition";
  import SongList from "$lib/components/song-list.svelte";
  import type { Song, Playlist } from "$lib/types";

  // 临时数据 - 歌单信息
  const empty = "";
  const a = (id: string, name: string) =>
    ({
      id,
      md5: empty,
      name,
      shareUrl: empty,
      avatarUrl: empty,
      smallAvatarUrl: empty,
      middleAvatarUrl: empty,
      createDate: empty,
      description: empty,
      songs: [],
      artists: [],
      artistsName: name,
      collectionCount: 0,
      trackCount: 0,
    }) as Song["album"];
  const r = (id: string, name: string) =>
    ({
      id,
      md5: empty,
      name,
      shareUrl: empty,
      avatarUrl: empty,
      smallAvatarUrl: empty,
      middleAvatarUrl: empty,
      description: empty,
      songs: [],
      musicCount: 0,
      trans: empty,
    }) as Song["artists"][number];

  function s(
    name: string,
    artistId: string,
    artistName: string,
    albumId: string,
    albumName: string,
    durationText: string,
    isLiked: boolean,
  ): Song {
    return {
      id: empty,
      md5: empty,
      name,
      shareUrl: empty,
      avatarUrl: empty,
      smallAvatarUrl: empty,
      middleAvatarUrl: empty,
      album: a(albumId, albumName),
      artists: [r(artistId, artistName)],
      isEmpty: false,
      duration: 0,
      url: empty,
      lyric: null,
      available: true,
      isLiked,
      trans: null,
      albumName,
      artistsName: artistName,
      durationText,
    };
  }

  const playlist: Playlist = {
    id: "playlist-on-the-road",
    md5: empty,
    name: "在路上-2026",
    shareUrl: empty,
    avatarUrl: empty,
    smallAvatarUrl: empty,
    middleAvatarUrl: empty,
    title: "在路上-2026",
    creator: "Miaoyww",
    createTime: "2026-01-15",
    description:
      "感受每一段旅程的自由与洒脱，让音乐陪伴你在路上的每一个瞬间。收藏了这些年开车旅行时最喜欢的歌曲，从经典摇滚到独立民谣，从华语流行到欧美金曲。",
    musicTrackIds: [],
    tags: [],
    musics: [],
    isInitialized: true,
    musicsCount: 104,
    playCount: 0,
  };

  let playlistLiked = $state(false);

  let songs: Song[] = $state([
    s(
      "曾经的你",
      "artist-xw",
      "许巍",
      "album-msk",
      "每一刻都是崭新的",
      "4:23",
      true,
    ),
    s("蓝莲花", "artist-xw", "许巍", "album-sgmb", "时光·漫步", "4:32", true),
    s("平凡之路", "artist-ps", "朴树", "album-lhxz", "猎户星座", "5:02", false),
    s(
      "夜空中最亮的星",
      "artist-tp",
      "逃跑计划",
      "album-sj",
      "世界",
      "4:14",
      true,
    ),
    s("南山南", "artist-md", "马頔", "album-gd", "孤岛", "4:37", false),
    s(
      "理想三旬",
      "artist-chy",
      "陈鸿宇",
      "album-nysg",
      "浓烟下的诗歌电台",
      "3:47",
      true,
    ),
    s(
      "春风十里",
      "artist-lxs",
      "鹿先森乐队",
      "album-sydj",
      "所有的酒，都不如你",
      "6:24",
      false,
    ),
    s("成都", "artist-zl", "赵雷", "album-wfzd", "无法长大", "5:28", true),
    s(
      "Don't Look Back in Anger",
      "artist-oasis",
      "Oasis",
      "album-wtsmg",
      "(What's the Story) Morning Glory?",
      "4:48",
      false,
    ),
    s(
      "Hotel California",
      "artist-eagles",
      "Eagles",
      "album-hc",
      "Hotel California",
      "6:30",
      true,
    ),
    s(
      "Bohemian Rhapsody",
      "artist-queen",
      "Queen",
      "album-anato",
      "A Night at the Opera",
      "5:55",
      false,
    ),
    s(
      "Stairway to Heaven",
      "artist-lz",
      "Led Zeppelin",
      "album-lz4",
      "Led Zeppelin IV",
      "8:02",
      true,
    ),
  ]);

  function handleLike(index: number) {
    songs = songs.map((sg, i) =>
      i === index ? { ...sg, isLiked: !sg.isLiked } : sg,
    );
  }

  function handlePlayAll() {
    // TODO: play all songs
  }

  function handleTogglePlaylistLike() {
    playlistLiked = !playlistLiked;
  }
</script>

<div
  class="h-[calc(100vh-36px)] overflow-y-auto"
  transition:fly={{ y: -20, duration: 200 }}
>
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
          style="background: linear-gradient(135deg, hsl(220, 50%, 45%), hsl(260, 60%, 38%))"
        ></div>
      {/if}
    </div>

    <!-- Info -->
    <div class="flex flex-col justify-between flex-1 min-w-0">
      <div class="flex flex-col gap-1">
        <!-- Title -->
        <h1
          class="text-4xl font-extrabold tracking-tight text-foreground truncate"
        >
          {playlist.name}
        </h1>

        <!-- Sub-info row -->
        <div
          class="flex items-center gap-2 text-sm text-muted-foreground/60 mt-1"
        >
          <span>{playlist.musicsCount} 首</span>
          <span>&middot;</span>
          <span>{playlist.creator}</span>
          <span>&middot;</span>
          <span>{playlist.createTime}</span>
        </div>

        <!-- Description -->
        <p
          class="text-base text-muted-foreground/70 mt-3 line-clamp-3 max-w-150 leading-relaxed"
        >
          {playlist.description}
        </p>
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
            <DropdownMenuItem>
              <Link size={14} class="mr-2" />
              复制链接
            </DropdownMenuItem>
            <DropdownMenuItem>
              <Hash size={14} class="mr-2" />
              复制 ID
            </DropdownMenuItem>
            <DropdownMenuItem>
              <Copy size={14} class="mr-2" />
              复制信息
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  </div>

  <!-- ===== Song List ===== -->
  <SongList class="mt-10 mx-8 mb-24" {songs} onlike={handleLike} />
</div>
