<script lang="ts">
	import {
		Upload,
		RefreshCw,
		Store,
		Loader,
		WifiOff,
		CheckCircle2,
		AlertCircle,
		FolderOpen,
		X
	} from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { onMount } from 'svelte';

	// ── 本地 .csmod 导入 ──
	type ImportState = 'idle' | 'loading' | 'done' | 'error';
	let importState = $state<ImportState>('idle');
	let importError = $state<string | null>(null);
	let importedName = $state<string | null>(null);
	let dragOver = $state(false);
	let fileInput: HTMLInputElement;

	async function handleFiles(files: FileList | null) {
		const file = files?.[0];
		if (!file) return;
		importState = 'loading';
		importError = null;
		importedName = null;
		try {
			const plugin = await importModPackage(file);
			importedName = plugin.manifest.name;
			importState = 'done';
		} catch (err) {
			importError = err instanceof Error ? err.message : '导入失败';
			importState = 'error';
		}
	}

	function resetImport() {
		importState = 'idle';
		importError = null;
		importedName = null;
		if (fileInput) fileInput.value = '';
	}

	function onDragOver(e: DragEvent) {
		e.preventDefault();
		dragOver = true;
	}
	function onDragLeave() {
		dragOver = false;
	}
	function onDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		handleFiles(e.dataTransfer?.files ?? null);
	}

	// ── 在线注册中心 ──
	type FetchState = 'idle' | 'loading' | 'done' | 'error';
	let fetchState = $state<FetchState>('idle');
	let remotePlugins = $state<PluginManifest[]>([]);
	let fetchError = $state<string | null>(null);
	let installedIds = $state(new Set<string>());
	let starsMap = $state<Record<string, number>>({});

	onMount(() => {
		loadRegistry();
	});

	async function loadRegistry() {
		fetchState = 'loading';
		fetchError = null;
		starsMap = {};
		try {
			const [plugins, dbPlugins] = await Promise.all([
				fetchPluginRegistry(),
				dbGetAllPlugins()
			]);
			remotePlugins = plugins;
			installedIds = new Set(dbPlugins.map((p) => p.id));
			fetchState = 'done';
			// stars 在后台独立加载，完成后通过自定义事件通知各卡片
			fetchPluginStars().then((stars) => {
				starsMap = stars;
				window.dispatchEvent(new CustomEvent('veto:stars-loaded', { detail: stars }));
			});
		} catch (err) {
			fetchError = err instanceof Error ? err.message : '未知错误';
			fetchState = 'error';
		}
	}

	function markInstalled(id: string) {
		installedIds = new Set([...installedIds, id]);
	}
</script>

<div class="space-y-5">
	<!-- ── 本地 .csmod 导入区 ── -->
	<div>
		<p class="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">本地导入</p>

		<!-- 隐藏 file input -->
		<input
			bind:this={fileInput}
			type="file"
			accept=".csmod"
			class="hidden"
			onchange={(e) => handleFiles((e.currentTarget as HTMLInputElement).files)}
		/>

		{#if importState === 'idle'}
			<!-- 拖放区 -->
			<div
				role="button"
				tabindex="0"
				class="flex min-h-36 cursor-pointer flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed transition-colors
					{dragOver
					? 'border-primary bg-primary/5 text-primary'
					: 'border-stone-300 text-stone-400 hover:border-stone-400 hover:text-stone-500 dark:border-stone-700 dark:text-stone-500'}"
				ondragover={onDragOver}
				ondragleave={onDragLeave}
				ondrop={onDrop}
				onclick={() => fileInput.click()}
				onkeydown={(e) => e.key === 'Enter' && fileInput.click()}
			>
				<Upload class="size-7 opacity-60" />
				<div class="text-center">
					<p class="text-sm font-medium">拖放 .csmod 文件，或点击选择</p>
					<p class="mt-0.5 text-xs opacity-70">支持本地离线导入，无需网络</p>
				</div>
				<Button variant="outline" size="sm" class="gap-1.5" onclick={(e) => { e.stopPropagation(); fileInput.click(); }}>
					<FolderOpen class="size-3.5" />
					选择文件
				</Button>
			</div>

		{:else if importState === 'loading'}
			<div class="flex min-h-36 flex-col items-center justify-center gap-3 rounded-lg border border-border text-muted-foreground">
				<Loader class="size-6 animate-spin opacity-60" />
				<p class="text-sm">正在解析并导入…</p>
			</div>

		{:else if importState === 'done'}
			<div class="flex items-center justify-between gap-3 rounded-lg border border-green-200 bg-green-50 px-4 py-3 dark:border-green-900 dark:bg-green-950/40">
				<div class="flex items-center gap-2 text-green-700 dark:text-green-400">
					<CheckCircle2 class="size-4 shrink-0" />
					<div>
						<p class="text-sm font-medium">导入成功</p>
						<p class="text-xs opacity-80">"{importedName}" 已载入并持久化</p>
					</div>
				</div>
				<Button variant="ghost" size="icon" class="size-7 shrink-0" onclick={resetImport} title="导入另一个">
					<X class="size-4" />
				</Button>
			</div>

		{:else if importState === 'error'}
			<div class="flex items-center justify-between gap-3 rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-3">
				<div class="flex items-center gap-2 text-destructive">
					<AlertCircle class="size-4 shrink-0" />
					<div>
						<p class="text-sm font-medium">导入失败</p>
						<p class="text-xs opacity-80">{importError}</p>
					</div>
				</div>
				<Button variant="ghost" size="icon" class="size-7 shrink-0" onclick={resetImport} title="重试">
					<X class="size-4" />
				</Button>
			</div>
		{/if}
	</div>

	<!-- 分隔线 -->
	<div class="flex items-center gap-3">
		<div class="h-px flex-1 bg-border"></div>
		<span class="text-xs text-muted-foreground">在线安装</span>
		<div class="h-px flex-1 bg-border"></div>
	</div>

	<!-- ── 在线注册中心 ── -->
	<div class="space-y-3">
		{#if fetchState === 'idle'}
			<div class="flex min-h-48 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-stone-300 text-stone-400 dark:border-stone-700 dark:text-stone-500">
				<Store size={28} class="opacity-40" />
				<p class="text-sm">从 VetoExpress 官方插件库获取插件列表</p>
				<Button variant="outline" size="sm" class="gap-1.5" onclick={loadRegistry}>
					<RefreshCw class="size-3.5" />获取列表
				</Button>
			</div>
		{:else if fetchState === 'loading'}
			<div class="flex min-h-48 flex-col items-center justify-center gap-3 text-muted-foreground">
				<Loader class="size-7 animate-spin opacity-60" />
				<p class="text-sm">正在从插件库拉取…</p>
			</div>
		{:else if fetchState === 'error'}
			<div class="flex min-h-48 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-destructive/40 text-destructive/70">
				<WifiOff size={28} class="opacity-60" />
				<p class="text-sm">{fetchError}</p>
				<Button variant="outline" size="sm" class="gap-1.5" onclick={loadRegistry}>
					<RefreshCw class="size-3.5" />重试
				</Button>
			</div>
		{:else}
			<div class="flex items-center justify-between">
				<span class="text-xs text-muted-foreground">共 {remotePlugins.length} 个插件</span>
				<Button variant="ghost" size="sm" class="h-7 gap-1 px-2 text-xs" onclick={loadRegistry}>
					<RefreshCw class="size-3" />刷新
				</Button>
			</div>
			<div class="flex flex-col gap-2">
				{#each remotePlugins as manifest (manifest.id)}
					<RegistryPluginCard
						{manifest}
						installed={installedIds.has(manifest.id)}
						oninstalled={markInstalled}
					/>
				{/each}
				{#if remotePlugins.length === 0}
					<p class="py-8 text-center text-sm text-muted-foreground">暂无插件</p>
				{/if}
			</div>
		{/if}
	</div>
</div>
