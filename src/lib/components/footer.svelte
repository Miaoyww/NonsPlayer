<script lang="ts">
	import { isTauri } from '@tauri-apps/api/core';
</script>

<footer class="w-full py-6 text-center text-xs text-stone-400 dark:text-stone-500">
	<p>
		Copyright © <a
			href="https://github.com/Miaoyww"
			target="_blank"
			rel="noopener noreferrer"
			class="transition-colors hover:text-stone-600 dark:hover:text-stone-300">Miaoyww</a
		> 2025–2026
	</p>
	<p class="mt-1">
		Powered by
		<a
			href="https://svelte.dev"
			target="_blank"
			rel="noopener noreferrer"
			class="transition-colors hover:text-stone-600 dark:hover:text-stone-300">Svelte</a
		>
		·
		{#if isTauri()}
			<a
				href="https://tauri.app"
				target="_blank"
				rel="noopener noreferrer"
				class="transition-colors hover:text-stone-600 dark:hover:text-stone-300">Tauri</a
			>
		{:else}
			<a
				href="https://vercel.com"
				target="_blank"
				rel="noopener noreferrer"
				class="transition-colors hover:text-stone-600 dark:hover:text-stone-300">Vercel</a
			>
		{/if}
	</p>
</footer>
	