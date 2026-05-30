<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import Button from "$lib/components/ui/button/button.svelte";
	import ScrollArea from "$lib/components/ui/scroll-area/scroll-area.svelte";
	import { Tags } from "@lucide/svelte";
	import type { PlaylistCategory } from "$lib/types";

	interface Props {
		categories: PlaylistCategory[];
		selectedTag: string;
		open: boolean;
		onopenchange: (open: boolean) => void;
		onselect: (catName: string) => void;
	}

	let { categories, selectedTag, open = $bindable(), onopenchange, onselect }: Props = $props();
</script>

<Dialog.Root bind:open onOpenChange={onopenchange}>
	<Dialog.Content class="sm:max-w-lg max-h-[80vh] flex flex-col">
		<Dialog.Header>
			<Dialog.Title class="flex items-center gap-2">
				<Tags class="size-4" />
				歌单分类
			</Dialog.Title>
			<Dialog.Description>
				选择你喜欢的音乐风格
			</Dialog.Description>
		</Dialog.Header>

		<ScrollArea class="flex-1 min-h-0">
			<div class="flex flex-col gap-4 pr-2">
				<!-- "全部歌单" option -->
				<div class="flex flex-wrap gap-1.5">
					<button
						class="inline-flex items-center px-3 py-1 rounded-full text-sm transition-colors cursor-pointer
							{selectedTag === '全部歌单'
							? 'bg-primary text-primary-foreground'
							: 'bg-muted hover:bg-accent text-foreground'}"
						onclick={() => onselect("全部歌单")}
					>
						全部歌单
					</button>
				</div>

				{#each categories as cat}
					<div class="flex flex-col gap-1">
						<p class="text-sm font-semibold text-foreground">{cat.name}</p>
						<div class="flex flex-wrap gap-1.5">
							{#each cat.tags as tag}
								<button
									class="inline-flex items-center px-3 py-1 rounded-full text-sm transition-colors cursor-pointer
										{selectedTag === tag
										? 'bg-primary text-primary-foreground'
										: 'bg-muted hover:bg-accent text-foreground'}"
									onclick={() => onselect(tag)}
								>
									{tag}
								</button>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</ScrollArea>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => onopenchange(false)}>关闭</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
