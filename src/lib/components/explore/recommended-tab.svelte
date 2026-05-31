<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount } from "svelte";
	import { Sparkles } from "@lucide/svelte";
	import { adapterStore } from "$lib/stores/adapter-store.svelte";
	import { getRecommendedPlaylists } from "$lib/services/adapter-service";
	import CoverListCard from "$lib/components/cards/playlist/cover-list-card.svelte";
	import Skeleton from "$lib/components/ui/skeleton/skeleton.svelte";
	import { fly } from "svelte/transition";
	import type { Playlist } from "$lib/types";

	let playlists = $state<Playlist[]>([]);
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
					const recs = await getRecommendedPlaylists(a.slug, 24);
					if (recs.length > 0) {
						playlists = recs;
						break;
					}
				} catch {
					// adapter doesn't support recommend
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

<div in:fly={{ y: 16, duration: 300, opacity: 0 }}>
	<div class="flex items-center gap-2 mb-4">
		<Sparkles class="size-5 text-foreground" />
		<h2 class="text-lg font-bold text-foreground">推荐歌单</h2>
	</div>
	{#if loading}
		<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
			{#each Array.from({ length: 24 }) as _}
				<div class="flex flex-col gap-2">
					<Skeleton class="aspect-square rounded-2xl w-full" />
					<Skeleton class="h-4 w-3/4 rounded" />
					<Skeleton class="h-3 w-1/2 rounded" />
				</div>
			{/each}
		</div>
	{:else if error}
		<div class="flex flex-col items-center justify-center py-16 gap-2 text-muted-foreground">
			<p class="text-sm">加载推荐失败</p>
			<p class="text-xs">{error}</p>
		</div>
	{:else if playlists.length === 0}
		<div class="flex flex-col items-center justify-center py-16 gap-2">
			<Sparkles class="size-10 text-muted-foreground/50" />
			<p class="text-sm text-muted-foreground">暂无推荐歌单</p>
			<p class="text-xs text-muted-foreground/70">连接音乐平台后获取个性化推荐</p>
		</div>
	{:else}
		<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
			{#each playlists as p}
				<CoverListCard playlist={p} onclick={() => goPlaylist(p)} />
			{/each}
		</div>
	{/if}
</div>
