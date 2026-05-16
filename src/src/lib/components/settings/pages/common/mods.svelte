<script lang="ts">
	import { ExternalLink } from '@lucide/svelte';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { registry } from '$lib/registry/mod-registry.svelte';
	import AllMods from './mods/all.svelte';
	import ExistingMods from './mods/existing.svelte';
	import InstallMod from './mods/install.svelte';
	import { fly } from 'svelte/transition';

	const stats = $derived.by(() => {
		return {
			branches: registry.getBranchesSize(),
			categories: registry.getCategoriesSize(),
			templates: registry.getUnitTemplatesSize()
		};
	});
</script>

<div class="flex flex-col" in:fly={{ y: 8, duration: 320, opacity: 0 }}>
	<div class="mb-1 text-xl font-bold text-stone-800 dark:text-stone-100">Mod 管理</div>
	<p class="mb-4 text-sm text-muted-foreground">
		管理已安装的扩展包，Mod 以 <code class="rounded bg-muted px-1 py-0.5 font-mono text-xs"
			>.csmod</code
		>
		格式导入。可在
		<a
			href="https://github.com/VetoExpress/veto-plugins"
			target="_blank"
			rel="noreferrer"
			class="inline-flex items-center gap-1 text-stone-600 underline underline-offset-2 hover:text-stone-900 dark:text-stone-400 dark:hover:text-stone-200"
		>
			veto-plugins <ExternalLink size={11} />
		</a>
		浏览社区 Mod。
	</p>

	<!-- 统计栏 -->
	<div class="mb-4 grid grid-cols-3 gap-2">
		{#each [{ label: '军种', value: stats.branches }, { label: '单位大类', value: stats.categories }, { label: '单位模板', value: stats.templates }] as stat}
			<div class="rounded-lg border border-border bg-muted/40 px-3 py-2.5 text-center">
				<div class="text-lg font-bold text-foreground">{stat.value}</div>
				<div class="text-xs text-muted-foreground">{stat.label}</div>
			</div>
		{/each}
	</div>

	<Tabs.Root value="all">
		<Tabs.List>
			<Tabs.Trigger value="all">全部</Tabs.Trigger>
			<Tabs.Trigger value="installed">已安装</Tabs.Trigger>
			<Tabs.Trigger value="install">安装</Tabs.Trigger>
		</Tabs.List>

		<Tabs.Content value="all">
			<AllMods />
		</Tabs.Content>

		<Tabs.Content value="installed">
			<ExistingMods />
		</Tabs.Content>

		<Tabs.Content value="install">
			<InstallMod />
		</Tabs.Content>
	</Tabs.Root>
</div>
