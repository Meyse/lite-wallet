<script lang="ts">
  import { RadioGroup as RadioGroupPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children: childrenSnippet,
    ...restProps
  }: RadioGroupPrimitive.ItemProps = $props();
</script>

<RadioGroupPrimitive.Item
  bind:ref
  data-slot="radio-group-item"
  class={cn(
    'group flex w-full items-center justify-between gap-4 rounded-xl border border-border/70 bg-muted/25 px-4 py-3 text-left transition-colors outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring/60 disabled:pointer-events-none disabled:opacity-50 data-[state=checked]:border-primary data-[state=checked]:bg-primary/8 dark:data-[state=checked]:bg-primary/14',
    className
  )}
  {...restProps}
>
  {#snippet children({ checked })}
    {@render childrenSnippet?.({ checked })}
    <span
      class="size-4 shrink-0 rounded-full border-2 border-muted-foreground/40 group-data-[state=checked]:border-[5px] group-data-[state=checked]:border-primary"
      aria-hidden="true"
    ></span>
  {/snippet}
</RadioGroupPrimitive.Item>
