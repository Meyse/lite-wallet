---
owner: lite-wallet-team
last_reviewed: 2026-09-15
---

# Plan: VerusID avatar and description

- Status: proposed; feasibility work comes before implementation
- Owner: lite-wallet-team
- Last updated: 2026-09-15
- Scope: public avatar and short description in the desktop wallet

## Goal

Let people recognize and personalize their VerusID with an avatar and a short
description, with all published profile data stored on Verus ecosystem chains.

## Recommendation

Start with storage on the identity's own chain: a short description in its
`contentmultimap`, and an avatar descriptor in that map pointing to image data
stored in additional outputs of the same identity-update transaction. Initially
prove this for VRSC/VRSCTEST. Keep the editor to two fields.

**An avatar does not have to fit in 6 KB.** The PBaaS script-element limit is
6,000 bytes, including serialization overhead. Released Verus code supports
splitting larger evidence into multiple outputs. The identity can hold the small
reference while the image occupies those outputs. This corrects the earlier
6 KB image-limit recommendation. [Script limits][script-limits],
[identity update storage construction][update-storage].

This is entirely on-chain, but it is useful to distinguish **bytes inside the
identity record** from **on-chain bytes referenced by that record**. Larger images
use the second form. No Arweave, IPFS, image hosting service, or external image URL
is required.

Begin with a bounded technical feasibility milestone. It must establish image
quality, actual transaction costs, safe construction, and reliable retrieval
before the publishing UI is built. Treat vDEX as a later storage option, subject
to the cross-chain requirements below.

## Constraints and exclusions

- Only avatar and description. No banners, social links, profile themes, general
  file manager, new namespace identities, or currency registrations.
- Authentication protocol review and modernization are separate follow-up work.
  Existing request-handler code does not establish compatibility with current
  authentication flows.
- Preserve identity ownership, signing thresholds, recovery/revocation
  authorities, private addresses, and unrelated content.
- Profile publishing uses the wallet's backend preflight/sign/send boundary.
  No private keys, wallet viewing keys, or local filesystem paths go to RPC servers.
- Profile changes are public blockchain transactions. Removing a current avatar
  or description does not erase previously published data.
- This document authorizes no implementation, signing, broadcast, or mainnet
  expenditure. Research for this plan used source inspection and public read-only
  RPC calls; no storage transaction was published.

## Research findings

### Storage and retrieval

The supplied [AutoBB storage guide][wiki-storage] describes descriptors, multipart
evidence, and retrieval. It is a useful community guide; its large-file examples,
limits, and testnet cost examples are not wallet acceptance evidence or mainnet
quotes. The implementation baseline reviewed here is
[Verus v1.2.17-6][core-release], also reported by the queried live nodes.

- `updateidentity` recognizes a `data` wrapper within a content-multimap value.
  It packages the payload, adds evidence outputs, and writes a descriptor/reference
  to the identity. Oversized evidence goes through `BreakApart`.
  [Construction][update-storage], [multipart structures][multipart].
- The public-data path can use a temporary Sapling encryption wrapper with a
  published incoming viewing key. This is publicly readable data, not a private
  profile. [Construction][update-storage], [descriptor guide][wiki-descriptor].
- References can be relative to their containing transaction. Preserve each
  record's **originating transaction**, even after later identity updates.
  `getidentitycontent` returns aggregated values but its top-level `txid` belongs
  to the current identity output; it is not per-value provenance.
  [Content RPC][identity-content], [aggregation implementation][aggregation].
- Multimap changes accumulate through identity history; deletion uses explicit
  removal records. Do not treat the aggregated map as a document to overwrite or
  republish wholesale. [Aggregation and deletion][aggregation].
- Daemon `filename` inputs refer to the daemon's filesystem. The data-packing
  path also calls wallet-dependent `signdata`. Remote unsigned-template support
  must be demonstrated with in-memory bytes, not assumed from a local CLI example.
  [Data packing RPC][signdata].

An MMR is a hash structure used to describe and check data collections. It is not
an instruction to manually divide an avatar among arbitrary profile keys. Use
the existing descriptor/evidence format and its standard multipart handling.

### Existing VDXF names

The Verus website defines these semantic keys in its
[profile key registry][profile-keys]:

| Field | Qualified VDXF name | Key |
| --- | --- | --- |
| Avatar | `vrsc::system.identity.profile.avatar` | `iMMRVtGBNkr7V2hUNd4LLFiPQyzGrxAhx1` |
| Description | `vrsc::system.identity.profile.about` | `iAvXhoTu7EtDcGiUsyd1BysHMo1bTNDrTd` |

Reuse established meanings where the value format is compatible. The existence
of these names does **not** establish a universal inline/on-chain profile schema
or guarantee that other wallets will display our records. The website's
Arweave-oriented profile representation is not the proposed storage transport.

The primitives library also includes `vrsc::identity.profile.media` and standard
descriptor/reference types. Verify their relationship to existing profile readers
before choosing the final wire format. Freeze example serialized records and a
versioned encoding specification in milestone 0; do not silently assign an
incompatible value format to an existing convention.
[Library key definitions][library-keys].

### vDEX: real opportunity, additional work

Public `getinfo` and `getcurrency` calls on 2026-09-15 returned these settings:

| Chain | System ID | `transactionexportfee` in that chain's native coin |
| --- | --- | --- |
| VRSC | `i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV` | 0.01 VRSC |
| vDEX | `iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N` | 0.0005 vDEX |

Evidence endpoints: [Verus public RPC][vrsc-rpc] and [vDEX public RPC][vdex-rpc].
Both reported `VRSCversion: 1.2.17-6`; sampled heights were 4,238,874 and
1,079,841 respectively. `getcurrency("vDEX")` agreed on both endpoints. These
are POST JSON-RPC observations, not prices shown by those URLs in a browser.

The node's storage-fee calculation includes the serialized storage-script size
and the chain's `transactionExportFee`. The vDEX parameter is numerically 20
times lower. That does **not** mean a guaranteed 20-times-cheaper upload: the
coins have different values, and total fees include other transaction costs.
Recalculate against the actual transaction and selected network at preflight.
[Fee calculation][storage-fees].

There is also a concrete retrieval limitation: although evidence references can
carry a system ID, the released `CPBaaSEvidenceRef::GetOutputTransaction` accepts
only an absent system ID or its own chain ID. A VRSC node does not automatically
retrieve vDEX data through this path. The wallet would need explicit chain routing
and verification. [Reference resolution][reference-resolution].

For a VRSC identity with its image on vDEX, plan for:

1. A storage transaction on vDEX, with a proven funding/signing route.
2. Confirmation and retrieval of those exact image bytes.
3. A VRSC identity transaction anchoring the vDEX system ID, transaction/output
   reference, and content integrity information.
4. Recovery from interruption between those transactions. Uploaded data can remain
   unused if the identity update fails; the operation is not atomic.

Existing vDEX endpoint routing in this wallet is useful infrastructure, but it
does not implement this flow. Do not require users to export their identity,
register another identity, or convert funds merely to set an avatar. A generic
vDEX storage transaction without those steps still needs a demonstrated builder
and retrieval path.

### Wallet integration gaps

| Area | Current source evidence | Required change |
| --- | --- | --- |
| Avatar | `IdentityAvatar.svelte` generates initials and a gradient | Display a resolved image with the existing fallback |
| Profile data | `IdentityDetails` and `build_identity_details_from_payload` omit profile content | Add a bounded profile projection and retrieval service |
| Content retrieval | `VrpcProvider::getidentitycontent` passes only the identity argument | Selective queries, history/provenance, bounded resolution |
| Editor command | `IdentityPatch` covers control/address changes | Dedicated profile patch contract |
| Serialization | `normalize_vdxf_univalue` supports only a subset of typed values | Explicit support for the chosen descriptor format |
| Transaction validation | `validate_identity_transaction_intent` accepts the identity output and exact P2PKH change only | Validate the intended storage outputs without admitting arbitrary outputs |
| Fees | Current identity preflight uses `DEFAULT_FEE_SAT = 10_000` | Storage-aware fee calculation for the final funded transaction |

Source locations are listed under implementation ownership below. These are
source-level findings; they are not an end-to-end publishing test.

## Proposed product behavior

- Overview: avatar, full VerusID name, and existing status. Keep the name visible
  because images can be copied and are not proof of identity.
- Detail view: avatar and short description above the existing identity details.
  Keep addresses and controls in their existing detail areas; avoid a broader
  overview redesign in this change.
- One **Edit profile** action when this wallet can perform an ordinary identity
  update. Linked/watch-only or unsupported authority configurations remain readable
  and get a specific explanation when editing is unavailable.
- Editor: choose/crop/remove avatar and edit/remove description. One preview and
  one publication review with the backend-calculated fee and funding source.
- Explain publication once: profile information is public, and older versions
  remain on the blockchain. No VDXF keys, evidence-part controls, or chain-selection
  form in the initial user flow.
- Use **Publishing**, **Submitted**, and **Confirmed** for distinct states.
  Keep the previous confirmed profile visible while an update is pending, with
  a clear pending indication; broadcast alone must not replace it as confirmed.
- Missing or unsupported images use initials. Image failure must not block the
  VerusID list or identity controls.

### Initial content budgets

These are proposed product limits to validate in milestone 0, not protocol limits:

- Description: at most 160 user-perceived characters and 1,024 UTF-8 bytes, plain
  text only. Define identical counting and normalization rules on both sides.
- Avatar: crop to a square; evaluate 128 px and 256 px exports. Aim for 16–32 KiB
  and provisionally cap the encoded image at 64 KiB.
- Accept supported static raster input; re-encode locally to JPEG or PNG for the
  first version. Strip EXIF/location metadata and reject SVG, HTML, and animation.
- Bound source-file bytes and decoded pixels before processing. Confirm the image
  is a valid raster from its bytes, not merely its extension or supplied MIME type.
- Show the actual compressed preview. If acceptable quality cannot fit the chosen
  budget, explain that clearly instead of silently publishing a degraded image.

## Steps

### 0. Prove the storage contract and cost

- [ ] Build public, non-personal image fixtures at approximately 8, 16, 32, and
  64 KiB. Compare 128/256 px quality at the actual overview/detail display sizes.
- [ ] Demonstrate standard same-transaction multipart construction and reassembly
  against v1.2.17-6 or a freshly verified supported release. Record image bytes,
  transaction size, output count, overhead, and the complete fee breakdown.
- [ ] Demonstrate the remote-compatible unsigned-template path using serialized
  bytes. If the provider's wallet requirements prevent it, identify the exact
  descriptor/evidence serialization to implement locally. Do not introduce a
  server holding user keys as a workaround.
- [ ] Specify the exact avatar/description value layout, version, MIME handling,
  hashes and their algorithms, and optional MMR behavior. Check an independent
  existing reader/library; document compatibility actually demonstrated.
- [ ] Prove that an avatar remains resolvable after an unrelated identity update.
  Preserve its originating transaction/output, including self-references. An
  aggregated `getidentitycontent` response alone is insufficient for this case.
- [ ] Specify replacement/removal semantics and deterministic selection among
  multiple recognized records, using canonical history position and deletion
  records. Do not select the first array entry without a defined ordering.
- [ ] Recheck vDEX fees and estimate the complete two-chain flow separately.
  Mark any cross-chain construction or proof step not demonstrated.

Deliverable: fixture vectors, a compact size/quality/fee table, a supported wire
format, and a decision to proceed with the selected same-chain path. If that path
is unsuitable, report the specific blocker and revise the plan before implementing
publishing. Tests requiring new on-chain writes belong to subsequently authorized
implementation/testing, not the research that produced this document.

### 1. Add bounded profile reading

- [ ] Add a profile service returning description, image status, supported image
  bytes/handle, originating chain/transaction, and confirmation/version metadata.
  Keep raw VDXF objects out of the presentation component.
- [ ] Retrieve only relevant keys with explicit history bounds. Use transaction
  history or another proven provenance source for relative references. Process
  removal records and resolve the selected recognized value deterministically.
- [ ] Decode inline values and the chosen same-chain descriptor/evidence format.
  Verify serialized transaction references, payload digests/MMR where present,
  MIME, dimensions, and final bytes. Bind the expected digest to the published
  identity record; a hash returned next to untrusted image bytes is insufficient.
- [ ] Bound RPC response bytes, number of parts, nesting depth, history pages,
  decoded pixels, concurrency, and time. Derive concrete part/response limits
  from the fixture overhead before enabling the resolver.
- [ ] Never follow arbitrary URLs from profile content. Unsupported formats or
  chains yield a fallback; public profiles require no user wallet viewing keys.
- [ ] Cache verified images by network/system ID, identity ID, record provenance,
  and digest. Load lazily and coalesce duplicate requests; handle reorganizations,
  replacement, deletion, and provider outages without stale profile promotion.
- [ ] Scope asynchronous results to the current account/session and identity.

### 2. Add safe profile publication

- [ ] Add a dedicated backend profile request: set/remove avatar and set/remove
  description, with immutable normalized payloads. Backend validation repeats the
  content limits; frontend validation is only for feedback.
- [ ] Fetch the latest canonical identity and check active status, update authority,
  supported signing capability, and the exact identity input being spent.
- [ ] Construct only the intended profile additions and exact removal records for
  recognized values being replaced. Preserve unknown values, unrelated keys,
  content-map entries, and all identity control fields. Do not copy aggregated
  historical content into the next update or clear an entire map/key casually.
- [ ] Extend transaction intent with a narrowly specified storage contract: exact
  identity semantics, evidence format, payload digest, part count/order, output
  scripts/amounts, reference targets, funding inputs, change, and fee. Derive this
  from the user's normalized image and description, not from a server template.
- [ ] Check the transformed identity reference against the generated evidence:
  the submitted `data` wrapper will not be byte-identical to the resulting identity
  descriptor. Reassemble candidate evidence and verify that it represents precisely
  the approved payload before signing.
- [ ] Reject omitted, additional, reordered-invalid, mismatched, or spend-bearing
  storage outputs. Retain the existing strict rules for ordinary identity updates;
  do not add a blanket allowance for `EVAL_NOTARY_EVIDENCE`.
- [ ] Calculate storage, identity-content, input/output, and funding/change fees
  on the final transaction using supported chain policy. Check integer arithmetic,
  provider policy acceptance, available funds, and the approved total fee.
- [ ] Bind the prepared transaction to a single-use, expiring `preflight_id`,
  account/session, network, funding source, identity outpoint, payload digest, and
  fee. An identity update, payload edit, or funding change invalidates the quote.
- [ ] Sign locally, broadcast once, and reconcile pending state against the chain.
  Support safe restart/retry without duplicate publication or automatic re-signing.

### 3. Add the two-field editor and display

- [ ] Extend the existing avatar component with loading/verified-image/fallback
  behavior and integrate the description into the detail header.
- [ ] Add the editor, local crop/preview, explicit removal, publication review,
  cancellation, pending state, and action-specific errors.
- [ ] Surface fee affordability before signing. Use existing funding patterns;
  changing funding requires a fresh backend preflight.
- [ ] Use translation keys, sentence case, existing input/button styles, Lucide
  icons, normal arrow cursors, and keyboard-accessible controls.
- [ ] Verify light/dark mode and normal/compact desktop layouts, including long
  identity names, empty descriptions, unsupported images, and pending updates.

### 4. Verify and release within proven capability

- [ ] Complete the verification matrix below and document the exact networks,
  node/provider versions, signing configurations, and wire formats tested.
- [ ] Enable publishing only for combinations with proven construction, signing,
  fees, and retrieval. Unsupported cases retain readable identity details.
- [ ] Document the public-history behavior and the supported profile encoding.
  Record cross-wallet compatibility limits and keep authentication work separate.

## Implementation ownership and likely files

These are starting points, not permission to refactor unrelated flows. The
checkout already contains other identity/UI edits; re-inspect Git state and
integrate with those changes when implementation is authorized.

| Responsibility | Existing files / proposed module |
| --- | --- |
| Backend profile API | `src-tauri/src/commands/identity.rs`, `src-tauri/src/types/identity.rs`; prefer a focused profile module if this grows |
| Content retrieval and chain routing | `src-tauri/src/core/channels/vrpc/provider.rs`, `src-tauri/src/core/runtime_config.rs` |
| Descriptor codec and bounded resolver | Proposed module beside `src-tauri/src/core/channels/vrpc/identity/` with shared test vectors |
| Preflight and local signing integration | `src-tauri/src/core/channels/vrpc/identity/preflight.rs`, `send.rs`, and `verus_tx/` |
| Strict output validation | `src-tauri/src/core/channels/vrpc/intent.rs` |
| Frontend profile service | `src/lib/services/identityService.ts` or a focused adjacent service |
| Avatar/editor/detail presentation | `src/lib/components/wallet/sections/identity/IdentityAvatar.svelte`, `IdentityDetailView.svelte`, `LinkedIdentityCard.svelte`, `LinkedIdentityRow.svelte`, and focused new editor component(s) |
| User-facing copy | `src/lib/i18n/locales/` using the existing locale policy |

The generic identity-request implementation is reference material for existing
session/preflight behavior. Profile editing must not depend on modernizing login
or expanding external authentication request formats.

## Verification

| Area | Required evidence |
| --- | --- |
| Encoding | Known source/library vectors; inline and multipart round trips; invalid/truncated/oversized descriptors fail safely |
| Integrity | Modified image part, reference, MIME, hash/MMR, output count, amount, or authority is rejected before signing |
| History | Add, replace, remove, multiple values, unrelated later update, and reorganization resolve the correct confirmed profile |
| Preservation | Unknown map values and every non-profile identity control field survive profile edits |
| Fees | Actual funded fixtures match supported node policy; insufficient funds, extra funding inputs, change, and size boundaries covered |
| Lifecycle | Lock/account/network switch, expired preflight, concurrent identity update, duplicate submit, interruption and restart |
| Media | Oversized source, excessive decoded pixels, misleading MIME, metadata stripping, animation/SVG rejection, acceptable small-image quality |
| UI | Rendered light/dark and normal/compact desktop; keyboard use; readable name/status; empty/error/pending states |
| End to end | Authorized disposable testnet identity publishes, confirms, resolves on a fresh client, changes another field, still resolves, replaces, then removes |

During implementation, run focused Rust codec/intent/preflight tests, relevant
Vitest service/component tests, `pnpm check`, and `pnpm lint:ui` as appropriate.
Use the repository-pinned pnpm version. Broaden testing when shared behavior
changes; static and fixture checks do not substitute for the testnet sequence.

## Later decision: enable vDEX storage for identities on other chains

Proceed only after all of the following have evidence:

- A storage-only vDEX transaction can be funded, validated, and signed through
  the wallet without creating/exporting an identity just for storage.
- A versioned cross-chain reference can be anchored in the original identity and
  resolved through an allowlisted, chain-verified vDEX provider.
- Image integrity and chain confirmation have explicit verification rules. Digest
  agreement proves bytes; it does not by itself prove chain inclusion/finality.
- Both transactions have fee quotes in the correct native currencies, sufficient
  funding, honest confirmation states, and durable interruption recovery.
- Measured total cost and latency improve enough to justify the extra dependency
  and the possibility of paying for an upload whose reference is never published.

Keeping the data model aware of system IDs now avoids redesign later. It does not
require exposing a storage-chain choice in the first editor.

## Exit criteria

- A supported, update-capable VerusID can publish an avatar and description through
  one understandable flow, with no external image storage.
- A fresh wallet can retrieve the confirmed profile, including an image larger
  than one script element, and continue doing so after unrelated identity updates.
- Exact fees, identity controls, profile bytes, and every transaction output are
  validated before signing. Invalid or unsupported data fails safely.
- Replacement and removal affect current presentation without claiming that
  historical blockchain data has been erased.
- The supported network/provider matrix and any cross-wallet interoperability
  limitations are explicit. Authentication modernization remains separate.

[wiki-storage]: https://wiki.autobb.app/concepts/on-chain-file-storage/
[wiki-descriptor]: https://wiki.autobb.app/concepts/data-descriptor/
[core-release]: https://github.com/VerusCoin/VerusCoin/releases/tag/v1.2.17-6
[script-limits]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/script/script.h#L34-L36
[update-storage]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/rpc/pbaasrpc.cpp#L16038-L16442
[multipart]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/primitives/block.cpp
[identity-content]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/rpc/pbaasrpc.cpp#L17445-L17613
[aggregation]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/pbaas/identity.cpp#L475-L581
[signdata]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/wallet/rpcwallet.cpp#L1170-L1460
[profile-keys]: https://github.com/VerusCoin/verus.io/blob/master/data/vdxfid/identityJSON.ts
[library-keys]: https://github.com/VerusCoin/verus-typescript-primitives/blob/master/src/vdxf/vdxfdatakeys.ts
[storage-fees]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/rpc/pbaasrpc.cpp#L10557-L10661
[reference-resolution]: https://github.com/VerusCoin/VerusCoin/blob/v1.2.17-6/src/pbaas/notarization.cpp#L1033-L1098
[vrsc-rpc]: https://api.verus.services
[vdex-rpc]: https://api.vdex.to
