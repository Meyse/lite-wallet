<script lang="ts">
  import type { RequestFact } from './types';

  let { rows }: { rows: RequestFact[] } = $props();
</script>

<dl class="flex flex-col rounded-xl bg-muted/45 px-3.5 py-0.5 text-sm dark:bg-muted/30">
  {#each rows as row (row.label)}
    <div
      class="flex items-start justify-between gap-4 border-b border-border/70 py-2.5 last:border-b-0"
    >
      <dt class="shrink-0 text-muted-foreground">{row.label}</dt>
      <dd class="min-w-0 text-right font-medium text-foreground tabular-nums">
        {#if row.content}{@render row.content()}{:else}{row.value}{/if}
      </dd>
    </div>
  {/each}
</dl>
