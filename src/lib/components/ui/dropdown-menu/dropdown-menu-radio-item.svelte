<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
	import CheckIcon from "@lucide/svelte/icons/check";
	import { cn, type WithoutChild } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		children: childrenProp,
		...restProps
	}: WithoutChild<DropdownMenuPrimitive.RadioItemProps> = $props();
</script>

<DropdownMenuPrimitive.RadioItem
	bind:ref
	data-slot="dropdown-menu-radio-item"
	class={cn(
		"data-[state=checked]:bg-settings-selection-surface focus:bg-settings-selection-surface data-[highlighted]:bg-settings-selection-surface relative flex h-[30px] cursor-default items-center gap-2 rounded-sm px-2 py-0 text-[13px] leading-[18px] font-normal outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
		className
	)}
	{...restProps}
>
	{#snippet children({ checked })}
		{@render childrenProp?.({ checked })}
		{#if checked}
			<CheckIcon class="ms-auto size-4 text-primary dark:text-settings-focus-ring" />
		{/if}
	{/snippet}
</DropdownMenuPrimitive.RadioItem>
