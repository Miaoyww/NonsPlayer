<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { cn } from '$lib/utils';
	export let title = '';
	export let description = '';

	const id = Math.trunc(Math.random() * Number.MAX_SAFE_INTEGER).toString() + title;

	let className = '';

	export { className as class };
</script>

<div
	class={cn(
		'flex flex-col justify-between gap-3 rounded-lg px-6 py-4 md:flex-row md:items-center blur-setting-card',
		className
	)}
>
	<Label for={id} class="space-1 block grow leading-[unset]">
		<div class="font-bold">{title}</div>
		<div class="block text-xs whitespace-pre-wrap text-muted-foreground">{description}</div>
	</Label>
	<slot {id} />
</div>
