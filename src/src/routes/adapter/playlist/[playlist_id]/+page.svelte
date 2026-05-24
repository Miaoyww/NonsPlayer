<script lang="ts">
  import { Play, Heart, Ellipsis, Copy, Link, Hash } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button';
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
  } from '$lib/components/ui/dropdown-menu';
  import { fly } from 'svelte/transition';

  interface SongItem {
    index: number;
    name: string;
    artist: string;
    album: string;
    duration: string;
    liked: boolean;
    coverUrl?: string;
  }

  const playlist = {
    name: '在路上-2026',
    coverUrl: '',
    musicsCount: '104 首',
    creator: 'Miaoyww',
    createTime: '2026-01-15',
    description:
      '感受每一段旅程的自由与洒脱，让音乐陪伴你在路上的每一个瞬间。收藏了这些年开车旅行时最喜欢的歌曲，从经典摇滚到独立民谣，从华语流行到欧美金曲。',
    liked: false,
  };

  let songs: SongItem[] = $state([
    { index: 1, name: '曾经的你', artist: '许巍', album: '每一刻都是崭新的', duration: '4:23', liked: true },
    { index: 2, name: '蓝莲花', artist: '许巍', album: '时光·漫步', duration: '4:32', liked: true },
    { index: 3, name: '平凡之路', artist: '朴树', album: '猎户星座', duration: '5:02', liked: false },
    { index: 4, name: '夜空中最亮的星', artist: '逃跑计划', album: '世界', duration: '4:14', liked: true },
    { index: 5, name: '南山南', artist: '马頔', album: '孤岛', duration: '4:37', liked: false },
    { index: 6, name: '理想三旬', artist: '陈鸿宇', album: '浓烟下的诗歌电台', duration: '3:47', liked: true },
    { index: 7, name: '春风十里', artist: '鹿先森乐队', album: '所有的酒，都不如你', duration: '6:24', liked: false },
    { index: 8, name: '成都', artist: '赵雷', album: '无法长大', duration: '5:28', liked: true },
    {
      index: 9,
      name: "Don't Look Back in Anger",
      artist: 'Oasis',
      album: "(What's the Story) Morning Glory?",
      duration: '4:48',
      liked: false,
    },
    {
      index: 10,
      name: 'Hotel California',
      artist: 'Eagles',
      album: 'Hotel California',
      duration: '6:30',
      liked: true,
    },
    {
      index: 11,
      name: 'Bohemian Rhapsody',
      artist: 'Queen',
      album: 'A Night at the Opera',
      duration: '5:55',
      liked: false,
    },
    {
      index: 12,
      name: 'Stairway to Heaven',
      artist: 'Led Zeppelin',
      album: 'Led Zeppelin IV',
      duration: '8:02',
      liked: true,
    },
  ]);

  function handleLike(index: number) {
    songs = songs.map((s, i) => (i === index ? { ...s, liked: !s.liked } : s));
  }

  function handlePlayAll() {
    // TODO: play all songs
  }

  function handleTogglePlaylistLike() {
    playlist.liked = !playlist.liked;
  }
</script>

<div in:fly={{ y: 8, duration: 320, opacity: 0 }} class="h-full overflow-auto">
  <!-- ===== Header ===== -->
  <div class="flex gap-8 pt-12 pl-8">
    <!-- Cover -->
    <div class="w-64 h-64 shrink-0 rounded-xl border border-border bg-muted overflow-hidden shadow-lg">
      {#if playlist.coverUrl}
        <img src={playlist.coverUrl} alt={playlist.name} class="w-full h-full object-cover" />
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
        <h1 class="text-4xl font-extrabold tracking-tight text-foreground truncate">
          {playlist.name}
        </h1>

        <!-- Sub-info row -->
        <div class="flex items-center gap-2 text-sm text-muted-foreground/60 mt-1">
          <span>{playlist.musicsCount}</span>
          <span>&middot;</span>
          <span>{playlist.creator}</span>
          <span>&middot;</span>
          <span>{playlist.createTime}</span>
        </div>

        <!-- Description -->
        <p class="text-base text-muted-foreground/70 mt-3 line-clamp-3 max-w-150 leading-relaxed">
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
          class="h-12.5 w-12.5 rounded-full cursor-pointer {playlist.liked ? 'text-red-500 border-red-500' : ''}"
          onclick={handleTogglePlaylistLike}
          aria-label={playlist.liked ? '取消收藏' : '收藏'}
        >
          <Heart size={18} class={playlist.liked ? 'fill-red-500 text-red-500' : ''} />
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
  <div class="mt-10 mx-8 mb-24">
    <!-- Table Header -->
    <div
      class="flex items-center gap-4 px-3 py-2 text-xs font-semibold text-muted-foreground/50 uppercase tracking-wider border-b border-border"
    >
      <span class="w-8 text-center">#</span>
      <span class="w-12 shrink-0"></span>
      <span class="flex-1">歌曲</span>
      <span class="w-72">专辑</span>
      <span class="w-20 text-right mr-16">时长</span>
    </div>

    <!-- Song Rows -->
    {#each songs as song, i}
      <div
        class="flex items-center gap-4 px-3 py-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer group"
        role="button"
        tabindex="0"
      >
        <!-- Index -->
        <span class="w-8 text-center text-sm text-muted-foreground group-hover:hidden">
          {song.index}
        </span>
        <span class="w-8 text-center hidden group-hover:flex items-center justify-center">
          <Play size={14} />
        </span>

        <!-- Cover -->
        <div class="w-10 h-10 shrink-0 rounded-md bg-muted overflow-hidden">
          {#if song.coverUrl}
            <img src={song.coverUrl} alt="" class="w-full h-full object-cover" />
          {:else}
            <div
              class="w-full h-full"
              style="background: linear-gradient(135deg, hsl({(song.index * 60) % 360}, 40%, 45%), hsl({(song.index * 60 + 40) % 360}, 45%, 35%))"
            ></div>
          {/if}
        </div>

        <!-- Song + Artist -->
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium truncate">{song.name}</p>
          <p class="text-xs text-muted-foreground truncate">{song.artist}</p>
        </div>

        <!-- Album -->
        <p class="w-72 text-sm text-muted-foreground truncate">{song.album}</p>

        <!-- Duration -->
        <span class="w-20 text-right text-sm text-muted-foreground tabular-nums">{song.duration}</span>

        <!-- Like -->
        <Button
          class="shrink-0 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer {song.liked
            ? 'opacity-100'
            : ''}"
          variant="ghost"
          size="icon"
          onclick={() => handleLike(i)}
          aria-label={song.liked ? '取消收藏' : '收藏'}
        >
          <Heart size={16} class={song.liked ? 'fill-red-500 text-red-500' : ''} />
        </Button>

        <!-- More -->
        <Button
          class="shrink-0 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
          variant="ghost"
          size="icon"
          aria-label="更多"
        >
          <Ellipsis size={16} />
        </Button>
      </div>
    {/each}
  </div>
</div>
