<script lang="ts">
		import { Play } from "@lucide/svelte";
		import type { Playlist } from "$lib/types";
		import { coverSrc } from "$lib/utils";

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

	<button
		class="flex items-center gap-2.5 w-full rounded-xl p-2 cursor-pointer text-left
		       bg-card border border-border/50
		       hover:bg-accent/40 hover:border-border hover:shadow-md
		       transition-all duration-200 group"
		onclick={() => onclick?.()}
	>
		<div class="size-10 shrink-0 rounded-lg overflow-hidden shadow-sm">
			{#if playlist.avatarUrl}
				<img src={coverSrc(playlist.avatarUrl)} alt={playlist.name} class="size-10 object-cover" />
			{:else}
				<div class="size-10" style="background: {gradient}"></div>
			{/if}
		</div>
		<p class="text-sm font-medium text-foreground truncate flex-1">{playlist.name}</p>
		<Play class="size-3.5 text-muted-foreground/50 shrink-0 group-hover:text-foreground/80 transition-colors" />
	</button>
