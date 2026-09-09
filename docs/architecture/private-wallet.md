---
owner: lite-wallet-team
last_reviewed: 2026-09-09
---

# Private Sapling wallet

The private wallet uses one Rust scanner and one encrypted state generation for
balances, history, the Sapling tree, witnesses, and pending sends. Sending still
requires a backend-owned, session-scoped, single-use preflight ID.

## Mobile-compatible keys

Private phrase derivation matches Verus Mobile: `m/32'/133'/0'` on both mainnet
and testnet, with `zs` addresses and `secret-extended-key-main` spending-key
exports. Testnet does not select coin type 1. Imported extended spending keys
retain their key material; exports normalize the prefix. There is no alternate
Lite Wallet derivation option. Earlier local testnet phrases therefore resolve
to the Mobile-compatible address. The original phrase remains in Stronghold.

The selected network controls endpoints, storage, and Sapling activation height
(227520 for VRSC, 1 for VRSCTEST). Consensus explicitly remains Sapling v4 with
pre-ZIP-212 note encryption on both chains. Later Zcash upgrades cannot activate
through a default library network configuration.

## Encrypted state and scanning

The cache lives under
`dlight/<network>/<account hash>/<coin>/scope-<address hash>/private-wallet.cache`.
XChaCha20-Poly1305 authenticates and encrypts each generation with a random
24-byte nonce. Its key derives from the spending key and is bound to the
account, coin, and address. The background scanner retains a viewing key and
cache encryption key, not the mnemonic or spending key.

An exclusive file lock prevents another process from writing the same scope. A
write encrypts a complete generation, syncs a private temporary file, renames
it, and syncs the directory. Unix directories use 0700 and files use 0600. An
unreadable or unauthentic encrypted generation fails closed. It is never
silently replaced with an empty balance or pending-send journal.

Pre-release plaintext caches are not resumed. Their exact contents are archived
and verified under encryption before removing the plaintext files, and the new
scanner rebuilds from the chain. This cannot erase earlier backups, filesystem
snapshots, or logs produced by an older build.

One connection downloads each block range once. A blocking worker prepares scan
keys once per batch, decrypts incoming outputs, checks every transaction's
nullifiers, updates witnesses, and commits the generation. Checking every
transaction also records payments with no change and spends of notes received
earlier in the same block. Repeated reads use the unlocked runtime rather than
reloading Stronghold or deserializing the cache. An unchanged synchronized chain
does not rewrite the state file.

Verus lightwalletd can report zero as a placeholder tree size. In that case the
scanner calculates the size from its prior tree and outputs. Every batch is
checked against the server's full tree, including its root, before committing.

Recent batch checkpoints contain matching block hashes, notes, and trees. A
same-height replacement or lower tip triggers rollback of notes, witnesses,
history, and pending confirmations together. The scanner finds a retained
ancestor or rescans from the creation checkpoint/activation. An unverified tree
is never attached to an arbitrary replacement hash.

Only a newly generated, unexposed private phrase can receive a creation
checkpoint, stored alongside the secret in Stronghold. It contains the complete
Sapling tree and block hash, not only a height. Imported or reused phrases scan
from activation. If the server cannot supply a valid creation checkpoint, a new
phrase also uses the complete scan. Startup verifies the checkpoint hash before
using it.

## Pending sends and lock

1. Select and durably reserve unspent notes through the same writer as scanning.
   Concurrent sends cannot reserve the same notes.
2. Build on a blocking worker. Parsed proving parameters are shared across
   sends; cold loading parses the exact bytes that passed checksum verification.
3. Recheck the anchor and inputs, then persist transaction bytes, ID, expiry,
   destination, and reserved nullifiers before submitting to the network.
4. Admit every poll of the broadcast future through the session guard. Lock,
   logout, timeout, and account changes cancel further transport admission.
5. Reconcile against mined transactions, conflicting spends, or scanning past
   expiry. A transport error alone never releases a prepared reservation.

An interrupted build with no persisted transaction is safe to release. Prepared
transactions survive restart, remain visible as pending with zero confirmations,
and are never automatically rebroadcast. An uncertain submission asks the user
to check history before retrying. A reorganization restores reservations when
confirmations are rolled back.

CPU proving cannot be interrupted inside the cryptographic library. Lock
suppresses progress, discards the result, and blocks later submission; key
objects inside an already running proof are dropped when that worker finishes. A
network poll admitted before lock may already have reached the server.

The encrypted state contains the durable pending-send journal. Do not delete it
while a submission is unresolved: chain history alone cannot establish whether
an unexpired transaction was broadcast.

## Dependencies and proof boundaries

Compatible updates are `zcash_client_backend` 0.21.2, `zcash_transparent` 0.6.4,
`zcash_note_encryption` 0.4.2, and transitive `shardtree` 0.6.2. This wallet
manages its own witnesses; the shardtree patch does not replace the state/reorg
fixes. A newer librustzcash family needs a separate compatibility exercise
against v4 serialization/signatures, pre-ZIP-212 encryption, Mobile key
fixtures, and a live disposable-testnet send. Shared witness-tree storage is a
separate optimization.

Tests cover real compact-note encryption/scanning, encrypted cache
authentication, concurrent reservations and scanning, pending restart/expiry,
chain rollback, malformed blocks, and session cancellation. The explicit proving
test constructs a v4 transaction with canonical parameters, validates its proofs
and signatures, and rescans its pre-ZIP-212 outputs:

```sh
cd src-tauri
cargo test --locked --offline --lib -- --test-threads=1
cargo test --locked --offline --lib real_proof_roundtrip -- --ignored --test-threads=1
```

The explicit `live_testnet_checkpoint` test also passed against the configured
testnet lightwalletd: it fetches a creation checkpoint and five subsequent
blocks, then verifies the reconstructed tree root and size against the server.
Run it with `LITE_WALLET_TEST_DLIGHT_ENDPOINT` set to the public endpoint and
the same `--ignored --test-threads=1` flags. This is a read-only RPC check.

These checks do not establish native synchronization latency or live-daemon
acceptance. The externally operated testnet lightwalletd was behind upstream in
the September 9 investigation; a repository change does not upgrade that server.
