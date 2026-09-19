<script lang="ts">
  import type { ContactIdentity } from '$lib/types/addressBook';
  import { contactChainId } from '$lib/contacts/identity';
  import { contactSession } from '$lib/contacts/session';
  import { resolveContactIdentity } from '$lib/contacts/service';
  import IdentityMention from './IdentityMention.svelte';
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
  import InlineTextActionButton from '$lib/components/common/InlineTextActionButton.svelte';
  import { i18nStore } from '$lib/i18n';
  let {
    value,
    chainId,
    delay = 350,
    showStatus = false,
  }: { value: string; chainId: string; delay?: number; showStatus?: boolean } = $props();
  const i18n = $derived($i18nStore);
  let result = $state<{ key: string; identity: ContactIdentity } | null>(null);
  let progress = $state<{ key: string; status: 'loading' | 'error' } | null>(null);
  let retry = $state(0);
  const requestKey = $derived(
    JSON.stringify([$contactSession?.sessionId, $contactSession?.network, chainId, value.trim()])
  );
  const identity = $derived(result?.key === requestKey ? result.identity : null);
  $effect(() => {
    retry;
    const session = $contactSession;
    const key = requestKey;
    const address = value.trim();
    if (
      !session ||
      chainId !== contactChainId(session.network) ||
      !(/^i[1-9A-HJ-NP-Za-km-z]{24,60}$/.test(address) || /^[A-Za-z0-9._-]+@$/.test(address))
    )
      return undefined;
    let cancelled = false;
    const timer = setTimeout(() => {
      progress = { key, status: 'loading' };
      void resolveContactIdentity(address)
        .then((identity) => {
          if (!cancelled) {
            result = { key, identity };
            progress = null;
          }
        })
        .catch(() => {
          if (!cancelled) progress = { key, status: 'error' };
        });
    }, delay);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });
</script>

{#if identity}<IdentityMention {identity} />
{:else if showStatus && progress?.key === requestKey}
  {#if progress.status === 'loading'}
    <p class="flex items-center gap-2 text-xs text-settings-muted-foreground" role="status">
      <LoaderCircleIcon
        class="size-3.5 animate-spin motion-reduce:animate-none"
        aria-hidden="true"
      />
      {i18n.t('wallet.contacts.lookingUp')}
    </p>
  {:else}
    <div class="space-y-1 text-xs text-settings-muted-foreground">
      <p role="status">{i18n.t('wallet.contacts.lookupFailed')}</p>
      <InlineTextActionButton onclick={() => retry++}
        >{i18n.t('wallet.contacts.retry')}</InlineTextActionButton
      >
    </div>
  {/if}
{/if}
