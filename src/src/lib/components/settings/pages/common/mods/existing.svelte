<script lang="ts">
	import { Button } from '$lib/components/ui/button';
</script>

<div class="space-y-4">
	{#if dbPlugins.length > 0}
		<div>
			<p class="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">已安装</p>
			<div class="flex flex-col gap-2">
				{#each dbPlugins as plugin (plugin.id)}
					<div class="flex flex-col gap-1.5 rounded-lg border border-border bg-card p-3">
						<div class="flex items-center justify-between gap-2">
							<div class="min-w-0">
								<p class="truncate text-sm font-semibold text-foreground">{plugin.manifest.name}</p>
								<p class="text-xs text-muted-foreground">
									v{plugin.manifest.version} · {plugin.manifest.author}
									· 安装于 {new Date(plugin.installedAt).toLocaleDateString('zh-CN')}
								</p>
							</div>
							<Button
								variant="ghost"
								size="icon"
								class="size-7 shrink-0 text-muted-foreground hover:text-destructive"
								title="卸载"
								onclick={() => handleUninstall(plugin)}
							>
								<Trash2 class="size-3.5" />
							</Button>
						</div>
						{#if plugin.manifest.description}
							<p class="text-xs text-muted-foreground">{plugin.manifest.description}</p>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{:else}
		<div class="flex min-h-60 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-stone-300 text-stone-400 dark:border-stone-700 dark:text-stone-500">
			<PackageCheck size={32} class="opacity-40" />
			<span class="text-sm">尚未安装任何 Mod</span>
			<span class="text-xs">在「安装」标签页中从注册中心或本地导入</span>
		</div>
	{/if}
</div>

