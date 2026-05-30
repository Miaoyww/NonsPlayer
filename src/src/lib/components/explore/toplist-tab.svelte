<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { ChartColumn } from "@lucide/svelte";
	import { adapterStore } from "$lib/stores/adapter-store.svelte";
	import { getTopPlaylists } from "$lib/services/adapter-service";
	import ToplistCard from "$lib/components/cards/playlist/toplist-card.svelte";
	import Skeleton from "$lib/components/ui/skeleton/skeleton.svelte";
	import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
	import { fly } from "svelte/transition";
	import type { TopPlaylistGroup, Playlist } from "$lib/types";

	let officialPlaylists = $state<Playlist[]>([]);
	let featuredPlaylists = $state<Playlist[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			await adapterStore.refresh();
			const online = adapterStore.streaming;
			if (online.length === 0) {
				loading = false;
				return;
			}

			for (const a of online) {
				try {
					const groups: TopPlaylistGroup[] = await getTopPlaylists(a.slug);
					for (const g of groups) {
						if (g.name === "Official") {
							officialPlaylists = g.playlists;
						} else {
							featuredPlaylists = g.playlists;
						}
					}
					if (groups.length > 0) break;
				} catch {
					// adapter doesn't support toplist
				}
			}
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	});

	function goPlaylist(p: Playlist) {
		goto(`/adapter/${p.adapterSlug}/playlist/${encodeURIComponent(p.id)}`);
	}
</script>

<div in:fly={{ y: 16, duration: 300, opacity: 0 }} class="h-full">
	<ScrollArea class="h-full">
		{#if loading}
		<!-- Official chart skeletons -->
		<div class="mb-8">
			<div class="h-5 w-20 bg-muted rounded mb-4 animate-pulse"></div>
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
				{#each Array.from({ length: 4 }) as _}
					<div class="rounded-xl border border-border/50 p-3 flex flex-col gap-3">
						<Skeleton class="h-5 w-3/4 rounded" />
						<Skeleton class="aspect-square rounded-xl w-full" />
						{#each Array.from({ length: 5 }) as _}
							<Skeleton class="h-4 w-full rounded" />
						{/each}
					</div>
				{/each}
			</div>
		</div>

		<!-- Featured chart skeletons -->
		<div>
			<div class="h-5 w-20 bg-muted rounded mb-4 animate-pulse"></div>
			<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
				{#each Array.from({ length: 10 }) as _}
					<div class="flex items-center gap-3 rounded-xl border border-border/50 p-3">
						<Skeleton class="size-20 rounded-xl shrink-0" />
						<div class="flex flex-col gap-2 flex-1">
							<Skeleton class="h-4 w-3/4 rounded" />
							<Skeleton class="h-3 w-1/2 rounded" />
						</div>
					</div>
				{/each}
			</div>
		</div>
	{:else if error}
		<div class="flex flex-col items-center justify-center py-16 gap-2 text-muted-foreground">
			<p class="text-sm">加载排行榜失败</p>
			<p class="text-xs">{error}</p>
		</div>
	{:else if officialPlaylists.length === 0 && featuredPlaylists.length === 0}
		<div class="flex flex-col items-center justify-center py-16 gap-2">
			<ChartColumn class="size-10 text-muted-foreground/50" />
			<p class="text-sm text-muted-foreground">暂无排行榜数据</p>
			<p class="text-xs text-muted-foreground/70">连接音乐平台后获取排行榜</p>
		</div>
	{:else}
		<!-- Official charts -->
		{#if officialPlaylists.length > 0}
			<div class="mb-8">
				<div class="flex items-center gap-2 mb-4">
					<ChartColumn class="size-4 text-foreground" />
					<h2 class="text-base font-bold text-foreground">官方榜</h2>
				</div>
				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
					{#each officialPlaylists as p}
						<ToplistCard playlist={p} size="normal" onclick={() => goPlaylist(p)} />
					{/each}
				</div>
			</div>
		{/if}

		<!-- Featured charts -->
		{#if featuredPlaylists.length > 0}
			<div>
				<div class="flex items-center gap-2 mb-4">
					<ChartColumn class="size-4 text-foreground" />
					<h2 class="text-base font-bold text-foreground">精选榜</h2>
				</div>
				<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
					{#each featuredPlaylists as p}
						<ToplistCard playlist={p} size="small" onclick={() => goPlaylist(p)} />
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</ScrollArea>
</div>
