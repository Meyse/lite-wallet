<script lang="ts">
  import { cn } from '$lib/utils.js';
  import {
    formatIdentifierForDisplay,
    type IdentifierDisplayMode,
  } from '$lib/utils/identifierDisplay.js';

  type IdentifierTextProps = {
    value: string;
    mode?: IdentifierDisplayMode;
    class?: string;
    title?: string;
  };

  let { value, mode = 'full', class: className = '', title }: IdentifierTextProps = $props();

  const normalizedValue = $derived(value.trim());
  const displayValue = $derived(formatIdentifierForDisplay(value, mode));
  const effectiveTitle = $derived(
    title ?? (displayValue !== normalizedValue ? normalizedValue : undefined)
  );
</script>

<span
  class={cn('identifier-text', mode === 'full' ? 'break-all' : '', className)}
  title={effectiveTitle}
>
  {displayValue}
</span>
