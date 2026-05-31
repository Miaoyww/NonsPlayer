<script lang="ts">
	import { Accordion as AccordionPrimitive } from 'bits-ui';
	import { cn, type WithoutChild } from '$lib/utils.js';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';

	let {
		ref = $bindable(null),
		class: className,
		level = 3,
		children,
		...restProps
	}: WithoutChild<AccordionPrimitive.TriggerProps> & {
		level?: AccordionPrimitive.HeaderProps['level'];
	} = $props();
</script>

<AccordionPrimitive.Header {level} class="flex">
	<AccordionPrimitive.Trigger
		data-slot="accordion-trigger"
		bind:ref
		class={cn(
			/* 移除了 ml-auto，保留其他样式 */
			'group/accordion-trigger relative flex flex-1 items-center rounded-lg border border-transparent py-2.5 text-left text-sm font-medium transition-all outline-none hover:underline focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:after:border-ring disabled:pointer-events-none disabled:opacity-50 **:data-[slot=accordion-trigger-icon]:size-4 **:data-[slot=accordion-trigger-icon]:text-muted-foreground',
			className
		)}
		{...restProps}
	>
		{@render children?.()}
		<ChevronDownIcon
			data-slot="accordion-trigger-icon"
			class="cn-accordion-trigger-icon pointer-events-none mr-2 shrink-0 group-aria-expanded/accordion-trigger:hidden"
		/>
		<ChevronUpIcon
			data-slot="accordion-trigger-icon"
			class="cn-accordion-trigger-icon pointer-events-none mr-2 hidden shrink-0 group-aria-expanded/accordion-trigger:inline"
		/>
	</AccordionPrimitive.Trigger>
</AccordionPrimitive.Header>
