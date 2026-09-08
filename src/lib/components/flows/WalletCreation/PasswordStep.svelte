<!--
  Component: PasswordStep
  Purpose: Password validation input for wallet creation flow
  Last Updated: Removed duplicate create_wallet invoke path
  Security: Handles wallet password input only; parent component performs creation
-->

<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { i18nStore } from '$lib/i18n';
  import { getPasswordStrength, MIN_PASS_SCORE, scorePassword } from '$lib/utils/auth/scorePassword';

  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
    network: 'mainnet' | 'testnet';
  };

  type PasswordStepProps = {
    walletData?: WalletData;
    onUpdate?: any;
    onCanCreateChanged?: any;
  };

  let {
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' },
    onUpdate = () => {},
    onCanCreateChanged = () => {}
  }: PasswordStepProps = $props();

  const i18n = $derived($i18nStore);

  // Local state
  let confirmPassword = $state('');

  // Password strength tracking
  let passwordScore = $state(0);
  const passwordStrength = $derived(getPasswordStrength(passwordScore));
  const strengthBars = [0, 1, 2, 3, 4];
  const strengthLabel = $derived(i18n.t(`walletCreation.password.level.${passwordStrength.label}`));

  // Update score when password changes
  $effect(() => {
    if (!walletData.password) {
      passwordScore = 0;
    } else {
      passwordScore = scorePassword(walletData.password);
    }
  });

  // Password validation with strength requirements
  const passwordsMatch = $derived(walletData.password === confirmPassword && walletData.password !== '');
  const passwordValid = $derived(passwordScore >= MIN_PASS_SCORE);
  const showConfirmField = $derived(passwordValid || confirmPassword.length > 0);
  const canCreateWallet = $derived(passwordValid && passwordsMatch);

  // Notify parent component of validation state
  $effect(() => {
    onCanCreateChanged(canCreateWallet);
  });
</script>

<!-- Content only for password step -->
<div class="mx-auto w-full max-w-[360px] space-y-5">
  <!-- Password Input with Strength Indicator -->
  <div class="space-y-2">
    <Label for="wallet-password" class="text-card-foreground">
      {i18n.t('walletCreation.password.choose')}
    </Label>
    <Input
      id="wallet-password"
      type="password"
      value={walletData.password}
      oninput={(e) => onUpdate({ password: (e.target as HTMLInputElement).value })}
      placeholder={i18n.t('walletCreation.password.placeholder')}
      autocomplete="new-password"
      aria-invalid={Boolean(walletData.password) && !passwordValid}
    />

    <!-- Strength Bars (5 bars) -->
    <div class="space-y-1 min-h-9">
      {#if walletData.password}
        <div class="flex h-1.5 gap-1">
          {#each strengthBars as i}
            {@const isActive = i < passwordStrength.level}
            {@const barColor = isActive
              ? passwordStrength.color === 'destructive'
                ? 'bg-destructive'
                : passwordStrength.color === 'blue-500'
                  ? 'bg-blue-500'
                  : passwordStrength.color === 'primary'
                    ? 'bg-primary'
                    : passwordStrength.color === 'green-500'
                      ? 'bg-green-500 dark:bg-green-600'
                      : 'bg-muted'
              : 'bg-muted'}
            <div class="flex-1 rounded-full transition-colors {barColor}"></div>
          {/each}
        </div>

        {@const labelColor = passwordStrength.color === 'destructive'
          ? 'text-destructive'
          : passwordStrength.color === 'blue-500'
            ? 'text-blue-500'
            : passwordStrength.color === 'primary'
              ? 'text-primary'
              : passwordStrength.color === 'green-500'
                ? 'text-green-500 dark:text-green-400'
                : 'text-muted-foreground'}
        <p class="text-xs {labelColor}">
          {i18n.t('walletCreation.password.strength', { label: strengthLabel })}
        </p>
      {/if}
    </div>

  </div>

  {#if showConfirmField}
    <!-- Confirm Password -->
    <div class="space-y-2">
      <Label for="confirm-password" class="text-card-foreground">
        {i18n.t('walletCreation.password.confirm')}
      </Label>
      <Input
        id="confirm-password"
        type="password"
        value={confirmPassword}
        oninput={(e) => {
          confirmPassword = (e.target as HTMLInputElement).value;
        }}
        placeholder={i18n.t('walletCreation.password.confirmPlaceholder')}
        autocomplete="new-password"
        aria-invalid={Boolean(confirmPassword) && !passwordsMatch}
      />
      <p class="min-h-5 text-xs text-destructive">
        {#if confirmPassword && !passwordsMatch}
          {i18n.t('walletCreation.password.mismatch')}
        {/if}
      </p>
    </div>
  {/if}
</div>
