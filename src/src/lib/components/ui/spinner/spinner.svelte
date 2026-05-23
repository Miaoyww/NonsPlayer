<script lang="ts">
	import { cn } from "$lib/utils.js";
	import { HugeiconsIcon } from "@hugeicons/svelte"
	import { Loading03Icon } from '@hugeicons/core-free-icons';
	import type { SVGAttributes } from "svelte/elements";

	let {
		class: className,
		role = "status",
		// we add name, color, and stroke for compatibility with different icon libraries props
		name,
		color,
		stroke,
		"aria-label": ariaLabel = "Loading",
		...restProps
	}: SVGAttributes<SVGSVGElement> = $props();
</script>

<HugeiconsIcon icon={Loading03Icon} strokeWidth={2} {role} name={name === null ? undefined : name} color={color === null ? undefined : color} stroke={stroke === null ? undefined : stroke} aria-label={ariaLabel} class={cn("size-4 animate-spin", className)} {...restProps} />
