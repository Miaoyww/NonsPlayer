<script lang="ts">
	import { ArrowLeft, Map, Puzzle, Settings, Info, Sword } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import VenuePage from '$lib/components/settings/pages/common/venue.svelte';
	import ModsPage from '$lib/components/settings/pages/common/mods.svelte';
	import GeneralPage from '$lib/components/settings/pages/common/general.svelte';
	import AboutPage from '$lib/components/settings/pages/common/about.svelte';
	import BattlePage from '$lib/components/settings/pages/battle/battle.svelte';
	import { fly } from 'svelte/transition';
	import Footer from '$lib/components/footer.svelte';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import ScrollArea from '../ui/scroll-area/scroll-area.svelte';

	let activeSection = $state<Section>('venue');
	let backUrl = $state('/');
	const battleId = page.params.battle_id ?? null;
	type Section = 'venue' | 'mods' | 'general' | 'about' | 'battle';

	interface NavItem {
		key: Section;
		label: string;
		icon: typeof Map;
	}

	let NAV_ITEMS: NavItem[] = $state([
		{ key: 'mods', label: 'Mod 管理', icon: Puzzle },
		{ key: 'general', label: '常规', icon: Settings },
		{ key: 'about', label: '关于', icon: Info }
	]);

	onMount(() => {
		if (battleId) {
			NAV_ITEMS.unshift({ key: 'battle', label: '战役', icon: Sword });
			backUrl = `/battle/${battleId}`;
			return;
		}
		NAV_ITEMS.unshift({ key: 'venue', label: '会场', icon: Map });
	});
</script>

<div
	class="min-h-screen w-screen bg-gradient-to-br from-slate-100 to-stone-200 dark:from-slate-900 dark:to-stone-900"
>
	<div class="veto-page ml-5 w-fit gap-3">
		<a
			href={backUrl}
			class="inline-flex items-center justify-center rounded-md p-2 text-stone-600 transition-colors hover:bg-stone-200/50 hover:text-stone-900 dark:text-stone-400 dark:hover:bg-stone-700/50 dark:hover:text-stone-100"
		>
			<ArrowLeft class="h-5 w-5" />
		</a>
		<!-- 如果这个 div 没有任何内容，可以考虑移除或者保留作为间距占位 -->
		<p class="mr-5 flex gap-2">返回</p>
	</div>

	<div class="flex w-screen flex-col" in:fly={{ y: 16, duration: 300, opacity: 0 }}>
		<!-- 主体 -->
		<div class="flex gap-6 p-5 pb-6">
			<!-- 左侧导航 -->
			<div class="veto-page w-36 shrink-0 flex-col gap-1 self-start">
				{#each NAV_ITEMS as item}
					<Button
						class="w-full cursor-pointer justify-start gap-2 px-3"
						variant={activeSection === item.key ? 'secondary' : 'ghost'}
						onclick={() => (activeSection = item.key)}
					>
						<item.icon size={15} />
						<span class="text-sm">{item.label}</span>
					</Button>
				{/each}
			</div>

			<!-- 右侧内容 -->
			<div class="veto-page h-[calc(100vh-160px)] flex-1 rounded-lg">
				<ScrollArea class="h-full w-full">
					<div class="p-6">
						{#if activeSection === 'venue'}<VenuePage />{/if}
						{#if activeSection === 'battle'}<BattlePage />{/if}
						{#if activeSection === 'mods'}<ModsPage />{/if}
						{#if activeSection === 'general'}<GeneralPage />{/if}
						{#if activeSection === 'about'}<AboutPage />{/if}
					</div>
				</ScrollArea>
			</div>
		</div>

		<Footer />
	</div>
</div>
