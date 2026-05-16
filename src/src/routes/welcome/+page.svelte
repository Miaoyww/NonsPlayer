<script lang="ts">
	import { fly, fade, scale } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
	import { globalSettings } from '$lib/stores/global-settings.store';
	import { NONSPLAYER_NAME, NONSPLAYER_TAGLINE, AVAILABLE_ADAPTERS, THEME_OPTIONS, type MusicAdapter, type ThemeOption } from '$lib/const';
	import { Music, Headphones, Folder, Sun, Moon, Settings2, ArrowRight, ArrowLeft, CircleCheck, Plus, Trash2 } from '@lucide/svelte';

	const TOTAL_STEPS = 4; // 0: welcome, 1: adapters, 2: theme, 3: local
	let currentStep = $state(0);
	let direction = $state(1);

	// Step 1: Adapter selection
	let selectedAdapters = $state<string[]>([]);

	// Step 2: Theme selection
	let selectedTheme = $state<'light' | 'dark' | 'system'>('system');

	// Step 3: Local music folders
	let localFolders = $state<string[]>([]);

	function toggleAdapter(id: string) {
		if (selectedAdapters.includes(id)) {
			selectedAdapters = selectedAdapters.filter((a) => a !== id);
		} else {
			selectedAdapters = [...selectedAdapters, id];
		}
	}

	function addFolder() {
		localFolders = [...localFolders, ''];
	}

	function removeFolder(index: number) {
		localFolders = localFolders.filter((_, i) => i !== index);
	}

	function updateFolder(index: number, value: string) {
		localFolders = localFolders.map((f, i) => (i === index ? value : f));
	}

	function nextStep() {
		if (currentStep < TOTAL_STEPS - 1) {
			direction = 1;
			currentStep++;
		}
	}

	function prevStep() {
		if (currentStep > 0) {
			direction = -1;
			currentStep--;
		}
	}

	function skipWizard() {
		globalSettings.completeWelcome();
		goto('/');
	}

	function completeWizard() {
		globalSettings.patch({
			selectedAdapters: selectedAdapters.filter((id) => id !== 'local'),
			theme: selectedTheme,
			localMusicFolders: localFolders.filter((f) => f.trim() !== '')
		});
		globalSettings.completeWelcome();
		goto('/');
	}

	function getAdapterIcon(adapter: MusicAdapter) {
		switch (adapter.icon) {
			case 'headphones': return Headphones;
			case 'music-note': return Music;
			case 'folder': return Folder;
			default: return Music;
		}
	}

	function getThemeIcon(theme: ThemeOption) {
		switch (theme.icon) {
			case 'sun': return Sun;
			case 'moon': return Moon;
			case 'settings': return Settings2;
			default: return Sun;
		}
	}

	$effect(() => {
		if (selectedTheme === 'dark') {
			document.documentElement.classList.add('dark');
		} else if (selectedTheme === 'light') {
			document.documentElement.classList.remove('dark');
		} else {
			if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
				document.documentElement.classList.add('dark');
			} else {
				document.documentElement.classList.remove('dark');
			}
		}
	});
</script>

<div class="welcome-container">
	<!-- Background decoration -->
	<div class="bg-decoration">
		<div class="bg-circle bg-circle-1"></div>
		<div class="bg-circle bg-circle-2"></div>
		<div class="bg-circle bg-circle-3"></div>
	</div>

	<!-- Skip button -->
	<button class="skip-btn" onclick={skipWizard}>
		跳过引导
		<ArrowRight class="size-3.5" />
	</button>

	<!-- Step indicator -->
	<div class="step-indicator">
		{#each Array(TOTAL_STEPS) as _, i}
			<button
				class="step-dot"
				class:active={i === currentStep}
				class:completed={i < currentStep}
				onclick={() => { direction = i > currentStep ? 1 : -1; currentStep = i; }}
				aria-label="步骤 {i + 1}"
			>
				{#if i < currentStep}
					<CircleCheck class="size-3" />
				{:else}
					{i + 1}
				{/if}
			</button>
			{#if i < TOTAL_STEPS - 1}
				<div class="step-line" class:filled={i < currentStep}></div>
			{/if}
		{/each}
	</div>

	<!-- Step content -->
	<div class="step-content">
		{#key currentStep}
			{#if currentStep === 0}
				<div class="welcome-step" transition:fly={{ y: 30 * direction, duration: 400, easing: quintOut }}>
					<div class="welcome-logo">
						<div class="logo-icon">
							<Music class="size-16" />
						</div>
					</div>
					<h1 class="welcome-title" transition:scale={{ start: 0.9, duration: 500, delay: 100, easing: quintOut }}>
						{NONSPLAYER_NAME}
					</h1>
					<p class="welcome-tagline" transition:fade={{ duration: 400, delay: 300 }}>
						{NONSPLAYER_TAGLINE}
					</p>
					<p class="welcome-desc" transition:fade={{ duration: 400, delay: 400 }}>
						多音源驱动的跨平台音乐播放器。开始前，让我们完成一些基本设置。
					</p>
					<div class="welcome-actions" transition:fade={{ duration: 400, delay: 500 }}>
						<Button size="lg" class="start-btn" onclick={nextStep}>
							开始设置
							<ArrowRight class="size-5" />
						</Button>
					</div>
				</div>

			{:else if currentStep === 1}
				<div class="step-card-container" transition:fly={{ y: 30 * direction, duration: 400, easing: quintOut }}>
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

			{:else if currentStep === 2}
				<div class="step-card-container" transition:fly={{ y: 30 * direction, duration: 400, easing: quintOut }}>
					<Card class="step-card">
						<CardHeader>
							<CardTitle class="step-title">选择主题</CardTitle>
							<CardDescription class="step-desc">
								选择你喜欢的界面风格，随时可以在设置中切换。
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div class="theme-list">
								{#each THEME_OPTIONS as theme}
									{@const ThemeIcon = getThemeIcon(theme)}
									<button
										class="theme-item"
										class:selected={selectedTheme === theme.id}
										onclick={() => { selectedTheme = theme.id as 'light' | 'dark' | 'system'; }}
									>
										<div class="theme-preview" class:theme-light={theme.id === 'light'} class:theme-dark={theme.id === 'dark'} class:theme-system={theme.id === 'system'}>
											<ThemeIcon class="size-8" />
										</div>
										<div class="theme-info">
											<span class="theme-name">{theme.name}</span>
											<span class="theme-desc">{theme.description}</span>
										</div>
										<div class="theme-radio" class:checked={selectedTheme === theme.id}>
											{#if selectedTheme === theme.id}
												<div class="radio-dot"></div>
											{/if}
										</div>
									</button>
								{/each}
							</div>
						</CardContent>
					</Card>
				</div>

			{:else if currentStep === 3}
				<div class="step-card-container" transition:fly={{ y: 30 * direction, duration: 400, easing: quintOut }}>
					<Card class="step-card">
						<CardHeader>
							<CardTitle class="step-title">本地音乐</CardTitle>
							<CardDescription class="step-desc">
								添加本地音乐文件夹，NonsPlayer 会自动扫描其中的音频文件。你可以在设置中随时添加更多文件夹。
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div class="folder-section">
								{#if localFolders.length === 0}
									<div class="folder-empty">
										<Folder class="size-12 text-muted-foreground/40" />
										<p class="text-muted-foreground text-sm">暂未添加文件夹</p>
									</div>
								{/if}
								<div class="folder-list">
									{#each localFolders as folder, i}
										<div class="folder-item" transition:fly={{ y: -10, duration: 200 }}>
											<Folder class="size-4 text-amber-500 shrink-0" />
											<input
												type="text"
												class="folder-input"
												placeholder="输入音乐文件夹路径，如 C:\Users\You\Music"
												value={folder}
												oninput={(e) => updateFolder(i, (e.target as HTMLInputElement).value)}
											/>
											<button class="folder-remove" onclick={() => removeFolder(i)}>
												<Trash2 class="size-4" />
											</button>
										</div>
									{/each}
								</div>
								<button class="folder-add-btn" onclick={addFolder}>
									<Plus class="size-4" />
									添加文件夹
								</button>
							</div>
						</CardContent>
					</Card>
				</div>
			{/if}
		{/key}
	</div>

	<!-- Navigation -->
	<div class="step-nav" transition:fade={{ duration: 300 }}>
		{#if currentStep > 0}
			<Button variant="outline" onclick={prevStep}>
				<ArrowLeft class="size-4" />
				上一步
			</Button>
		{:else}
			<div></div>
		{/if}

		{#if currentStep < TOTAL_STEPS - 1}
			<Button onclick={nextStep}>
				下一步
				<ArrowRight class="size-4" />
			</Button>
		{:else}
			<Button onclick={completeWizard}>
				<CircleCheck class="size-4" />
				完成设置
			</Button>
		{/if}
	</div>
</div>

<style>
	.welcome-container {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		padding: 2rem;
		overflow: hidden;
	}

	.bg-decoration {
		position: fixed;
		inset: 0;
		pointer-events: none;
		overflow: hidden;
	}

	.bg-circle {
		position: absolute;
		border-radius: 50%;
		opacity: 0.15;
		filter: blur(80px);
	}

	.bg-circle-1 {
		width: 500px;
		height: 500px;
		background: var(--primary);
		top: -150px;
		right: -100px;
		animation: float1 12s ease-in-out infinite;
	}

	.bg-circle-2 {
		width: 350px;
		height: 350px;
		background: var(--chart-1);
		bottom: -100px;
		left: -80px;
		animation: float2 15s ease-in-out infinite;
	}

	.bg-circle-3 {
		width: 250px;
		height: 250px;
		background: var(--chart-2);
		top: 50%;
		left: 50%;
		animation: float3 10s ease-in-out infinite;
	}

	@keyframes float1 {
		0%, 100% { transform: translate(0, 0) scale(1); }
		33% { transform: translate(-30px, 20px) scale(1.1); }
		66% { transform: translate(20px, -10px) scale(0.95); }
	}

	@keyframes float2 {
		0%, 100% { transform: translate(0, 0) scale(1); }
		33% { transform: translate(20px, -15px) scale(1.05); }
		66% { transform: translate(-15px, 10px) scale(0.9); }
	}

	@keyframes float3 {
		0%, 100% { transform: translate(0, 0) scale(1); }
		50% { transform: translate(-15px, -15px) scale(1.08); }
	}

	.skip-btn {
		position: fixed;
		top: 1.5rem;
		right: 1.5rem;
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.375rem 0.75rem;
		border-radius: 0.5rem;
		transition: all 0.2s;
		z-index: 10;
	}

	.skip-btn:hover {
		color: var(--foreground);
		background: var(--muted);
	}

	.step-indicator {
		display: flex;
		align-items: center;
		gap: 0;
		margin-bottom: 2rem;
		z-index: 1;
	}

	.step-dot {
		width: 2rem;
		height: 2rem;
		border-radius: 50%;
		border: 2px solid var(--border);
		background: var(--background);
		color: var(--muted-foreground);
		font-size: 0.75rem;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.3s;
		flex-shrink: 0;
	}

	.step-dot.active {
		border-color: var(--primary);
		background: var(--primary);
		color: var(--primary-foreground);
	}

	.step-dot.completed {
		border-color: var(--primary);
		background: var(--primary);
		color: var(--primary-foreground);
	}

	.step-line {
		width: 3rem;
		height: 2px;
		background: var(--border);
		transition: background 0.3s;
	}

	.step-line.filled {
		background: var(--primary);
	}

	.step-content {
		width: 100%;
		max-width: 520px;
		z-index: 1;
	}

	.welcome-step {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 1rem;
	}

	.welcome-logo {
		margin-bottom: 0.5rem;
	}

	.logo-icon {
		width: 5rem;
		height: 5rem;
		border-radius: 1.25rem;
		background: var(--primary);
		color: var(--primary-foreground);
		display: flex;
		align-items: center;
		justify-content: center;
		box-shadow: 0 10px 40px -10px var(--primary);
	}

	.welcome-title {
		font-size: 2.5rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: var(--foreground);
		margin: 0;
	}

	.welcome-tagline {
		font-size: 1.125rem;
		color: var(--primary);
		font-weight: 500;
		margin: 0;
	}

	.welcome-desc {
		font-size: 0.9375rem;
		color: var(--muted-foreground);
		max-width: 380px;
		line-height: 1.6;
		margin: 0;
	}

	.welcome-actions {
		margin-top: 1rem;
	}

	.start-btn {
		gap: 0.5rem;
		padding: 0.625rem 2rem;
		font-size: 0.9375rem;
	}

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

	.theme-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.theme-item {
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

	.theme-item:hover {
		border-color: var(--ring);
		background: var(--accent);
	}

	.theme-item.selected {
		border-color: var(--primary);
		background: var(--primary) / 0.05;
	}

	.theme-preview {
		width: 2.75rem;
		height: 2.75rem;
		border-radius: 0.75rem;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.theme-preview.theme-light {
		background: #fef3c7;
		color: #f59e0b;
	}

	.theme-preview.theme-dark {
		background: #1e293b;
		color: #6366f1;
	}

	.theme-preview.theme-system {
		background: linear-gradient(135deg, #fef3c7 50%, #1e293b 50%);
		color: #8b5cf6;
	}

	.theme-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.theme-name {
		font-weight: 600;
		font-size: 0.9375rem;
		color: var(--foreground);
	}

	.theme-desc {
		font-size: 0.8125rem;
		color: var(--muted-foreground);
	}

	.theme-radio {
		width: 1.25rem;
		height: 1.25rem;
		border-radius: 50%;
		border: 2px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition: all 0.2s;
	}

	.theme-radio.checked {
		border-color: var(--primary);
	}

	.radio-dot {
		width: 0.625rem;
		height: 0.625rem;
		border-radius: 50%;
		background: var(--primary);
	}

	.folder-section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.folder-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 1.5rem;
		border: 2px dashed var(--border);
		border-radius: 0.75rem;
	}

	.folder-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.folder-item {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.625rem 0.75rem;
		border: 1px solid var(--border);
		border-radius: 0.5rem;
		background: var(--background);
	}

	.folder-input {
		flex: 1;
		border: none;
		outline: none;
		background: transparent;
		font-size: 0.8125rem;
		color: var(--foreground);
		min-width: 0;
	}

	.folder-input::placeholder {
		color: var(--muted-foreground);
	}

	.folder-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.25rem;
		border: none;
		background: none;
		color: var(--muted-foreground);
		cursor: pointer;
		border-radius: 0.25rem;
		transition: all 0.2s;
		flex-shrink: 0;
	}

	.folder-remove:hover {
		color: var(--destructive);
		background: var(--destructive) / 0.1;
	}

	.folder-add-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.875rem;
		border: 1.5px dashed var(--border);
		border-radius: 0.5rem;
		background: var(--background);
		color: var(--muted-foreground);
		font-size: 0.8125rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.folder-add-btn:hover {
		border-color: var(--ring);
		color: var(--foreground);
		background: var(--accent);
	}

	.step-nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		max-width: 520px;
		margin-top: 1.5rem;
		z-index: 1;
	}
</style>
