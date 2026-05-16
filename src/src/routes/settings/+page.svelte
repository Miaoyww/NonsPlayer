<script lang="ts">
	import { ArrowLeft, Map, Settings, Info, } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import GeneralPage from '$lib/components/settings/pages/common/general.svelte';
	import AboutPage from '$lib/components/settings/pages/common/about.svelte';
	import { fly } from 'svelte/transition';
	import Footer from '$lib/components/footer.svelte';
	import ScrollArea from '$lib/components/ui/scroll-area/scroll-area.svelte';

	let activeSection = $state<Section>('general');
	type Section =  | 'general' | 'about';

	interface NavItem {
		key: Section;
		label: string;
		icon: typeof Map;
	}

	let NAV_ITEMS: NavItem[] = $state([
		{ key: 'general', label: '常规', icon: Settings },
		{ key: 'about', label: '关于', icon: Info }
	]);

</script>

<div
	class="min-h-screen w-screen bg-gradient-to-br from-slate-100 to-stone-200 dark:from-slate-900 dark:to-stone-900"
>
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
						{#if activeSection === 'general'}<GeneralPage />{/if}
						{#if activeSection === 'about'}<AboutPage />{/if}
					</div>
				</ScrollArea>
			</div>
		</div>

		<Footer />
	</div>
</div>
