<script lang="ts">
	import { Play } from "@lucide/svelte";

	interface Props {
		playlistName: string;
		coverUrl?: string;
		onclick?: () => void;
	}

	let { playlistName, coverUrl = '', onclick }: Props = $props();

	function getGradient(name: string): string {
		let hash = 0;
		for (let i = 0; i < name.length; i++) {
			hash = name.charCodeAt(i) + ((hash << 5) - hash);
		}
		const h1 = Math.abs(hash % 360);
		const h2 = (h1 + 40) % 360;
		return `linear-gradient(135deg, hsl(${h1}, 50%, 45%), hsl(${h2}, 60%, 38%))`;
	}

	const gradient = getGradient(playlistName);
</script>

<button
	class="flex items-center gap-2.5 w-full rounded-xl p-2 cursor-pointer text-left
	       bg-card border border-border/50
	       hover:bg-accent/40 hover:border-border hover:shadow-md
	       transition-all duration-200 group"
	onclick={() => onclick?.()}
>
	<div class="size-10 shrink-0 rounded-lg overflow-hidden shadow-sm">
		{#if coverUrl}
			<img src={coverUrl} alt={playlistName} class="size-10 object-cover" />
		{:else}
			<div class="size-10" style="background: {gradient}"></div>
		{/if}
	</div>
	<p class="text-sm font-medium text-foreground truncate flex-1">{playlistName}</p>
	<Play class="size-3.5 text-muted-foreground/50 shrink-0
	             opacity-0 group-hover:opacity-100 transition-opacity duration-200" />
</button>
