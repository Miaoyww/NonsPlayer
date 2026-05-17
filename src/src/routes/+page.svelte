<script lang="ts">
	import { Star, SkipForward, ListMusic, ChevronRight } from '@lucide/svelte';
</script>

<div class="flex flex-col gap-8 p-8 h-full overflow-auto">
	<!-- 头部区域 -->
	<div class="flex justify-between items-start">
		<!-- 问候语 -->
		<div class="flex flex-col gap-2">
			<p class="text-xl font-bold text-foreground">晚上好</p>
			<p class="text-[11px] font-medium text-[#747474] whitespace-nowrap">受尽苦难而不厌，此乃阿修罗之道。</p>
		</div>

		<!-- 每日推荐 -->
		<div class="flex items-center gap-1">
			<Star class="w-[26px] h-[22px]" />
			<p class="text-xl font-bold text-foreground">每日推荐</p>
		</div>
	</div>

	<!-- 最爱歌单 -->
	<div class="flex items-center gap-1">
		<ListMusic class="size-6 text-foreground" />
		<p class="text-xl font-bold text-foreground">最爱歌单</p>
		<ChevronRight class="w-2 h-3.5 text-foreground" />
	</div>

	<!-- 底部双栏 -->
	<div class="flex justify-between items-center mt-auto">
		<!-- 下一首播放 -->
		<div class="flex items-center gap-1">
			<SkipForward class="w-6 h-[26px] text-foreground" />
			<p class="text-xl font-bold text-foreground">下一首播放</p>
		</div>

		<!-- 推荐歌单 -->
		<div class="flex items-center gap-1">
			<ListMusic class="size-6 text-foreground" />
			<p class="text-xl font-bold text-foreground">推荐歌单</p>
			<ChevronRight class="w-2 h-3.5 text-foreground" />
		</div>
	</div>
</div>

