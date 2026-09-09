<script lang="ts">
  import { i18nStore } from '$lib/i18n';
  import type { LinkedIdentity } from '$lib/types/wallet.js';
  import { formatIdentityDisplayName } from '$lib/utils/identityDisplay';

  let { identity }: { identity: LinkedIdentity | null } = $props();
  const i18n = $derived($i18nStore);
</script>

<!-- Match the detail view's header, card, and row geometry before data arrives. -->
<div class="mx-auto flex w-full max-w-4xl min-w-0 flex-col gap-5 px-6 pt-3 pb-6" aria-busy="true">
  <p class="sr-only" role="status">{i18n.t('wallet.identity.detail.loading')}</p>
  <div class="flex h-8 items-center justify-between gap-3" aria-hidden="true">
    <div class="placeholder h-4 w-32 rounded bg-muted/50"></div>
    <div class="placeholder h-8 w-24 rounded-md bg-muted/50"></div>
  </div>

  <div class="rounded-xl bg-muted/20 p-4">
    <p class="text-sm font-semibold text-foreground">
      {identity ? formatIdentityDisplayName(identity) : '\u00a0'}
    </p>
    <p class="mt-1 font-mono text-xs text-muted-foreground">
      {identity?.identityAddress ?? '\u00a0'}
    </p>
  </div>

  {#each [{ title: 'wallet.identity.detail.sections.base', fields: ['name', 'iAddress', 'status', 'system'] }, { title: 'wallet.identity.detail.sections.authorities', fields: ['revocationAuthority', 'recoveryAuthority'] }] as section}
    <section class="space-y-2 rounded-xl bg-muted/20 p-4" aria-hidden="true">
      <h3 class="text-sm font-semibold text-foreground">{i18n.t(section.title)}</h3>
      <div class="space-y-2 text-sm">
        {#each section.fields as field}
          <div
            class="flex items-center justify-between gap-3 rounded-md bg-background/55 px-3 py-2 dark:bg-background/40"
          >
            <div class="min-w-0">
              <p class="text-xs text-muted-foreground">
                {i18n.t(`wallet.identity.detail.fields.${field}`)}
              </p>
              <div class="flex h-5 items-center">
                <div class="placeholder h-3 w-32 rounded bg-muted/50"></div>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  /* Keep quick loads quiet without a shimmer or a flashing loading sentence. */
  .placeholder {
    animation: reveal-placeholder 0s 240ms both;
  }

  @keyframes reveal-placeholder {
    from {
      visibility: hidden;
    }
    to {
      visibility: visible;
    }
  }
</style>
