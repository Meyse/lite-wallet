import type { WalletNetwork } from '$lib/types/wallet';
export type ImportMethod = 'seed24' | 'text';

export type WalletData = {
  name: string;
  emoji: string;
  color: string;
  password: string;
  network: WalletNetwork;
};

export type WalletUpdate = Partial<WalletData>;
