import { extractWalletErrorMessage, extractWalletErrorType } from '$lib/utils/walletErrors.js';

/**
 * Why Rust refuses a request (`GenericRequestRejected` in
 * `src-tauri/src/core/generic_request`). Each code has translated copy under
 * `genericRequest.rejected.<code>`.
 */
export const REJECTION_CODES = [
  'unknownDetail',
  'detailOrder',
  'unsigned',
  'walletBackup',
  'network',
  'encryptedDetails',
  'tooLong',
  'expired',
  'delegatedSigner',
  'signature',
  'notOpen',
  'responder',
  'unsupportedDetail',
  'responseAddress',
  'insecureDelivery',
  'invoiceTagged',
  'invoiceUnpayable',
  'invoicePrivate',
  'invoiceNoDestination',
  'invoiceCurrency',
  'invoiceCurrencyMissing',
  'invoicePreconvertLaunched',
  'alreadyPaid',
  'dataTransmittal',
  'dataNotForSignature',
  'encryptionKeyAddress',
  'encryptionKeyIdentity',
  'privateNotSetUp',
  'credentialsOnly',
  'credentialsNotEncrypted',
  'credentialAddress',
  'loginContext',
  'loginPermission',
  'giftCardInvalid',
  'giftCardPassword',
  'giftCardEmpty',
  'giftCardIdentity',
] as const;

export type RejectionCode = (typeof REJECTION_CODES)[number];

function isRejectionCode(value: string): value is RejectionCode {
  return (REJECTION_CODES as readonly string[]).includes(value);
}

export function rejectionCode(error: unknown): RejectionCode | null {
  if (extractWalletErrorType(error) !== 'GenericRequestRejected') return null;
  const code = (extractWalletErrorMessage(error) ?? '').trim();
  return isRejectionCode(code) ? code : null;
}

/** An error the TypeScript side raises for a rule it checks itself. */
export function rejectionError(code: RejectionCode): Error {
  return new Error(`genericRequest.rejected.${code}`);
}
