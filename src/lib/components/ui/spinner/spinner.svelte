<script lang="ts">
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import type { ComponentProps } from 'svelte';
  import { cn } from '$lib/utils.js';

  type Props = ComponentProps<typeof LoaderCircleIcon>;

  // Adjacent localized text (or the parent control) owns the announcement.
  // For a spinner-only status, pass a localized aria-label or aria-labelledby.
  let {
    class: className,
    'aria-label': label,
    'aria-labelledby': labelledBy,
    ...restProps
  }: Props = $props();
</script>

<LoaderCircleIcon
  role={label || labelledBy ? 'status' : undefined}
  aria-label={label}
  aria-labelledby={labelledBy}
  aria-hidden={label || labelledBy ? undefined : true}
  class={cn('size-4 animate-spin', className)}
  {...restProps}
/>
