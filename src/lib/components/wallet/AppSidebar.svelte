<!-- 
  Component: AppSidebar
  Purpose: Left sidebar navigation for wallet sections
  Last Updated: Sidebar redesign with flattened navigation
  Security: No sensitive operations - navigation only
-->

<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar';
  import WalletIcon from '@lucide/svelte/icons/wallet';
  import AppWindowIcon from '@lucide/svelte/icons/app-window';
  import ActivityIcon from '@lucide/svelte/icons/activity';
  import BookUserIcon from '@lucide/svelte/icons/book-user';
  import Link2Icon from '@lucide/svelte/icons/link-2';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import LockIcon from '@lucide/svelte/icons/lock';
  import VerusIdAtIcon from '$lib/components/icons/VerusIdAtIcon.svelte';
  import { i18nStore } from '$lib/i18n';
  import { getWalletColorHex } from '$lib/constants/walletColors';
  import { forceWalletToUnlock } from '$lib/services/walletLockCoordinator.js';

  type SectionId =
    | 'overview'
    | 'send'
    | 'receive'
    | 'conversions'
    | 'identity'
    | 'address-book'
    | 'apps'
    | 'activity'
    | 'settings';

  interface WalletData {
    name: string;
    emoji: string;
    color: string;
    network?: 'mainnet' | 'testnet';
  }

  interface MenuItem {
    id: SectionId;
    title: string;
    icon: typeof WalletIcon;
  }

  // `activeSection` is bindable, so this props object must be mutable.
  let {
    activeSection = $bindable('overview' as SectionId),
    walletData,
    onSelectOverview = () => {},
    onSelectSettings = () => {},
    onOpenRequest = () => {}
  }: {
    activeSection?: SectionId;
    walletData: WalletData;
    onSelectOverview?: () => void;
    onSelectSettings?: () => void;
    onOpenRequest?: () => void;
  } = $props();

  const i18n = $derived($i18nStore);
  const colorHex = $derived(getWalletColorHex(walletData.color));

  const menuItems = $derived<MenuItem[]>([
    { id: 'overview', title: i18n.t('wallet.sidebar.wallet'), icon: WalletIcon },
    { id: 'identity', title: i18n.t('wallet.sidebar.identities'), icon: VerusIdAtIcon },
    { id: 'apps', title: i18n.t('wallet.sidebar.apps'), icon: AppWindowIcon },
    { id: 'activity', title: i18n.t('wallet.sidebar.activity'), icon: ActivityIcon },
    { id: 'address-book', title: i18n.t('wallet.sidebar.addressBook'), icon: BookUserIcon }
  ]);
  const menuButtonClass =
    'h-8 rounded-md px-2 text-[13px] dark:text-[14px] hover:bg-sidebar-item-hover hover:text-sidebar-accent-foreground data-[state=open]:hover:bg-sidebar-item-hover active:bg-sidebar-item-pressed active:text-sidebar-accent-foreground data-[active=true]:bg-sidebar-item-active data-[active=true]:text-sidebar-accent-foreground data-[active=true]:hover:bg-sidebar-item-active data-[state=open]:hover:text-sidebar-accent-foreground';
  const footerButtonClass =
    'text-sidebar-foreground/65 ring-sidebar-ring cursor-pointer hover:bg-sidebar-item-hover hover:text-sidebar-accent-foreground active:bg-sidebar-item-pressed active:text-sidebar-accent-foreground focus-visible:ring-2 flex h-8 w-full items-center gap-2 rounded-md px-2 text-left text-[13px] dark:text-[14px] font-normal outline-hidden transition-colors';

  function isMenuItemActive(itemId: MenuItem['id']): boolean {
    if (itemId === 'overview') {
      return (
        activeSection === 'overview' ||
        activeSection === 'send' ||
        activeSection === 'receive' ||
        activeSection === 'conversions'
      );
    }

    return activeSection === itemId;
  }

  async function handleLock() {
    try {
      await forceWalletToUnlock();
    } catch {
      console.error('[WALLET] Lock failed');
    }
  }

  function handleMenuClick(itemId: MenuItem['id']): void {
    if (itemId === 'overview') {
      onSelectOverview();
    }
    activeSection = itemId;
  }

  function handleSettingsClick(): void {
    onSelectSettings();
    activeSection = 'settings';
  }
</script>

<Sidebar.Root class="[--sidebar:var(--sidebar-surface)]">
  <Sidebar.Header class="px-3 pt-11 pb-1">
    <div class="flex items-center gap-2 px-2 py-1.5">
      <div
        class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-base text-white dark:text-[15px]"
        style={`background-color: ${colorHex};`}
      >
        {walletData.emoji}
      </div>
      <div class="min-w-0">
        <p class="truncate text-[13px] font-semibold text-sidebar-foreground">{walletData.name}</p>
      </div>
    </div>
  </Sidebar.Header>

  <Sidebar.Content class="px-2 pb-3">
    <Sidebar.Group>
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          {#each menuItems as item}
            <Sidebar.MenuItem>
              <Sidebar.MenuButton
                size="sm"
                class={menuButtonClass}
                isActive={isMenuItemActive(item.id)}
                onclick={() => handleMenuClick(item.id)}
                tooltipContent={item.title}
              >
                <item.icon class="size-3.5" />
                <span>{item.title}</span>
              </Sidebar.MenuButton>
            </Sidebar.MenuItem>
          {/each}
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>
  </Sidebar.Content>

  <Sidebar.Footer class="pt-0 pb-3">
    <Sidebar.Group class="p-2 pt-0">
      <Sidebar.GroupContent>
        <Sidebar.Menu>
          <Sidebar.MenuItem>
            <button
              type="button"
              class={footerButtonClass}
              aria-label={i18n.t('wallet.sidebar.openRequest')}
              onclick={onOpenRequest}
            >
              <Link2Icon class="size-4" />
              <span>{i18n.t('wallet.sidebar.openRequest')}</span>
            </button>
          </Sidebar.MenuItem>
          <Sidebar.MenuItem>
            <Sidebar.MenuButton
              size="sm"
              class={menuButtonClass}
              isActive={activeSection === 'settings'}
              onclick={handleSettingsClick}
              tooltipContent={i18n.t('wallet.topbar.settings')}
            >
              <SettingsIcon class="size-3.5" />
              <span>{i18n.t('wallet.topbar.settings')}</span>
            </Sidebar.MenuButton>
          </Sidebar.MenuItem>
          <Sidebar.MenuItem>
            <button
              type="button"
              class={footerButtonClass}
              aria-label={i18n.t('wallet.topbar.lockWallet')}
              onclick={handleLock}
            >
              <LockIcon class="size-4" />
              <span>{i18n.t('wallet.topbar.lockWallet')}</span>
            </button>
          </Sidebar.MenuItem>
        </Sidebar.Menu>
      </Sidebar.GroupContent>
    </Sidebar.Group>
  </Sidebar.Footer>
</Sidebar.Root>
