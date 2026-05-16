<script lang="ts">
	import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '$lib/components/ui/card';
	import { globalSettings } from '$lib/stores/global-settings.store';
	import { THEME_OPTIONS, type ThemeOption } from '$lib/const';
	import { Sun as SunIcon, Moon, Settings2 } from '@lucide/svelte';
	import { onMount } from 'svelte';

	let selectedTheme = $state<'light' | 'dark' | 'system'>('system');

	onMount(() => {
		const unsub = globalSettings.subscribe((s) => {
			selectedTheme = s.theme;
		});
		return unsub;
	});

	function selectTheme(id: 'light' | 'dark' | 'system') {
		selectedTheme = id;
		// 即时应用并保存
		globalSettings.patch({ theme: id });
		if (id === 'dark') {
			document.documentElement.classList.add('dark');
		} else if (id === 'light') {
			document.documentElement.classList.remove('dark');
		} else {
			document.documentElement.classList.toggle(
				'dark',
				window.matchMedia('(prefers-color-scheme: dark)').matches
			);
		}
	}

	function getThemeIcon(theme: ThemeOption) {
		switch (theme.icon) {
			case 'sun': return SunIcon;
			case 'moon': return Moon;
			case 'settings': return Settings2;
			default: return SunIcon;
		}
	}
</script>

<div class="step-card-container" >
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
						onclick={() => selectTheme(theme.id)}
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

<style>
	.step-card-container { width: 100%; }
	.step-card {
		border: 1px solid var(--border);
		box-shadow: 0 4px 24px rgba(0, 0, 0, 0.06);
	}
	.step-title { font-size: 1.25rem; text-align: center; }
	.step-desc { text-align: center; line-height: 1.5; }

	.theme-list { display: flex; flex-direction: column; gap: 0.75rem; }

	.theme-item {
		display: flex; align-items: center; gap: 1rem; padding: 1rem;
		border: 1.5px solid var(--border); border-radius: 0.75rem;
		background: var(--background); cursor: pointer; transition: all 0.2s;
		text-align: left; width: 100%;
	}
	.theme-item:hover { border-color: var(--ring); background: var(--accent); }
	.theme-item.selected { border-color: var(--primary); background: var(--primary) / 0.05; }

	.theme-preview {
		width: 2.75rem; height: 2.75rem; border-radius: 0.75rem;
		display: flex; align-items: center; justify-content: center; flex-shrink: 0;
	}
	.theme-preview.theme-light { background: #fef3c7; color: #f59e0b; }
	.theme-preview.theme-dark { background: #1e293b; color: #6366f1; }
	.theme-preview.theme-system { background: linear-gradient(135deg, #fef3c7 50%, #1e293b 50%); color: #8b5cf6; }

	.theme-info { flex: 1; display: flex; flex-direction: column; gap: 0.125rem; }
	.theme-name { font-weight: 600; font-size: 0.9375rem; color: var(--foreground); }
	.theme-desc { font-size: 0.8125rem; color: var(--muted-foreground); }

	.theme-radio {
		width: 1.25rem; height: 1.25rem; border-radius: 50%;
		border: 2px solid var(--border);
		display: flex; align-items: center; justify-content: center;
		flex-shrink: 0; transition: all 0.2s;
	}
	.theme-radio.checked { border-color: var(--primary); }
	.radio-dot {
		width: 0.625rem; height: 0.625rem; border-radius: 50%;
		background: var(--primary);
	}
</style>
