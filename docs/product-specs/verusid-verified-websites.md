---
owner: lite-wallet-team
last_reviewed: 2026-09-19
---

# Verified websites for VerusID profiles

Status: implementation spec draft. The user requested this feature and its
guided setup on 2026-09-19. Product behavior below is the proposed
implementation contract; wire-format details and numeric limits are proposed
defaults to validate with protocol fixtures before publishing. Nothing here
claims that the feature is implemented, that mainnet publishing is enabled, or
that another Verus application already supports the proposed file format.

This is an optional extension to
[Contacts and VerusID profiles](./contacts-and-verusid-profiles.md). Use the
[shared glossary](../../CONTEXT.md). The original Contacts work remains
independently deliverable.

## Design reference

[Paper: Lite Wallet · Verified website setup](https://app.paper.design/file/01M2VY6SRHDQ63592GZARQVJR1)
contains an initial desktop design at 920×620: 11 main-flow states and seven
upload-help/recovery states, each in light and dark mode, plus a flow map. The
main flow covers website entry, signing, saving the proof file, upload
instructions, checking, publication review, confirmation, and the website on a
contact. The help paths cover a hosting dashboard, a source project, and a
website manager, with missing-file, hosting-limit, and resume states.

These are static design screens using illustrative identities, domains, dates,
and fees. Native authorization, file saving, hosting tools, and real publication
remain implementation work. Use this document for the behavioral contract;
renewal, replacement, removal, detailed verification states, and other edge
cases are specified below but are not yet drawn in Paper.

## Outcome and scope

A person who controls a VerusID and can publish a file on a website can link
them through the wallet. The wallet teaches the person how to publish the proof,
checks their work, and publishes the website reference with their identity.
Another wallet can discover the reference and independently verify the link.

The first version supports one website origin per VerusID on VRSC and VRSCTEST,
with separate network context. One website may carry proofs for several IDs.
Website publication is optional; Contacts, avatar/description publishing, and
payments remain usable without it. No hosting account is required by the wallet,
but the owner needs an existing HTTPS website with permission to publish at the
required path.

Include discovery, verification, owner setup, upload help, publication review,
pending confirmation, renewal, replacement, removal, and failure recovery.
Legacy website records are read-only compatibility inputs. Do not add social
accounts, DNS verification, email verification, hosting purchases, automatic
website deployment, arbitrary HTML editing, or website-derived payment
addresses.

**Website verified** means the current published identity reference, a valid
identity signature, and the proof served at the exact website origin agree at
the recorded check time. It is evidence of a published connection, not a legal
ownership certificate, a real-person check, an endorsement, or payment trust.
Domain transfers and compromise cannot be detected perfectly; validity periods
and rechecking limit the age of evidence without promising continuous control.

## Product placement

- **VerusID → Edit profile → Website:** owner setup and management. Adding a
  website opens the guided flow; avatar and description keep their existing
  editor. Do not add website setup to private contact editing or Settings.
- **Full contact profile and VerusID detail:** show the published domain, its
  website-specific status, and the external-link action. Omit the row when no
  website is published. Opening the link uses the system browser, not an
  embedded page inside the wallet.
- **Compact profile preview:** show a website row only when a record is already
  available. Reuse a fresh verification result; passive hover/focus does not
  contact the website. Offer **Check website** when needed. Keep Add/View
  contact as the primary action and do not add a badge to the identity name
  itself.
- **Details:** activating the status reveals what was checked, the exact origin,
  identity/network, proof URL, and last-check time. Keep raw signature data
  behind an optional proof-details disclosure.

Use existing wallet components and tokens, Lucide icons, sentence case,
`i18n.t(...)`, English/Dutch translations, and the normal arrow cursor. Verify
light/dark at 920×620 and a larger desktop size. Status needs readable text and
accessible names; color or a shield/check icon alone is insufficient. Do not
change the Contacts saved-state colors or reuse its transient Saved check as a
website-verification signal.

## Owner journey

### 1. Enter the website

Show **Website** with an example such as `https://example.com` and one
**Continue** action. Normalize a bare hostname to HTTPS and show the resulting
origin before signing. Explain only what is needed now:

> You’ll upload a small file to your website to link it to this VerusID.

Use the exact origin, not a page, account, or registrable-domain guess.
`example.com`, `www.example.com`, and `shop.example.com` are different origins.
Reject credentials, HTTP, IP literals, local/private destinations, nonstandard
ports, query strings, fragments, and paths other than `/`. For a pasted page
URL, offer its origin for explicit selection rather than silently stripping the
path. Display internationalized hosts with an unambiguous ASCII/punycode form in
the signing/publication review and proof details; labels cannot conceal the
host.

Check the active identity and local signing capability before asking the person
to upload anything. Unsupported chains or signing policies show a concise reason
and retain read-only website viewing. Do not silently relax an existing
publishing restriction. The guided v1 writer supports active identities with one
primary signing address and a one-signature policy; other identities remain
read-only until their signing policy has separately proven support.

### 2. Create the proof file

Present the exact VerusID, network, website origin, and proof expiry. Explain:

> This creates a public website proof. It does not send funds or publish an
> identity update yet.

**Create proof file** authorizes only the locally constructed, website-specific
statement. Use the existing unlock/signing guard. Private keys never leave the
wallet; the UI never supplies arbitrary signing bytes. The backend captures the
identity, current authority revision, network, and origin in a session-bound
draft and generates the statement and signature.

Offer **Save proof file** through a native file-save dialog, defaulting to
`verusid.json`. Cancelling the save is not an error and does not publish
anything. The file contains public proof data only. Explain that the file is
safe to put on the website, but the wallet password or recovery phrase must
never be uploaded.

Before preparing a replacement file, check for an existing supported proof file
at that origin, with the same bounded fetch rules used for verification.
Preserve other identities' entries; do not discard an existing document to make
room. If a file exists but is unsupported, exceeds limits, or cannot be safely
merged, provide **Show integration instructions** and stop the overwrite path. A
changed file since draft creation requires reloading/remerging. Recheck
unrelated entries after upload and identify missing entries; never report a
successful merge from the downloaded draft alone. The wallet cannot make
deployment on a third-party host atomic, so the upload guidance includes backing
up and coordinating edits.

### 3. Teach the upload

Show **Upload to your website**, the saved filename, the exact destination URL,
and **Copy URL**. Keep the core instructions visible:

1. Open the hosting dashboard or files for your website.
2. In the website’s public root, create a folder named `.well-known` if needed.
3. Upload `verusid.json` into that folder without editing its contents.
4. Publish/deploy your website if your host requires it.
5. Return here and select **Check website**.

For `https://example.com`, the file must be available at
`https://example.com/.well-known/verusid.json`. Clarify that `.well-known`
begins with a dot and may be hidden by a file manager. A file on the person's
computer, a dashboard preview, a download-sharing page, or a file that requires
login does not satisfy this requirement.

Provide **How do I upload this?** with three short paths:

| Hosting situation                          | Guidance                                                                                                                                                                                                                                                    |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hosting dashboard/file manager or SFTP     | Locate the public web root; its name varies by host, sometimes `public_html`. Enable hidden files, create `.well-known`, back up an existing proof file, upload, and publish if required. Do not ask for hosting credentials.                               |
| Site built from source                     | Add the file to the framework's public/static output so the deployed URL is exactly the one shown. `public/.well-known/verusid.json` is an example, not a universal path. Make sure rewrites do not return the app's HTML shell.                            |
| Website builder or someone else manages it | Check whether the host supports a file at this exact path. Offer **Copy instructions for my website manager**, containing only the target URL, filename, and upload steps. The user attaches the saved file themselves; the wallet does not send a message. |

The copyable website-manager text uses this template, with the real proof URL
inserted and no wallet/account details:

> Please publish the attached `verusid.json` file at {proof_url}. Keep its
> contents unchanged and make the file publicly accessible over HTTPS without
> sign-in or redirects. If that URL already serves a proof file, back it up and
> coordinate the update so other identities' proofs are preserved. Publish the
> website changes, then let me know so I can check them in my wallet.

Provide **Open proof URL** as an optional browser check. Explain: “The page
should show the proof file, not your homepage or a sign-in screen. Some browsers
download it instead.” The wallet still verifies the response itself; browser
appearance alone is not success. Hosts may serve JSON as `application/json`,
`text/plain`, or a download; the bounded body must parse as the supported proof
document.

If the host cannot serve the exact file, state **This host doesn’t support this
verification method** and allow leaving setup with the draft retained. Do not
suggest pasting a signature into a random page or buying hosting as an automatic
fallback. Provider-specific instructions, if added later, need current official
documentation and a visible maintenance owner/date.

### 4. Check the hosted proof

**Check website** fetches the public proof and verifies its signature, origin,
identity, chain, connection ID, and lifetime. Show progress in the action area;
repeated clicks coalesce. Keep the URL and upload help accessible during
failure. An external browser preview, local file, or signature-only result
cannot satisfy this step.

On success show **Website checked** and offer **Review publication**. This is a
setup result, not a published-profile badge. Draft evidence is never visible on
someone else's contact profile.

### 5. Review and publish

Review the identity, network, exact website, publication fee in the network's
currency, and what changes publicly. State that anyone can discover the
connection and older on-chain records remain in history. Proof signing has no
transaction fee; publishing the identity reference is a separate paid action.

**Publish website** uses the existing backend-owned transaction/preflight flow.
Revalidate current authority, identity revision, and hosted proof during
preflight. A preflight is single-use, expires after at most five minutes, and
cannot execute under a different account/session/network. A stale revision or
changed intent requires a fresh review, not automatic re-signing or retargeting.
An unrelated identity update must never be overwritten.

Show **Publishing…** and then **Waiting for confirmation**. A broadcast result
is not confirmation. Preserve the previous confirmed website during replacement;
show the draft/pending result only in the owner's management flow. On canonical
confirmation, independently read the published record and recheck the hosted
proof before showing **Website verified**. If hosting has failed meanwhile, the
publication can be confirmed while verification is unavailable; report both
facts without repeating the transaction.

### 6. Leave and resume

Leaving setup keeps an encrypted draft scoped to account/network/identity.
Persist only the selected origin, public proof, connection ID, stage, and
publication reconciliation data, never private signing material or hosting
credentials. Clear in-memory data on lock; resume only after unlocking the same
account. On resume, revalidate identity authority, expiry, hosted content, and
any pending transaction before choosing the next action. Reuse a valid proof
instead of asking for another signature after every navigation.

**Discard setup** clears the local draft; it does not delete a remotely uploaded
file. Explain file cleanup when relevant. An uncertain broadcast must first be
reconciled against its transaction and current identity record; retry never
blindly submits a duplicate update. A reorg removes confirmed presentation until
canonical evidence is re-established.

## Viewing and verification states

Discover references through the public identity reader, separately from fetching
the website. A fresh result is shared across Contacts, VerusID details, and
previews. First-time or stale website checks require **Check website** or an
explicit profile refresh; opening/hovering a list must not start a website
crawl. Explain before the first direct request:

> Checking connects to this website, which can see your IP address.

Keep that explanation available in details; no repeated modal confirmation is
needed. Owner setup explains the same direct request before its first fetch. Do
not silently introduce a central proxy: that changes who sees the identity
lookups and requires a separate product decision.

| Evidence                                                                                               | Visible status and behavior                                                                        |
| ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| No published reference                                                                                 | Omit the website row.                                                                              |
| Reference found, never checked                                                                         | **Not checked**, with **Check website**.                                                           |
| Request in progress                                                                                    | **Checking…**; saving contacts and Send remain available.                                          |
| Current reference and hosted proof both pass                                                           | **Website verified**; details include last-check time.                                             |
| Previously passed, freshness elapsed                                                                   | **Check needed**; details retain the last successful time without a current check badge.           |
| DNS, TLS, timeout, offline, rate limit, server error, or RPC outage                                    | **Couldn’t verify**, with **Try again**; distinguish temporary availability from invalid evidence. |
| File missing, expired, malformed, wrong identity/origin/chain, invalid signature, or inactive identity | **Not verified** with a specific reason in details. Never retain a success badge.                  |
| Unknown proof version or unsupported legacy claim/signing policy                                       | **Verification not supported**; do not call it invalid or silently accept it.                      |
| Conflicting current records or multiple matching proofs                                                | **Not verified**; report the conflict, never choose by list order.                                 |

Keep the published domain visible without a verification badge when a check
fails, provided it passes link-safety validation. A domain label must match the
actual origin opened. The published URL never changes a recipient, imports an
address, or authorizes payment.

## Management after publication

- **Renew proof:** default proof lifetime is 365 days, with **Renew proof**
  offered in owner management during the final 30 days or after expiry. Re-sign
  and upload a new proof for the same connection ID and origin. No identity
  update or fee is required if the published reference is unchanged. Do not
  create a background reminder or notification subscription as part of this
  feature.
- **Identity keys changed/recovered:** drop cached success and require a fresh
  check against current authority. If the old signature no longer passes, guide
  the owner through creating and uploading a new proof. Historical validity
  alone cannot restore the current badge. Revoked identities cannot verify.
- **Change website:** run the complete flow for the new origin and a new random
  connection ID. Keep the old confirmed reference until the replacement
  confirms. Afterwards explain that the old site's proof entry can be removed.
- **Remove website:** review a public identity update and fee. After
  confirmation, suppress the website everywhere and invalidate cached results.
  Publish an explicit removal marker so an older legacy record cannot reappear
  through fallback. Explain that this does not erase chain history and that the
  owner should remove only their proof entry from the website file.
- **File removed without updating the identity:** the reference remains public
  but fails the next check. Do not pretend local contact deletion or
  website-file deletion removed the public identity reference.

## Troubleshooting content

Use actionable copy at the failed step and keep **Upload instructions** nearby.
Share these help definitions across editor and details rather than duplicating
strings. Backend diagnostics are typed; raw HTML, RPC messages, and stack traces
are not user-facing content.

| Condition                                  | Suggested explanation/action                                                                                                                         |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 404                                        | “We couldn’t find the file at this address.” Show the exact URL and check folder spelling, public root, and deployment.                              |
| Homepage/HTML returned                     | “This address serves a webpage instead of your proof file.” Explain static-file placement and routing rules.                                         |
| 401/403 or login challenge                 | “The proof file must be public.” Check access protection; do not ask for a password.                                                                 |
| DNS/TLS failure                            | “We couldn’t connect securely to this website.” Check spelling, HTTPS, and hosting configuration; never offer to ignore certificate errors.          |
| Cross-origin redirect, such as apex to www | “This address redirects to a different website address.” Show both origins and offer restarting with the final origin; require a new matching proof. |
| Different/older file                       | “The website is serving a different proof file.” Re-upload the latest file, deploy, and check hosting/CDN cache.                                     |
| Modified/invalid signature                 | “This proof could not be verified.” Offer recreating the file; explain that its contents must remain unchanged.                                      |
| Expired proof                              | “This proof has expired.” Owner gets **Renew proof**; viewers get the reason without an owner action.                                                |
| Identity/network mismatch                  | “This file is for a different VerusID or network.” Keep the intended identity visible and offer regenerating its file.                               |
| Insufficient publication funds             | Show the required fee/balance through the normal transaction review; retain the checked draft.                                                       |
| Unsupported host or existing file conflict | Explain the exact requirement and offer copyable instructions for the website manager, without destroying other proofs.                              |

## Publication and proof contract

These are explicit proposed v1 defaults. Freeze the byte-level fixtures and
validate the VDXF/storage adapter before implementation publishes any record.
The names below are an application convention, not a claim of ecosystem
standardization or compatibility with verus.io's current parser.

### Identity reference

Use the existing website service identifier `vrsc::system.services.website` /
`i3X9irKQDqHNtYxfwEzECPHRV1RzzGnkHf` after confirming its encoding on each
supported chain. Store a versioned, public record through the wallet's on-chain
content/descriptor machinery, not a new Arweave publishing dependency. Define
the exact descriptor/data wrapper with a round-trip node fixture before enabling
writes.

The logical record has `format: "lite-wallet-website"`, `version: 1`, and a
`state` of `"linked"` or `"removed"`. A linked record includes canonical
`origin`, `proof_url`, and `connection_id`. The origin is HTTPS, ASCII host, no
trailing dot/path, and implicit port 443; the proof URL is exactly that origin
plus `/.well-known/verusid.json`. `connection_id` is 32 cryptographically random
bytes encoded as 64 lowercase hexadecimal characters. A removal record has no
active origin or proof URL. The containing authoritative identity supplies the
signer and chain context; never trust a self-reported record to select a
different ID.

There must be one unambiguous active record. Updates remove/replace only known
website records belonging to this format and preserve unrelated/unknown identity
content. Unsupported website data is not disposable. Resolve conflicts
explicitly before writing. Native removal or unsupported/conflicting native
records suppress legacy fallback. Returning to a removed origin uses a new
connection ID, so old proofs cannot reactivate a newly published connection.

### Hosted proof file and signed bytes

Use this envelope, with at most 16 proof entries and 64 KiB decoded body size:

```json
{
  "format": "verusid-website-proof",
  "version": 1,
  "proofs": [
    {
      "message": "<canonical statement generated by the wallet>",
      "signature": "<Verus signmessage-compatible Base64 signature>"
    }
  ]
}
```

Each message is exactly these UTF-8 lines, separated by LF, without a final LF:

```text
VerusID website proof v1
network=<VRSC or VRSCTEST>
chain=<canonical system identity address>
identity=<canonical identity address>
origin=<canonical HTTPS origin>
proof_url=<canonical proof URL>
connection_id=<64 lowercase hexadecimal characters>
issued_at=<Unix timestamp in whole seconds>
expires_at=<Unix timestamp in whole seconds>
```

Angle-bracket values are placeholders, not literal bytes. Reject embedded
newlines, missing/repeated/out-of-order fields, duplicate JSON keys, unexpected
v1 fields, and noncanonical values. JSON whitespace/escaping may differ, but the
decoded message bytes must match the canonical reconstruction exactly. Preserve
Base58 case. Validate timestamps as bounded integers: issuance no more than five
minutes in the future, expiry strictly after issuance and in the future, maximum
lifetime 365 days. Expiry ends success even within a cache interval.

Use Verus's ordinary identity message-signature semantics, including its message
serialization, not a generic Bitcoin signature or an assumed reuse of the
GenericRequest hash. Prove generation/verification against the full-node
`signmessage`/`verifymessage` implementation on both supported networks. Signing
stays local; the public RPC must never receive wallet secrets or a signing job.

Select by exact network/chain/identity/connection ID, never “first” or “last”
entry. Reject duplicate matches. When renewing, replace only that matching
entry; preserve unrelated entries without trusting their claims. A structured
file cannot fall back to HTML parsing after validation fails.

### Verification algorithm

Use two explicit modes: **setup check** binds to the backend-owned draft and
current identity authority; **published verification** binds to the confirmed
identity reference. Both run the same host, signature, and lifetime checks, but
only published verification can create a public **Website verified** result.
Never require an already published record to finish pre-publication setup, and
never promote a setup result directly into the shared public cache.

1. Resolve the canonical identity on its specified chain and current revision;
   require an active identity plus an unambiguous confirmed reference or, in
   setup mode, the current guarded draft.
2. Validate the origin/proof URL and fetch policy before any website request.
3. Fetch bounded bytes without executing HTML/JavaScript or loading
   subresources.
4. Parse the supported format and select the exact proof. Require every signed
   field to agree with the identity reference, fetched origin/path, and network.
5. Verify with `verifymessage(identityAddress, signature, message, true)`
   against the configured chain, or a locally equivalent, independently tested
   verifier. Only explicit boolean `true` is success; RPC errors/unavailability
   are not cryptographic failure. Reject incomplete or unsupported signing
   policies.
6. Recheck that identity authority and reference/draft did not change during the
   operation and that the proof has not expired. Bind the result to that
   identity revision and draft/context generation before presenting it.
7. Cache evidence and check time, not a permanent trusted-contact flag.

RPC-backed verification inherits the wallet's configured RPC trust boundary; it
is not trustless local chain verification. Preserve that distinction in
technical documentation and tests.

## Fetching, cache, and privacy requirements

The Rust backend owns fetching and verification. Use HTTPS with certificate
validation, no cross-origin redirects for native proofs, no cookies,
authorization headers, referrers, page scripts, or browser session state. Reject
all native redirects in v1 so the advertised path itself must serve the file.
Legacy redirects may be followed at most twice within the same HTTPS origin.

Reject IP literals and hosts resolving to loopback, private, link-local,
multicast, reserved/nonpublic, or local-network addresses, including IPv4-mapped
IPv6. Bind the connection to vetted DNS results to prevent DNS rebinding; verify
the connected peer and repeat policy checks for every allowed redirect. Ambient
proxy configuration must not bypass destination enforcement. Apply these rules
to all untrusted fetch inputs, including legacy profile discovery.

Proposed limits: 10-second total operation deadline for each website fetch, 64
KiB decompressed native file, 256 KiB legacy proof page, at most two concurrent
website checks, and 128 cached results. Bound legacy profile discovery
separately (1 MiB decoded profile response, finite manifest depth and request
budget) before enabling its adapter. Abort streaming at the limit; checking size
after an unbounded download is insufficient.

Cache key: account/unlock session, network/chain, canonical identity, authority
and reference revision, origin, proof URL, connection ID, and verifier version.
Success is fresh for at most 15 minutes or until expiry, whichever comes first.
Negative results back off for 60 seconds; an explicit retry can bypass that
delay within a per-origin rate limit. Do not fetch on every render or keystroke.
If current identity state cannot be established, do not newly present a cached
result as current verification; show its historical check time in details.

Clear in-memory caches on lock/account change and discard late responses after
network, identity, origin, draft, or reference changes. Do not save contact
relationships or verification state in plaintext browser storage. Never send
contact names, notes, the full contact list, wallet identifiers, or analytics to
a website. Avoid logging proof bodies and browsing relationships.

## Legacy compatibility

The current website reads older Arweave profiles linked through an identity's
`contentmap`. The website service entry is keyed by
`i3X9irKQDqHNtYxfwEzECPHRV1RzzGnkHf`; its controller proof URL is stored under
`vrsc::system.proofs.controller` / `i9TbCypmPKRpKPZDjk3YcCEZXK6wmPTXjw`.

Implement a bounded, read-only adapter for that website entry. It must establish
the identity-to-Arweave owner/transaction linkage, deterministic profile
selection, and provenance before using the URL. Do not add arbitrary remote
avatar loading, full Arweave profile editing, or hardcoded Cragorn transaction
fallbacks. Exact supported storage variants and fixtures must be recorded.

A supported legacy statement must explicitly bind the expected VerusID and
website host, use the known proof type/version, pass verification against
current authority, and be served at the claimed HTTPS origin. Arbitrary user
comments, social posts, externally hosted copies, or a valid signature over an
unrelated message are not website-control evidence. Reject ambiguous names,
claims without a website binding, duplicate matching statements, and unknown
grammars. Parse Base64 and URL delimiters correctly; do not copy the existing
regex.

Only promote a legacy variant to **Website verified** after fixture-backed
equivalence has been established. Legacy proofs without expiry disclose **Legacy
proof; no expiry** in details and still use current-authority/freshness checks.
Unknown variants remain **Verification not supported**, with an owner migration
path to the guided format. Do not downgrade a malformed native file to legacy.
Our proposed structured file will need separate adoption by verus.io; the wallet
must not promise that publishing here adds a verified badge there.

## Implementation seams and delivery

| Area                                 | Starting point and responsibility                                                                                                                                                                                                                                                              |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Identity discovery and public fields | [Profile reader](../../src-tauri/src/core/channels/vrpc/identity/profile/read.rs) and [identity types](../../src-tauri/src/types/identity.rs); add typed website reference/provenance without changing avatar/description validity.                                                            |
| Local signing                        | [Identity signature code](../../src-tauri/src/core/crypto/verus_id_signature.rs); add/test ordinary message signing and guarded draft lifecycle rather than exposing arbitrary signing to the UI.                                                                                              |
| Publication and reconciliation       | [Profile preflight](../../src-tauri/src/core/channels/vrpc/identity/profile/preflight.rs) and [intent validation](../../src-tauri/src/core/channels/vrpc/identity/profile/intent.rs); preserve unknown content, authority controls, backend-owned fees/outputs, and existing pending behavior. |
| Editor and shared presentation       | [Profile editor](../../src/lib/components/wallet/sections/identity/IdentityProfileEditor.svelte), [identity service](../../src/lib/services/identityLinkService.ts), and Contacts integration; share translated help, state mapping, and verification results.                                 |
| Remote proof boundary                | New focused backend module for normalized references, safe fetching, proof codecs, verifier, and cache; do not put arbitrary URL fetch commands in frontend components.                                                                                                                        |

Deliver in these slices, with the last slice required for the end-to-end
feature:

1. Freeze/prove native storage and signature fixtures, typed states, URL policy,
   and verifier; inspect real legacy examples without using them as golden tests
   that require external uptime.
2. Add native discovery, bounded legacy compatibility, and shared read-only
   website presentation. Publish precise supported/unsupported format coverage.
3. Add guarded signing, save-file support, instructional flow, hosted-file
   checking, and encrypted resume drafts.
4. Add reviewed publication, confirmation/reorg reconciliation, replacement,
   removal markers, renewal, and complete native user-journey verification.

The existing publisher currently restricts writes to supported single-key
VRSCTEST identities. Keep that gate until network-specific storage, fee,
signing, and reconciliation evidence justify mainnet support; hiding the gate is
not an implementation. Mainnet read and write capabilities must be explicit and
independently tested. An implementation may ship a clearly labelled testnet
preview first, but must not call the VRSC end-to-end feature complete then.

## Acceptance and release evidence

- [ ] A first-time owner can complete enter → sign → save → upload → check →
      review → publish → confirm using only the in-app guidance, without a CLI,
      private-key export, or prior knowledge of `.well-known`.
- [ ] The native file dialog, cancel/retry, website-manager instructions, hidden
      folder guidance, deployment step, and unsupported-host exit work.
- [ ] A second wallet independently discovers and verifies the confirmed website
      without the first wallet's draft/cache; Contacts membership is irrelevant.
- [ ] Existing proof entries survive addition/renewal; changed files and
      conflicts cannot be silently overwritten by the generated
      instructions/file.
- [ ] Golden vectors cover exact signed bytes and full-node interoperability on
      VRSC/VRSCTEST, including slash-containing Base64, wrong
      signer/chain/origin, copied proofs, duplicate keys/matches, unsupported
      versions, and expiry.
- [ ] Native record publication/removal round-trips through real node content
      APIs with provenance, unknown content preserved, and no legacy
      resurrection.
- [ ] Tests cover historical-valid/current-invalid signatures, revocation,
      recovery, unsupported multisig, changed authority/reference during
      checking, renewal, removal, and return to a previously used domain.
- [ ] Network tests cover rebinding, private/mapped IPs, redirects, TLS
      failures, authentication, HTML rewrites, oversized/compressed bodies,
      deadlines, offline/RPC failure, concurrency, and privacy-preserving
      request headers.
- [ ] Integration tests cover stale preflights, insufficient fees, ambiguous
      broadcast, restart/resume, reorgs, confirmation followed by hosting
      failure, account/network switch, lock, cancellation, and late-result
      suppression.
- [ ] A failed or slow website check never blocks contact saving, avatar
      display, recipient resolution, payment review, or sending; it never
      changes a saved destination or implies contact trust.
- [ ] Keyboard/screen-reader status, focus restoration, reduced motion,
      translated copy, long domains, and both themes are verified at the
      supported desktop sizes. Inspect the real rendered result, not only static
      component tests.
- [ ] Record controlled native testnet evidence for signing, file export, public
      HTTPS hosting, verification, broadcast, confirmation, second-wallet
      discovery, renewal, replacement, and removal. Use the repository's
      disposable-wallet runbook for authorized testnet signing. Separate this
      evidence from synthetic UI tests and public read-only research.
- [ ] Before declaring mainnet complete, record supported RPC/network behavior
      and explicitly authorized mainnet publication evidence; no real-funds
      transaction or public deployment is authorized by this document alone.

## Research basis and validation gates

Research was performed on 2026-09-19 against wallet commit
`316b104017ef73af2d194d575459449a2a632634` and the local `verus-website-2025-1`
checkout at `0d2c8ee4ffde6d393e4b8617d3d4e0d47227c7b2`. Relevant paths under the
website checkout, not this repository:

- `src/features/verusid_search/server/verification_check.ts`
- `src/features/verusid_search/lib/utils.ts`
- `src/features/verusid_search/server/fetch_verus_profile.ts`
- `src/data/vdxfid/servicesJSON.ts` and `src/data/vdxfid/proofsJSON.ts`
- `src/features/verify/server/verify-message.ts`

The [Cragorn profile](https://verus.io/verusid-search/cragorn@) linked
`https://cragslist.hopto.org`; its
[public Arweave record](https://arweave.net/WSrNhh1VQ_c18hHKhATXw0wEWoMoeaYiYcGkvNNYNfk)
was inspected. The website hostname did not resolve from the research
environment and the profile rendered unknown verification status. This is not a
working website-verification fixture. A separate public Cragorn-to-Crag identity
statement passed RPC verification with both historical and current authority;
that result proves RPC capability, not website verification.

The existing website verifier does not explicitly compare the signed website
claim to the fetched host, and omits the current-authority argument. Its parser
failed local examples with URL colons and truncated slash-containing signatures.
Verus's
[official `verifymessage` implementation](https://github.com/VerusCoin/VerusCoin/blob/master/src/rpc/misc.cpp)
documents `checklatest`; recheck the supported release during implementation.

Before enabling writes, resolve these engineering gates in this spec or a linked
implementation plan: byte-exact full-node vectors; the versioned website
descriptor/storage wrapper on both networks; bounded legacy provenance and
supported grammars; practical DNS/peer enforcement on each desktop platform; and
real host upload/resume testing. The proposed `.well-known` path, file schema,
lifetimes, and cache limits may change at that gate, with all user help and
fixtures updated together. Do not silently publish an experimental wire format
while those decisions remain unresolved.
