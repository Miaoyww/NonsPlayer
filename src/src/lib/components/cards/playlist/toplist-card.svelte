<script lang="ts">
	import { Play } from "@lucide/svelte";
	import type { Playlist } from "$lib/types";
	import { coverSrc } from "$lib/utils";

	interface Props {
		playlist: Playlist;
		size?: "normal" | "small";
		description?: string;
		onclick?: () => void;
	}

	let { playlist, size = "normal", description = "", onclick }: Props = $props();

	function getGradient(name: string): string {
		let hash = 0;
		for (let i = 0; i < name.length; i++) {
			hash = name.charCodeAt(i) + ((hash << 5) - hash);
		}
		const h1 = Math.abs(hash % 360);
		const h2 = (h1 + 40) % 360;
		return `linear-gradient(135deg, hsl(${h1}, 50%, 45%), hsl(${h2}, 60%, 38%))`;
	}

	const gradient = getGradient(playlist.name);

	// Get up to 3 cover images from the tracks for cascading effect
	const trackCovers = $derived(
		playlist.musics?.slice(0, 3).map((s) => s.middleAvatarUrl || s.avatarUrl).filter(Boolean) ?? [],
	);

	// Extract update tip from description (stored as "desc|updateTip")
	const updateTip = $derived(
		description || playlist.description?.split("|").pop() || "",
	);

	// First 5 tracks for the track listing
	const topTracks = $derived(playlist.musics?.slice(0, 5) ?? []);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
{#if size === "normal"}
	<div
		class="group cursor-pointer rounded-xl bg-card border border-border/50 hover:border-border hover:shadow-md transition-all duration-200 overflow-hidden"
		onclick={() => onclick?.()}
		onkeydown={(e) => e.key === "Enter" && onclick?.()}
		role="button"
		tabindex="0"
	>
		<!-- Header -->
		<div class="p-3 pb-0">
			<p class="text-base font-bold text-foreground truncate">{playlist.name}</p>
			{#if updateTip}
				<p class="text-xs text-muted-foreground line-clamp-2 mt-0.5">{updateTip}</p>
			{/if}
		</div>

		<!-- Cover area -->
		<div class="relative aspect-square m-3 mt-2 rounded-xl overflow-hidden">
			{#if trackCovers.length >= 3}
				<!-- Cascading 3-covers -->
				<div class="relative size-full">
					{#each trackCovers as cover, i}
						{@const rotate = i === 0 ? 0 : i === 1 ? 6 : 12}
						{@const scale = i === 0 ? 1 : i === 1 ? 0.9 : 0.8}
						{@const z = 3 - i}
						<img
							src={coverSrc(cover)}
							alt=""
							class="absolute rounded-lg object-cover transition-all duration-300 group-hover:brightness-[0.4]"
							style="width: 100%; height: 100%; z-index: {z}; transform: scale({scale}) rotate({rotate}deg); opacity: {i === 0 ? 1 : i === 1 ? 0.9 : 0.8};"
						/>
					{/each}
				</div>
			{:else if playlist.avatarUrl}
				<img
					src={coverSrc(playlist.avatarUrl)}
					alt={playlist.name}
					class="size-full object-cover transition-all duration-300 group-hover:brightness-[0.4]"
				/>
			{:else}
				<div class="size-full" style="background: {gradient}"></div>
			{/if}

			<!-- Centered play icon on hover -->
			<div
				class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-300"
			>
				<Play class="size-12 fill-white stroke-white drop-shadow-lg" />
			</div>
		</div>

		<!-- Track listing -->
		<div class="px-3 pb-3 flex flex-col gap-0.5">
			{#each topTracks as track, i}
				<p class="text-sm text-foreground/80 truncate">
					<span class="text-muted-foreground mr-1">{i + 1}.</span>
					{track.name}
					{#if track.artistsName}
						<span class="text-muted-foreground"> - {track.artistsName}</span>
					{/if}
				</p>
			{/each}
		</div>
	</div>
{:else}
	<!-- Small horizontal layout -->
	<div
		class="group flex items-center gap-3 rounded-xl bg-card border border-border/50 hover:border-border hover:shadow-md transition-all duration-200 overflow-hidden cursor-pointer p-3"
		onclick={() => onclick?.()}
		onkeydown={(e) => e.key === "Enter" && onclick?.()}
		role="button"
		tabindex="0"
	>
		<!-- Cover -->
		<div class="size-20 shrink-0 rounded-xl overflow-hidden relative">
			{#if trackCovers.length >= 3}
				<div class="relative size-full">
					{#each trackCovers as cover, i}
						{@const rotate = i === 0 ? 0 : i === 1 ? 6 : 12}
						{@const scale = i === 0 ? 1 : i === 1 ? 0.9 : 0.8}
						{@const z = 3 - i}
						<img
							src={coverSrc(cover)}
							alt=""
							class="absolute rounded-md object-cover transition-all duration-300 group-hover:brightness-[0.6]"
							style="width: 100%; height: 100%; z-index: {z}; transform: scale({scale}) rotate({rotate}deg); opacity: {i === 0 ? 1 : 0.9};"
						/>
					{/each}
				</div>
			{:else if playlist.avatarUrl}
				<img
					src={coverSrc(playlist.avatarUrl)}
					alt={playlist.name}
					class="size-full object-cover rounded-xl"
				/>
			{:else}
				<div class="size-full rounded-xl" style="background: {gradient}"></div>
			{/if}
		</div>

		<!-- Info -->
		<div class="flex flex-col justify-center gap-1 min-w-0 flex-1">
			<p class="text-sm font-medium text-foreground truncate">{playlist.name}</p>
			{#if playlist.creator}
				<p class="text-xs text-muted-foreground truncate">{playlist.creator}</p>
			{/if}
			{#if playlist.musicsCount > 0}
				<p class="text-xs text-muted-foreground">{playlist.musicsCount} 首</p>
			{/if}
		</div>

		<Play class="size-4 text-muted-foreground/40 group-hover:text-foreground/70 shrink-0 transition-colors" />
	</div>
{/if}
