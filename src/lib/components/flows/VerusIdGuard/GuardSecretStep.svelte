<script lang="ts">
  import * as Tabs from '$lib/components/ui/tabs';
  import SeedPhraseStep from '$lib/components/flows/WalletImport/SeedPhraseStep.svelte';
  import TextImportStep from '$lib/components/flows/WalletImport/TextImportStep.svelte';
  import { i18nStore } from '$lib/i18n';
  import type { GuardFlowMode, GuardSecretInputMode } from './types';

  type GuardSecretStepProps = {
    mode: GuardFlowMode;
    secretMode: GuardSecretInputMode;
    seedPhraseInput: string;
    textImportInput: string;
    busy?: boolean;
     
    onSecretModeChange?: (value: GuardSecretInputMode) => void;
     
    onSeedPhraseInputChange?: (value: string) => void;
     
    onSeedPhraseNormalizedChange?: (value: string) => void;
     
    onSeedPhraseValidityChange?: (valid: boolean) => void;
     
    onTextImportInputChange?: (value: string) => void;
     
    onTextImportValidityChange?: (valid: boolean) => void;
  };

  const defaultStringHandler = (value: string) => {
    void value;
  };
  const defaultBooleanHandler = (valid: boolean) => {
    void valid;
  };
  const defaultModeHandler = (value: GuardSecretInputMode) => {
    void value;
  };

   
  let {
    mode,
    secretMode,
    seedPhraseInput,
    textImportInput,
    busy = false,
    onSecretModeChange = defaultModeHandler,
    onSeedPhraseInputChange = defaultStringHandler,
    onSeedPhraseNormalizedChange = defaultStringHandler,
    onSeedPhraseValidityChange = defaultBooleanHandler,
    onTextImportInputChange = defaultStringHandler,
    onTextImportValidityChange = defaultBooleanHandler
  }: GuardSecretStepProps = $props();
   

  const i18n = $derived($i18nStore);
  const actionLabel = $derived(
    mode === 'revoke' ? i18n.t('guard.mode.revokeLower') : i18n.t('guard.mode.recoverLower')
  );
  const seedEntryMode = $derived(secretMode === 'typeOneByOne' ? 'manual' : 'paste');
</script>

<div class="mx-auto w-full max-w-[640px] space-y-7 py-4">
  <div class="space-y-3 text-center">
    <h1 class="text-foreground text-2xl font-semibold tracking-tight leading-tight">
      {i18n.t('guard.flow.secret.title', { action: actionLabel })}
    </h1>
    <p class="text-muted-foreground text-sm">
      {i18n.t('guard.flow.secret.description', { action: actionLabel })}
    </p>
  </div>

  <Tabs.Root value={secretMode}>
    <div class="flex justify-center">
      <Tabs.List>
        <Tabs.Trigger value="pastePhrase" disabled={busy} onclick={() => onSecretModeChange('pastePhrase')}>
          {i18n.t('guard.flow.secret.modePastePhrase')}
        </Tabs.Trigger>
        <Tabs.Trigger value="typeOneByOne" disabled={busy} onclick={() => onSecretModeChange('typeOneByOne')}>
          {i18n.t('guard.flow.secret.modeTypeOneByOne')}
        </Tabs.Trigger>
        <Tabs.Trigger value="textAuto" disabled={busy} onclick={() => onSecretModeChange('textAuto')}>
          {i18n.t('guard.flow.secret.modeWifPrivateSeed')}
        </Tabs.Trigger>
      </Tabs.List>
    </div>

    <div class="pt-5">
      {#if secretMode === 'textAuto'}
        <TextImportStep
          importTextInput={textImportInput}
          onInputChanged={onTextImportInputChange}
          onValidityChanged={onTextImportValidityChange}
          placeholderKey="guard.flow.secret.textPlaceholder"
          helperKey="guard.flow.secret.textHelp"
        />
      {:else}
        <SeedPhraseStep
          seedPhraseInput={seedPhraseInput}
          entryMode={seedEntryMode}
          showEntryModeTabs={false}
          onInputChanged={onSeedPhraseInputChange}
          onNormalizedChanged={onSeedPhraseNormalizedChange}
          onValidityChanged={onSeedPhraseValidityChange}
        />
      {/if}
    </div>
  </Tabs.Root>
</div>
