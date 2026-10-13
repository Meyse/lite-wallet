<!-- 
  Component: NameStep
  Purpose: Content-only for wallet naming step (right side content).
  Last Updated: Explicitly typed picker items in template.
  Security: No sensitive data - only wallet customization.
-->

<script lang="ts">
  import type { WalletNetwork } from '$lib/types/wallet';
  import { i18nStore } from '$lib/i18n';
  import { Input } from '$lib/components/ui/input';
  import { getWalletColor, WALLET_COLOR_OPTIONS } from '$lib/constants/walletColors';
  import { cn } from '$lib/utils.js';

  type WalletData = {
    name: string;
    emoji: string;
    color: string;
    password: string;
    network: WalletNetwork;
  };

  // Props
  let {
    walletData = { name: '', emoji: '💰', color: 'blue', password: '', network: 'mainnet' },
    onUpdate = (_data: Partial<WalletData>) => {},
    errorMessage = ''
  }: {
    walletData: WalletData;
    onUpdate: (data: Partial<WalletData>) => void;
    errorMessage: string;
  } = $props();

  const i18n = $derived($i18nStore);

  // Local state
  let showEmojiPicker = $state(false);
  let showColorPicker = $state(false);

  // Local validation
  const hasInvalidChars = $derived(/[/\\:*?"<>|]/.test(walletData.name));

  // Better emoji options (money, crypto, identity themed)
  const emojiOptions: string[] = ['💰', '💎', '🪙', '🔐', '⚡', '🔥', '🚀', '🌟', '🛡️', '🔑', '👑', '⭐'];

  const selectedColor = $derived(getWalletColor(walletData.color));
</script>

<!-- Content centered in available space -->
<div class="flex min-h-full flex-col items-center justify-center py-4">
  <div class="relative w-full max-w-lg space-y-6 text-center">
    <!-- Main Icon with Controls -->
    <div class="relative">
      <!-- Main Icon Display -->
      <div
        class="mx-auto flex h-28 w-28 items-center justify-center rounded-3xl border-4 border-black/10 shadow-xl dark:border-white/20"
        style={`background-color: ${selectedColor.hex};`}
      >
        <span class="text-6xl" role="img">{walletData.emoji}</span>
      </div>

      <!-- Small Picker Controls -->
      <div class="absolute left-1/2 top-1/2 ml-20 flex -translate-y-1/2 flex-col gap-2">
        <!-- Emoji Picker -->
        <button
          onclick={() => {
            showEmojiPicker = !showEmojiPicker;
            showColorPicker = false;
          }}
          class="w-8 h-8 rounded-full bg-background border-2 border-border hover:border-primary hover:scale-105 transition-all flex items-center justify-center shadow-md"
          title={i18n.t('walletCreation.name.changeEmoji')}
          aria-label={i18n.t('walletCreation.name.changeEmoji')}
        >
          <span class="text-sm" role="img">{walletData.emoji}</span>
        </button>

        <!-- Color Picker -->
        <button
          onclick={() => {
            showColorPicker = !showColorPicker;
            showEmojiPicker = false;
          }}
          class="w-8 h-8 rounded-full bg-background border-2 border-border hover:border-primary hover:scale-105 transition-all flex items-center justify-center shadow-md"
          title={i18n.t('walletCreation.name.changeColor')}
          aria-label={i18n.t('walletCreation.name.changeColor')}
        >
          <div
            class="h-4 w-4 rounded-full border border-black/10 dark:border-white/25"
            style={`background-color: ${selectedColor.hex};`}
          ></div>
        </button>
      </div>
    </div>

    <!-- Name Input -->
    <div class="mx-auto w-full max-w-[420px]">
      <Input
        type="text"
        value={walletData.name}
        oninput={(e) => onUpdate({ name: (e.target as HTMLInputElement).value })}
        placeholder={i18n.t('walletCreation.name.placeholder')}
        variant="lg"
        class={cn('w-full text-center', hasInvalidChars && 'text-destructive')}
        autocomplete="off"
        spellcheck="false"
        autocorrect="off"
        autocapitalize="off"
      />
    </div>

    <!-- Validation Messages -->
    {#if hasInvalidChars}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-xs text-destructive">{i18n.t('walletCreation.name.invalidChars')}</p>
      </div>
    {:else if walletData.name.length > 16}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-xs text-destructive">{i18n.t('walletCreation.name.maxLength')}</p>
      </div>
    {/if}

    <!-- Error Message -->
    {#if errorMessage}
      <div class="bg-destructive/10 border border-destructive/20 rounded-lg p-3">
        <p class="text-destructive text-xs">{errorMessage}</p>
      </div>
    {/if}
  </div>
</div>

<!-- Fixed Overlay Pickers -->
{#if showEmojiPicker}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/20"
    role="button"
    tabindex="0"
    aria-label={i18n.t('walletCreation.name.closeEmojiPicker')}
    onclick={(event) => {
      if (event.currentTarget === event.target) {
        showEmojiPicker = false;
      }
    }}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        showEmojiPicker = false;
      }
      if (event.key === 'Escape') {
        showEmojiPicker = false;
      }
    }}
  >
    <div
      class="bg-background border border-border rounded-2xl p-4 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label={i18n.t('walletCreation.name.emojiPicker')}
    >
      <div class="grid grid-cols-6 gap-3">
        {#each emojiOptions as emoji}
          {@const emojiValue = emoji as string}
          <button
            onclick={() => {
              onUpdate({ emoji: emojiValue });
              showEmojiPicker = false;
            }}
            class="w-14 h-14 rounded-xl hover:bg-muted transition-colors flex items-center justify-center
                   {emojiValue === walletData.emoji ? 'bg-primary/10 ring-2 ring-primary' : ''}"
          >
            <span class="text-2xl" role="img">{emojiValue}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

{#if showColorPicker}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/20"
    role="button"
    tabindex="0"
    aria-label={i18n.t('walletCreation.name.closeColorPicker')}
    onclick={(event) => {
      if (event.currentTarget === event.target) {
        showColorPicker = false;
      }
    }}
    onkeydown={(event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        showColorPicker = false;
      }
      if (event.key === 'Escape') {
        showColorPicker = false;
      }
    }}
  >
    <div
      class="bg-background border border-border rounded-2xl p-4 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label={i18n.t('walletCreation.name.colorPicker')}
    >
      <div class="grid grid-cols-7 gap-2">
        {#each WALLET_COLOR_OPTIONS as colorOption}
          <button
            onclick={() => {
              onUpdate({ color: colorOption.name });
              showColorPicker = false;
            }}
            class={cn(
              'h-8 w-8 rounded-full transition-all duration-200 hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background',
              colorOption.name === selectedColor.name
                ? 'scale-110 ring-2 ring-slate-900/70 ring-offset-2 ring-offset-background dark:ring-slate-100/90'
                : 'ring-1 ring-black/10 dark:ring-white/25'
            )}
            title={colorOption.name}
            aria-label={i18n.t('walletCreation.name.selectColor', { color: colorOption.name })}
          >
            <div
              class="h-full w-full rounded-full border border-black/10 shadow-lg dark:border-white/25"
              style={`background-color: ${colorOption.hex};`}
            ></div>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}
