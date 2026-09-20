<script lang="ts">
  import { tick } from 'svelte';
  import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import * as Popover from '$lib/components/ui/popover';
  import { i18nStore } from '$lib/i18n';
  import { openTrustedExternalUrl } from '$lib/utils/externalLinks';
  import { profileExternalUrl, type ProfileSocial } from './publicProfileContent';

  let { social }: { social: ProfileSocial } = $props();
  const i18n = $derived($i18nStore);
  const name = $derived(social.platform === 'linkedin' ? 'LinkedIn' : 'X');
  const profileUrl = $derived(profileExternalUrl(social.profileUrl));
  const proofUrl = $derived(profileExternalUrl(social.proofUrl));
  let open = $state(false);
  let trigger = $state<HTMLButtonElement | null>(null);
  let contentElement = $state<HTMLDivElement | null>(null);
  let suppressed = false;
  function focusOut(event: FocusEvent) {
    const next = event.relatedTarget;
    if (next instanceof Node && (trigger?.contains(next) || contentElement?.contains(next))) return;
    suppressed = false;
    open = false;
  }
  function changeOpen(next: boolean) {
    open = next && !suppressed;
  }
  $effect(() => {
    if (!open) return undefined;
    const dismiss = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopImmediatePropagation();
      open = false;
      suppressed = true;
      trigger?.focus();
    };
    window.addEventListener('keydown', dismiss, true);
    return () => window.removeEventListener('keydown', dismiss, true);
  });
</script>

{#if profileUrl}
  <Popover.Root {open} onOpenChange={changeOpen}>
    <Popover.Trigger
      bind:ref={trigger}
      openOnHover
      openDelay={100}
      closeDelay={200}
      class="inline-flex h-8 items-center gap-2 rounded-sm text-[13px] outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
      onpointerenter={() => (suppressed = false)}
      onpointerdown={() => (suppressed = false)}
      onfocus={() => {
        if (!suppressed && trigger?.matches(':focus-visible')) open = true;
      }}
      onblur={focusOut}
      onkeydown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') suppressed = false;
        if (event.key === 'Tab' && !event.shiftKey && open) {
          event.preventDefault();
          void tick().then(() => contentElement?.querySelector<HTMLAnchorElement>('a')?.focus());
        }
      }}
      aria-label={i18n.t('wallet.identity.publicProfile.socialAccount', { platform: name })}
    >
      <img
        src={`/brands/${social.platform}.png`}
        alt=""
        class="size-[18px] object-contain dark:hidden"
      />
      <img
        src={`/brands/${social.platform}-white.png`}
        alt=""
        class="hidden size-[18px] object-contain dark:block"
      />
      <span>{name}</span>
      {#if social.verification === 'verified'}<CircleCheckIcon
          class="size-3.5 text-contact-saved-foreground"
          aria-label={i18n.t('wallet.identity.publicProfile.verified')}
        />{/if}
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content
        bind:ref={contentElement}
        onfocusout={focusOut}
        onkeydown={(event) => {
          if (event.key !== 'Tab') return;
          const links = contentElement?.querySelectorAll('a');
          if (!links?.length) return;
          if (
            (event.shiftKey && event.target === links[0]) ||
            (!event.shiftKey && event.target === links[links.length - 1])
          ) {
            suppressed = true;
            trigger?.focus();
            open = false;
            // Resume the native document order at the trigger, outside the portal.
            if (event.shiftKey) event.preventDefault();
          }
        }}
        sideOffset={6}
        align="start"
        collisionPadding={12}
        trapFocus={false}
        class="z-50 w-[216px] rounded-xl border bg-popover p-3 text-popover-foreground shadow-md outline-none"
        aria-label={name}
        onOpenAutoFocus={(event) => event.preventDefault()}
        onCloseAutoFocus={(event) => event.preventDefault()}
      >
        <div class="mb-2 flex items-center justify-between gap-2 text-[13px]">
          <span class="font-medium">{name}</span>
          <span
            class={social.verification === 'verified'
              ? 'text-contact-saved-foreground'
              : 'text-muted-foreground'}
            >{i18n.t(`wallet.identity.publicProfile.${social.verification}`)}</span
          >
        </div>
        <a
          href={profileUrl}
          target="_blank"
          rel="noopener noreferrer"
          class="flex min-h-8 items-center justify-between gap-2 rounded-md px-1 text-[13px] hover:bg-muted focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
          onclick={(event) => {
            event.preventDefault();
            void openTrustedExternalUrl(profileUrl);
          }}
        >
          {i18n.t('wallet.identity.publicProfile.visitSocial')}<ExternalLinkIcon
            class="size-3.5"
            aria-hidden="true"
          />
        </a>
        {#if proofUrl}
          <a
            href={proofUrl}
            target="_blank"
            rel="noopener noreferrer"
            class="flex min-h-8 items-center justify-between gap-2 rounded-md px-1 text-[13px] hover:bg-muted focus-visible:ring-2 focus-visible:ring-settings-focus-ring"
            onclick={(event) => {
              event.preventDefault();
              void openTrustedExternalUrl(proofUrl);
            }}
          >
            {i18n.t('wallet.identity.publicProfile.visitProof')}<ExternalLinkIcon
              class="size-3.5"
              aria-hidden="true"
            />
          </a>
        {/if}
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
{/if}
