<script lang="ts">
	import * as Card from '$lib/components/ui/card';

	interface Props {
		playlistName: string;
		coverUrl?: string;
		onclick?: () => void;
	}

	let { playlistName, coverUrl = '', onclick }: Props = $props();
</script>

<button
	class="shrink-0 cursor-pointer text-left group"
	onclick={() => onclick?.()}
>
	<Card.Root class="flex flex-row items-center rounded-lg border-0  bg-transparent">
		<!-- 封面图 -->
		<div class="size-10 shrink-0 rounded-lg overflow-hidden bg-muted">
			{#if coverUrl}
				<img src={coverUrl} alt={playlistName} class="size-10  object-cover" />
			{/if}
		</div>
		<!-- 歌单名 -->
		<Card.Content class="p-0 w-50">
			<p class="text-sm font-bold text-foreground truncate">{playlistName}</p>
		</Card.Content>
	</Card.Root>
</button>
