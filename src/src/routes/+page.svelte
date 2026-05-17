<script lang="ts">
	import { Star, SkipForward, ListMusic, ChevronRight } from '@lucide/svelte';
</script>

<div class="h-full w-full overflow-auto">
	<div class="w-full h-[1024px] overflow-hidden relative shrink-0 bg-background">
		<!-- 头部区域 - 晚上好 & 每日推荐 -->
		<div class="w-full max-w-[1341px] absolute left-[51px] top-[88px] flex items-center gap-6">
			<!-- 问候语 -->
			<div class="flex-1 h-[37px] flex flex-col justify-center items-start">
				<div class="flex flex-col gap-[9px] items-start justify-center relative">
					<p class="text-xl font-bold text-foreground m-0">晚上好</p>
					<p class="text-[11px] font-medium text-[#747474] m-0 whitespace-nowrap">受尽苦难而不厌，此乃阿修罗之道。</p>
				</div>
			</div>

			<!-- 每日推荐 -->
			<div class="flex-1 h-[37px] flex flex-col justify-center items-start">
				<div class="flex flex-col gap-2 items-start justify-center">
					<div class="flex items-center gap-1 relative">
						<Star class="w-[26px] h-[22px]" />
						<p class="text-xl font-bold text-foreground m-0">每日推荐</p>
					</div>
				</div>
			</div>
		</div>

		<!-- 下一首播放 -->
		<div class="absolute left-[53px] top-[668px]">
			<div class="flex items-center gap-1">
				<div class="w-6 h-[26px] flex items-center justify-center">
					<SkipForward class="w-6 h-[26px] text-foreground" />
				</div>
				<p class="text-xl font-bold text-foreground m-0 whitespace-pre">下一首播放</p>
			</div>
		</div>

		<!-- 最爱歌单 -->
		<div class="absolute left-[51px] top-[403px]">
			<div class="flex items-center gap-1">
				<div class="size-6 flex items-center justify-center">
					<ListMusic class="size-6 text-foreground" />
				</div>
				<div class="flex items-center gap-0.5">
					<p class="text-xl font-bold text-foreground m-0 whitespace-pre">最爱歌单</p>
					<ChevronRight class="w-2 h-3.5 text-foreground" />
				</div>
			</div>
		</div>

		<!-- 推荐歌单 -->
		<div class="absolute left-[733px] top-[666px]">
			<div class="flex items-center gap-1">
				<div class="size-6 flex items-center justify-center">
					<ListMusic class="size-6 text-foreground" />
				</div>
				<div class="flex items-center gap-0.5">
					<p class="text-xl font-bold text-foreground m-0 whitespace-pre">推荐歌单</p>
					<ChevronRight class="w-2 h-3.5 text-foreground" />
				</div>
			</div>
		</div>
	</div>
</div>

