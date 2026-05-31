<script lang="ts">
	import { Play } from "@lucide/svelte";
	import type { Playlist } from "$lib/types";
	import { coverSrc, formatCount } from "$lib/utils";

	interface Props {
		playlist: Playlist;
		onclick?: () => void;
	}

	let { playlist, onclick }: Props = $props();

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
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="group cursor-pointer"
	onclick={() => onclick?.()}
	onkeydown={(e) => e.key === "Enter" && onclick?.()}
	role="button"
	tabindex="0"
>
	<!-- Cover -->
	<div class="relative aspect-square rounded-2xl overflow-hidden shadow-sm">
		{#if playlist.avatarUrl}
			<img
				src={coverSrc(playlist.avatarUrl)}
				alt={playlist.name}
				class="size-full object-cover transition-all duration-300 group-hover:scale-110 group-hover:brightness-[0.8]"
			/>
		{:else}
			<div class="size-full" style="background: {gradient}"></div>
		{/if}

		<!-- Play count badge (top-right) -->
		{#if playlist.playCount > 0}
			<div
				class="absolute top-0 right-0 pt-2 pr-2.5 flex items-center gap-1 text-white text-xs drop-shadow"
			>
				<Play class="size-3 fill-white stroke-white" />
				<span>{formatCount(playlist.playCount)}</span>
			</div>
		{/if}

		<!-- Play button (bottom-right, slides up on hover) -->
		<div
			class="absolute bottom-2.5 right-2.5 w-10 h-10 rounded-full bg-primary/90 flex items-center justify-center opacity-0 translate-y-3 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-300 shadow-lg"
		>
			<Play class="size-5 fill-primary-foreground stroke-primary-foreground ml-0.5" />
		</div>
	</div>

	<!-- Info -->
	<div class="mt-2 flex flex-col gap-0.5 px-0.5">
		<p class="text-sm font-medium text-foreground line-clamp-2 leading-tight">
			{playlist.name}
		</p>
		{#if playlist.creator}
			<p class="text-xs text-muted-foreground truncate">{playlist.creator}</p>
		{/if}
		{#if playlist.musicsCount > 0}
			<p class="text-xs text-muted-foreground">{playlist.musicsCount} 首</p>
		{/if}
	</div>
</div>
