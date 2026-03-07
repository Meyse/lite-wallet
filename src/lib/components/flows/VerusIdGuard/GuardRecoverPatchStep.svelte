<script lang="ts">
  import * as Accordion from '$lib/components/ui/accordion';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';
  import type { GuardRecoverDraft } from './types';

  type GuardRecoverPatchStepProps = {
    draft: GuardRecoverDraft;
    busy?: boolean;
    onDraftChange?: (next: GuardRecoverDraft) => void;
  };

  const defaultHandler = (next: GuardRecoverDraft) => {
    void next;
  };

   
  let {
    draft,
    busy = false,
    onDraftChange = defaultHandler
  }: GuardRecoverPatchStepProps = $props();
   

  const i18n = $derived($i18nStore);

  function updateField(field: keyof GuardRecoverDraft, value: string) {
    onDraftChange({ ...draft, [field]: value });
  }
</script>

<div class="mx-auto w-full max-w-[560px] space-y-6 py-4">
  <div class="space-y-2 text-center">
    <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {i18n.t('guard.flow.patch.title')}
    </h1>
    <p class="text-muted-foreground text-sm">{i18n.t('guard.flow.patch.description')}</p>
  </div>

  <div class="space-y-5">
    <div class="space-y-2">
      <Label for="guard-primary-address">{i18n.t('guard.flow.patch.primaryAddressLabel')}</Label>
      <Input
        id="guard-primary-address"
        value={draft.primaryAddress}
        oninput={(event) => updateField('primaryAddress', (event.target as HTMLInputElement).value)}
        placeholder={i18n.t('guard.flow.target.primaryPlaceholder')}
        class="identifier-text"
        disabled={busy}
        autocapitalize="off"
        spellcheck="false"
      />
      <p class="text-muted-foreground text-xs">{i18n.t('guard.flow.target.primaryHelp')}</p>
    </div>

    <Accordion.Root type="single" class="w-full">
      <Accordion.Item value="advanced">
        <Accordion.Trigger>{i18n.t('guard.flow.patch.advancedTitle')}</Accordion.Trigger>
        <Accordion.Content>
          <div class="space-y-4 pt-2">
            <div class="space-y-2">
              <Label for="guard-recovery-authority">{i18n.t('guard.flow.patch.recoveryAuthorityLabel')}</Label>
              <Input
                id="guard-recovery-authority"
                value={draft.recoveryAuthority}
                oninput={(event) => updateField('recoveryAuthority', (event.target as HTMLInputElement).value)}
                placeholder={i18n.t('guard.flow.patch.recoveryAuthorityPlaceholder')}
                class="identifier-text"
                disabled={busy}
                autocapitalize="off"
                spellcheck="false"
              />
            </div>

            <div class="space-y-2">
              <Label for="guard-revocation-authority">{i18n.t('guard.flow.patch.revocationAuthorityLabel')}</Label>
              <Input
                id="guard-revocation-authority"
                value={draft.revocationAuthority}
                oninput={(event) => updateField('revocationAuthority', (event.target as HTMLInputElement).value)}
                placeholder={i18n.t('guard.flow.patch.revocationAuthorityPlaceholder')}
                class="identifier-text"
                disabled={busy}
                autocapitalize="off"
                spellcheck="false"
              />
            </div>

            <div class="space-y-2">
              <Label for="guard-private-address">{i18n.t('guard.flow.patch.privateAddressLabel')}</Label>
              <Input
                id="guard-private-address"
                value={draft.privateAddress}
                oninput={(event) => updateField('privateAddress', (event.target as HTMLInputElement).value)}
                placeholder={i18n.t('guard.flow.patch.privateAddressPlaceholder')}
                class="identifier-text"
                disabled={busy}
                autocapitalize="off"
                spellcheck="false"
              />
            </div>
          </div>
        </Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  </div>
</div>
