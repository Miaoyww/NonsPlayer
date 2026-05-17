<script lang="ts">
	import { Play, Heart, Star } from '@lucide/svelte';

	interface Props {
		songName?: string;
		artist?: string;
		coverUrl?: string;
		liked?: boolean;
		onplay?: () => void;
		onlike?: () => void;
	}

	let {
		songName = '僕の戦争',
		artist = '神聖かまってちゃん',
		coverUrl = '',
		liked = $bindable(false),
		onplay,
		onlike
	}: Props = $props();

	function handleLike() {
		liked = !liked;
		onlike?.();
	}
</script>

<div class="flex flex-col gap-2">
	<!-- 卡片主体 -->
	<div class="w-full h-[250px] rounded-[15px] shadow-[0_4px_4px_rgba(0,0,0,0.1)] bg-[#fafafa] overflow-hidden">
		<div class="flex items-center gap-10 p-6 h-full">
			<!-- 专辑封面 -->
			<div
				class="w-[205px] h-[205px] shrink-0 rounded-[15px] bg-cover bg-center bg-muted"
				style={coverUrl ? `background-image: url(${coverUrl})` : ''}
			></div>

			<!-- 歌曲信息区 -->
			<div class="flex-1 flex flex-col justify-center gap-[41px] h-full py-6">
				<!-- 歌名 & 歌手 -->
				<div class="flex flex-col gap-[15px]">
					<p class="text-2xl font-black text-[#18191F] whitespace-pre">{songName}</p>
					<p class="text-2xl font-medium text-foreground whitespace-pre">{artist}</p>
				</div>

				<!-- 操作按钮 -->
				<div class="flex items-center gap-[20.5px]">
					<!-- 播放按钮 -->
					<button
						class="size-[47px] shrink-0 rounded-full bg-muted flex items-center justify-center hover:bg-muted/80 transition-colors cursor-pointer"
						aria-label="播放 {songName}"
						onclick={() => onplay?.()}
					>
						<Play class="size-6 text-foreground ml-0.5" fill="currentColor" />
					</button>

					<!-- 收藏按钮 -->
					<button
						class="size-9 shrink-0 flex items-center justify-center cursor-pointer transition-colors"
						class:text-red-500={liked}
						class:text-foreground={!liked}
						aria-label={liked ? '取消收藏' : '收藏'}
						onclick={handleLike}
					>
						<Heart class="size-9" fill={liked ? 'currentColor' : 'none'} />
					</button>
				</div>
			</div>
		</div>
	</div>
</div>
