<script lang="ts">
	import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
	import { globalSettings } from '$lib/stores/global-settings-store';
	import { AVAILABLE_ADAPTERS, type MusicAdapter } from '$lib/const';
	import { Music, Headphones, Folder, CircleCheck } from '@lucide/svelte';
	import { onMount } from 'svelte';

	let selectedAdapters = $state<string[]>([]);

	onMount(() => {
		const unsub = globalSettings.subscribe((s) => {
			selectedAdapters = [...s.selectedAdapters];
		});
		return unsub;
	});

	function toggleAdapter(id: string) {
		if (selectedAdapters.includes(id)) {
			selectedAdapters = selectedAdapters.filter((a) => a !== id);
		} else {
			selectedAdapters = [...selectedAdapters, id];
		}
		// 即时保存到 store
		globalSettings.patch({ selectedAdapters });
	}

	function getAdapterIcon(adapter: MusicAdapter) {
		switch (adapter.icon) {
			case 'headphones': return Headphones;
			case 'music-note': return Music;
			case 'folder': return Folder;
			default: return Music;
		}
	}
</script>

<div class="step-card-container">
	<Card class="step-card">
		<CardHeader>
			<CardTitle class="step-title">选择音乐平台</CardTitle>
			<CardDescription class="step-desc">
				选择你想使用的音乐平台，可以多选。后续可以在设置中随时更改。
			</CardDescription>
		</CardHeader>
		<CardContent>
			<div class="adapter-list">
				{#each AVAILABLE_ADAPTERS as adapter}
					{@const AdapterIcon = getAdapterIcon(adapter)}
					<button
						class="adapter-item"
						class:selected={selectedAdapters.includes(adapter.id)}
						onclick={() => toggleAdapter(adapter.id)}
					>
						<div class="adapter-icon {adapter.color}">
							<AdapterIcon class="size-7" />
						</div>
						<div class="adapter-info">
							<span class="adapter-name">{adapter.name}</span>
							<span class="adapter-desc">{adapter.description}</span>
						</div>
						<div class="adapter-check" class:checked={selectedAdapters.includes(adapter.id)}>
							{#if selectedAdapters.includes(adapter.id)}
								<CircleCheck class="size-5 text-primary" />
							{/if}
						</div>
					</button>
				{/each}
			</div>
		</CardContent>
	</Card>
</div>

<style>
	.step-card-container {
		width: 100%;
	}

	.step-card {
		border: 1px solid var(--border);
		box-shadow: 0 4px 24px rgba(0, 0, 0, 0.06);
	}

	.step-title {
		font-size: 1.25rem;
		text-align: center;
	}

	.step-desc {
		text-align: center;
		line-height: 1.5;
	}

	.adapter-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.adapter-item {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		border: 1.5px solid var(--border);
		border-radius: 0.75rem;
		background: var(--background);
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		width: 100%;
	}

	.adapter-item:hover {
		border-color: var(--ring);
		background: var(--accent);
	}

	.adapter-item.selected {
		border-color: var(--primary);
		background: var(--primary) / 0.05;
	}

	.adapter-icon {
		width: 2.75rem;
		height: 2.75rem;
		border-radius: 0.75rem;
		background: var(--muted);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.adapter-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		min-width: 0;
	}

	.adapter-name {
		font-weight: 600;
		font-size: 0.9375rem;
		color: var(--foreground);
	}

	.adapter-desc {
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		line-height: 1.4;
	}

	.adapter-check {
		width: 1.5rem;
		height: 1.5rem;
		border-radius: 50%;
		border: 2px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: all 0.2s;
	}

	.adapter-check.checked {
		border-color: var(--primary);
		background: var(--primary) / 0.1;
	}
</style>
