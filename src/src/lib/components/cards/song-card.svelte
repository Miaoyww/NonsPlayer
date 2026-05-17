<script lang="ts">
	import { Play, Heart } from '@lucide/svelte';
  import Button from '../ui/button/button.svelte';

	interface Props {
		songName: string;
		artist: string;
		coverUrl?: string;
		duration?: string;
		liked?: boolean;
		onplay?: () => void;
		onlike?: () => void;
	}

	let {
		songName,
		artist,
		coverUrl = '',
		duration = '',
		liked = $bindable(false),
		onplay,
		onlike
	}: Props = $props();

	function handleLike(e: MouseEvent) {
		e.stopPropagation();
		liked = !liked;
		onlike?.();
	}
</script>

<button
	class="group flex items-center gap-3 w-full rounded-xl p-3 hover:bg-muted/50 transition-colors cursor-pointer text-left"
	onclick={() => onplay?.()}
>
	<!-- 封面 -->
	<div
		class="size-12 shrink-0 rounded-lg bg-cover bg-center bg-muted"
		style={coverUrl ? `background-image: url(${coverUrl})` : ''}
	></div>

	<!-- 歌曲信息 -->
	<div class="flex-1 min-w-0">
		<p class="text-sm font-semibold text-foreground truncate">{songName}</p>
		<p class="text-xs text-muted-foreground truncate">{artist}</p>
	</div>

	<!-- 时长 -->
	{#if duration}
		<span class="text-xs text-muted-foreground shrink-0">{duration}</span>
	{/if}

	<!-- 收藏按钮 -->
	<Button
		class="shrink-0 p-1 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
		aria-label={liked ? '取消收藏' : '收藏'}
		onclick={handleLike}
	>
		<Heart class="size-4" fill={liked ? 'currentColor' : 'none'} />
	</Button>
</button>
