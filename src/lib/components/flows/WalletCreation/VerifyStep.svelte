<!-- 
  Component: VerifyStep
  Purpose: Complete verify step with 2-column layout + bottom action
  Last Updated: Exposes verifyWords() and field completion tracking for parent component
  Security: Validates backup without storing sensitive data locally
-->

<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';

  const i18n = $derived($i18nStore);

  // Props
  let {
    seedPhrase = '',
    verificationIndices = [],
    onSetupVerification = (indices: number[]) => {},
    onFieldsChanged = (filled: boolean) => {}
  } = $props();

  // Local verification state
  let verificationWords = $state(['', '', '']);
  let verificationErrors = $state([false, false, false]);
  let hasAttempted = $state(false);

  // Auto-setup verification indices if not provided
  $effect(() => {
    if (verificationIndices.length === 0 && seedPhrase) {
      const indices: number[] = [];
      while (indices.length < 3) {
        const randomIndex = Math.floor(Math.random() * 24);
        if (!indices.includes(randomIndex)) {
          indices.push(randomIndex);
        }
      }
      onSetupVerification(indices.sort((a, b) => a - b));
    }
  });

  // Get correct words for verification
  const seedWords = $derived(seedPhrase.split(' '));
  const correctWords = $derived(verificationIndices.map((index) => seedWords[index] || ''));

  // Watch for field changes and notify parent
  $effect(() => {
    const filled = verificationWords.every((word) => word.trim() !== '');
    onFieldsChanged(filled);
  });

  function handleWordInput(index: number, event: Event) {
    const target = event.target as HTMLInputElement;
    verificationWords[index] = target.value;

    // Clear error when user types
    if (hasAttempted) {
      verificationErrors[index] = false;
    }
  }

  // Expose verifyWords function - returns boolean for success/failure
  export function verifyWords(): boolean {
    hasAttempted = true;
    let hasErrors = false;

    verificationIndices.forEach((wordIndex, i) => {
      const userWord = verificationWords[i].toLowerCase().trim();
      const correctWord = correctWords[i].toLowerCase();

      if (userWord !== correctWord) {
        verificationErrors[i] = true;
        hasErrors = true;
      } else {
        verificationErrors[i] = false;
      }
    });

    if (!hasErrors) {
      console.info('[WALLET] Seed verification successful');
      return true;
    } else {
      console.info('[WALLET] Seed verification failed');
      return false;
    }
  }
</script>

<!-- Content only for verify step -->
<div class="mx-auto w-full max-w-[560px] space-y-3">
  {#each verificationIndices as wordIndex, i}
    <div class="space-y-1.5">
      <div class="grid grid-cols-[116px_1fr] items-center gap-3">
        <Label
          for="word-{i}"
          class="text-foreground flex items-center gap-2 text-sm font-medium"
        >
          <span
            class="inline-flex h-6 min-w-6 items-center justify-center rounded-md border border-border bg-muted px-1.5 text-xs font-semibold"
          >
            {wordIndex + 1}
          </span>
          <span>{i18n.t('walletCreation.verify.word', { index: wordIndex + 1 })}</span>
        </Label>

        <Input
          id="word-{i}"
          value={verificationWords[i]}
          oninput={(e) => handleWordInput(i, e)}
          placeholder={i18n.t('walletCreation.verify.enterWord')}
          autocomplete="off"
          aria-invalid={verificationErrors[i]}
        />
      </div>

      <p class="min-h-5 pl-[119px] text-xs text-destructive">
        {#if verificationErrors[i]}
          {i18n.t('walletCreation.verify.incorrect')}
        {/if}
      </p>
    </div>
  {/each}
</div>
