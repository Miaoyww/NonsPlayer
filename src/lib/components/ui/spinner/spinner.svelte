<script lang="ts">
	import { cn } from "$lib/utils.js";
	import { Loader } from "@lucide/svelte";

	let {
		class: className,
		role = "status",
		"aria-label": ariaLabel = "Loading",
		...restProps
	}: { class?: string; role?: string; "aria-label"?: string; [key: string]: unknown } = $props();
</script>

<Loader {role} aria-label={ariaLabel} class={cn("size-4 animate-spin", className)} {...restProps} />
