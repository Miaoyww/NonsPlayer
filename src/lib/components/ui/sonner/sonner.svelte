<script lang="ts">
	import { Toaster as Sonner, type ToasterProps as SonnerProps } from "svelte-sonner";
	import { mode } from "mode-watcher";
	import { Loader, CircleCheck, CircleX, Info, TriangleAlert } from "@lucide/svelte";

	let { ...restProps }: SonnerProps = $props();
</script>

<Sonner
	theme={mode.current}
	class="toaster group"
	style="--normal-bg: var(--color-popover); --normal-text: var(--color-popover-foreground); --normal-border: var(--color-border);"
	{...restProps}
>
	{#snippet loadingIcon()}
		<Loader class="size-4 animate-spin" />
	{/snippet}
	{#snippet successIcon()}
		<CircleCheck class="size-4" />
	{/snippet}
	{#snippet errorIcon()}
		<CircleX class="size-4" />
	{/snippet}
	{#snippet infoIcon()}
		<Info class="size-4" />
	{/snippet}
	{#snippet warningIcon()}
		<TriangleAlert class="size-4" />
	{/snippet}
</Sonner>
