<script lang="ts">
	import { Compass, Sparkles, ChartColumn, LayoutGrid, Radio } from "@lucide/svelte";
	import Button from "$lib/components/ui/button/button.svelte";
	import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
	import RecommendedTab from "$lib/components/explore/recommended-tab.svelte";
	import ToplistTab from "$lib/components/explore/toplist-tab.svelte";
	import RadioTab from "$lib/components/explore/radio-tab.svelte";
	import PlaylistSquareTab from "$lib/components/explore/playlist-square-tab.svelte";
	import { fly } from "svelte/transition";

	type Section = "recommended" | "toplist" | "radio" | "square";

	let activeSection = $state<Section>("recommended");

	type NavItem = { key: Section; label: string; icon: typeof Compass };

	const navItems: NavItem[] = [
		{ key: "recommended", label: "推荐歌单", icon: Sparkles },
		{ key: "toplist", label: "排行榜", icon: ChartColumn },
		{ key: "radio", label: "雷达歌单", icon: Radio },
		{ key: "square", label: "歌单广场", icon: LayoutGrid },
	];
</script>

<div
	class="flex h-full overflow-hidden"
	in:fly={{ y: 16, duration: 300, opacity: 0 }}
>
	<!-- 左侧导航 -->
	<div class="flex w-50 shrink-0 flex-col bg-muted/50">
		<div class="px-5 pt-5 pb-2">
			<div class="flex items-center gap-2">
				<Compass class="size-5 text-foreground" />
				<h1 class="text-xl font-bold leading-none tracking-tight">发现音乐</h1>
			</div>
			<p class="mt-1.5 text-sm text-muted-foreground">探索新音乐</p>
		</div>
		<div class="flex flex-1 flex-col gap-0.5 px-3 pt-3">
			{#each navItems as item}
				<Button
					class="w-full cursor-pointer justify-start gap-2.5 px-3 h-9"
					variant={activeSection === item.key ? "secondary" : "ghost"}
					onclick={() => (activeSection = item.key)}
				>
					<item.icon size={18} />
					<span class="text-sm">{item.label}</span>
				</Button>
			{/each}
		</div>
	</div>

	<!-- 右侧内容 -->
	<div class="flex flex-1 flex-col bg-background min-w-0">
		<ScrollArea class="h-full w-full">
			<div class="p-8">
				{#if activeSection === "recommended"}<RecommendedTab />{/if}
				{#if activeSection === "toplist"}<ToplistTab />{/if}
				{#if activeSection === "radio"}<RadioTab />{/if}
				{#if activeSection === "square"}<PlaylistSquareTab />{/if}
			</div>
      <div class="h-24"></div>
		</ScrollArea>
	</div>
</div>
