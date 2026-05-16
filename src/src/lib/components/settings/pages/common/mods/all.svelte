<script lang="ts">
	// 直接从 registry 获取响应式列表
	const modList = $derived(registry.getModList());

	// 按来源分类
	const userMods = $derived(modList.filter((m) => m.metadata?.source === 'user'));
	const systemMods = $derived(modList.filter((m) => m.metadata?.source === 'system'));
</script>

<div class="space-y-4">
	<!--系统内置-->
	<div>
		<p class="mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">系统内置</p>
		<div class="flex flex-col gap-2">
			{#each systemMods as entry (entry.id)}
				<ModCard {entry} />
			{/each}
		</div>
	</div>

	{#if userMods.length > 0}
		<div>
			<p class="mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">用户安装</p>
			<div class="flex flex-col gap-2">
				{#each userMods as entry (entry.id || entry.metadata?.id || JSON.stringify(entry))}
					<ModCard {entry} />
				{/each}
			</div>
		</div>
	{/if}

	{#if modList.length === 0}
		<div
			class="flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-stone-300 py-16 text-stone-400 dark:border-stone-700 dark:text-stone-500"
		>
			<Puzzle size={32} class="opacity-40" />
			<span class="text-sm">暂无任何 Mod</span>
		</div>
	{/if}
</div>
