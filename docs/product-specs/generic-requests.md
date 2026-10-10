---
owner: lite-wallet-team
last_reviewed: 2026-10-10
---

# Requests from websites and apps

- Status: approved direction; not implemented. Delivery is tracked in
  [the plan](../plans/active/generic-requests-guided-flow.md).
- Parity reference: VerusCoin/Verus-Mobile `dev-asf-parent` at `7d74cd6a`
  (2026-10-08). Parity covers protocol and behaviour, not the flow
  ([ADR 0003](../adr/0003-mobile-parity-scope.md)).
- Terms: the "Requests" section of [`CONTEXT.md`](../../CONTEXT.md).
- Decisions: [ADR 0001](../adr/0001-approval-before-any-action.md) (Approval
  before any action), [ADR 0002](../adr/0002-request-parsing-boundary.md)
  (parsing boundary), [ADR 0003](../adr/0003-mobile-parity-scope.md) (parity
  scope).
- Design exploration: variation B, "Guided flow", of the
  [design review page](https://claude.ai/artifact/6HjeGM5qEkBXnYdBidPVXt).

## Goal

The desktop wallet answers every request that Verus Mobile answers, in one
guided flow. Each detail type is a module of one shared system. No detail type
is hidden behind an experimental setting.

## Supported details

| Detail                       | Impact          | The user decides                            | The wallet returns                                        |
| ---------------------------- | --------------- | ------------------------------------------- | --------------------------------------------------------- |
| Sign in (authentication)     | Signs as you    | The responder                               | A sign-in response                                        |
| New VerusID (provisioning)   | Signs as you    | To ask for the offered VerusID              | A signed provisioning request to the provisioning address |
| Update identity              | Changes control | To accept the changes, and who pays the fee | A broadcast update and its transaction ID                 |
| Pay invoice (VerusPay)       | Spends funds    | Which source pays                           | A broadcast payment; no response detail                   |
| Sign data (data packet)      | Signs as you    | Consent to every item and statement         | One signature for each item and statement                 |
| Encryption key (app)         | Shares data     | The responder; approve or decline           | A viewing key and private address made for that app       |
| Share data (credentials)     | Shares data     | To share the matching credentials           | The matching credentials, encrypted to the requester      |
| Gift card claim              | Spends funds    | To claim; a password when the claim has one | Funds moved into the wallet                               |
| Legacy sign in               | Signs as you    | The responder                               | A LoginConsent response                                   |
| Legacy invoice link (v3, v4) | Spends funds    | Which source pays                           | A broadcast payment                                       |

Wallet backup details are rejected with "Wallet backups to NFC cards need Verus
Mobile." A legacy sign-in request is accepted only when its sole permission is
identity view.

## Acceptance rules

- The wallet verifies every request before it shows it. Rust checks the
  signature, the network, the expiry, and the order of details.
- The order of details follows `verusid-ts-client` at the `dev-asf-parent` pin.
  - Sign in appears at most once, and first.
  - At most one invoice or identity update in total, and always last.
  - New VerusID and Encryption key each need a Sign in before them.
  - Sign in plus one invoice is allowed. Share data and Sign data may combine.
- A request may be unsigned only when every detail is an invoice, a gift card
  claim, or a wallet backup, and it names no delegated app identity.
- Share data and Sign data need a signed request.
- When the request names a delegated app identity, the signer must be a
  single-signature VerusID that this wallet controls.
- The wallet rejects a request for another network than the open wallet's.
- Links of 2046 characters or more are rejected.
- Compact addresses in either form are accepted: i-address or fully qualified
  name.

## Intake

- **Sources:** an OS `verus://` link, or a paste into Open request. Paste
  accepts a `verus://` link, the raw QR text string, or hex. There is no QR
  image intake; websites offer a copy-link option.
- **One request at a time, in memory only.** Nothing about an unopened request
  is written to disk.
- **When it opens:** as soon as the wallet is unlocked, Help is closed, and no
  submission is running. Until then it waits in memory. Send and Convert drafts
  are kept.
- **A second link:** before Approval, it replaces the open request. After
  Approval, it is ignored with "Finish the current request, then open the new
  link again."

## Flow

The request takes over the window. The top bar shows "Request from" and the
response address, plus a close button. Close declines the request.

### Layout

- **Single screen:** a request with one detail whose impact is "Signs as you".
  It shows the requester header, the responder, the detail, the delivery line,
  Decline, and the primary action.
- **Rail:** every other request. The rail shows the requester header, the
  responder once chosen, one step per detail, Confirm, Done, and the time left.
  The main column is at most 560px wide. The footer has Back or Decline on the
  left and Continue on the right.

### Requester header

- The requester shows as an identity mention with its profile preview.
- "Signed by" names the requester after the signature check. An unsigned invoice
  shows "Not signed".
- When the request names a delegated app identity, the header names both.
- When the request opens, the wallet runs the existing website-connection check
  against the response address. "Connected to" shows only after it succeeds.
  While it runs, or when it fails, nothing shows and nothing blocks.
- **Expiry:** one format. Time-based: "9 min left". Block-based: "about 2 hours
  left".

### Steps per detail

- **Sign in:**
  - The responder picker lists only VerusIDs in the open wallet that meet the
    request's rules (required ID, system, parent).
  - The step lists what the website learns and the requirements.
  - When no VerusID qualifies, it offers Link VerusID, and New VerusID when the
    request offers one.
- **New VerusID:** shows the offered name, the issuer, and the network. After
  Approval the wallet sends the provisioning request and saves the job. The sign
  in finishes after the user links the new VerusID.
- **Update identity:** two steps.
  1. **Changes.** Content and profile changes. High-risk changes (primary
     addresses, revocation, recovery, status, clearing content) come with an
     outcome summary and an acknowledgement. Continue needs the acknowledgement.
  2. **Network fee.** The user picks who pays: the primary address or a linked
     VerusID on the request's system, never a watched address. Nothing is
     preselected. The fee shows after the choice.
- **Pay invoice:**
  - Shows the exact amount, the currency, the destination, and the expiry.
  - The user picks the source. Nothing is preselected, and the fee shows after
    the choice.
  - The total is the amount plus the fee. The amount is never lowered to cover
    the fee.
  - Tagged invoices, private-address destinations, unpayable invoices, missing
    destinations, unknown currencies, preconvert invoices after launch, and
    expired invoices are rejected with a clear reason.
  - Conversion comes in a second release.
- **Sign data:**
  - Lists every data item and statement, each with its own consent checkbox.
    Continue needs consent to all of them.
  - Data sent to the user (transmittal) is rejected.
- **Encryption key:**
  - The responder is chosen here, and no sign-in response is sent.
  - Shows the app, the key number, and what is shared. A requested spending key
    gets a strong warning.
  - Without private addresses the step shows "Set up private addresses first"
    and a button that starts the existing setup.
- **Share data:**
  - Reads the responder's on-chain content and decrypts it. It shows only
    credentials whose key was requested and whose scope includes the app
    identity, or the signer when no app identity is named.
  - Missing keys are listed.
  - A lookup or decryption failure is an error with Try again, never an empty
    answer.
- **Gift card claim:** shows what will be claimed and asks for the claim
  password when there is one. The request is saved encrypted until the claim
  finishes. The phase 5 design fixes the screen.

### Confirm and Approval

- Confirm lists one line per detail, each with its impact, plus the delivery
  line.
- The primary action names the result, for example "Sign in and update".
- Approval needs no password, as with Send.

## After Approval

1. **On-chain transaction.** The identity update or the payment is broadcast.
2. **Signing.** The response is signed.
3. **Delivery.** The response is delivered.

- A failed stage shows its own state and retries alone. The wallet never
  broadcasts twice for one open request.
- Auto-lock pauses while the stages run.
- Done lists what was signed, broadcast, and delivered as separate lines.
- **Failed delivery:** "Signed, not delivered", with Try again and Copy
  response, and "Open site instead" when a redirect address exists.
- **Decline:** "Nothing was signed or sent."
- **Lock before Approval:** the request stays in memory and reopens after
  unlock.
- **Manual lock or quit after a broadcast:** the response is lost, as on mobile.
  Reopening the link can offer that broadcast again.

## Response rules

- **Fields:** the response carries the request ID, the request hash (SHA-256 of
  the request), the creation time, the handled-by field, and the testnet flag,
  as mobile sets them.
- **Signer:** one responder signs every detail. A detail that needs a different
  identity fails the request.
- **Sign-in response:** sent next to every detail except Encryption key. It
  echoes the request ID when the request has one, else the sign-in detail's ID.
  Other details echo their own ID.
- **Encryption:** when the request names an address to encrypt to, every detail
  is encrypted into one descriptor and the response is flagged as encrypted.
  Share data needs this address. A plain spending key is never sent unencrypted.
- **Data signatures:** each item and statement is signed separately. The hash is
  SHA-256 of its bytes, and the signature covers the height-bound identity hash.
- **Encryption keys:** derived from the extended spending key, with plain ID
  byte order. The IVK is in display order. An encrypted reply is a data
  response.
- **Credentials written by an update:** before any network call, each entry is
  encrypted to the identity's own private address. That address must come from
  this wallet's private-address seed. Plain credential values never leave the
  device.
- **Delivery:** POST first (raw bytes, `application/octet-stream`), else
  redirect. HTTPS only in release builds; plain HTTP only in debug builds. POST
  and provisioning calls time out after 30 seconds.

## Copy and accessibility

- All text uses `i18n.t(...)` keys in English, Dutch, German, and Spanish. Help
  articles update in English and Dutch, the languages Help supports.
- Rust warning codes map to translated, plain-language copy. Raw codes never
  show.
- Sentence case, the normal arrow cursor, visible focus, and both themes at the
  920×620 minimum window.

## Out of scope

- Request history and receipts after closing.
- A credential viewer.
- QR image intake.
- Multisig requesters.
- Other PBaaS chains than the wallet's network.
- Conversion before phase 2's second release.
- Mainnet acceptance runs, which come later.

## Open protocol questions

- **GR3-P1:** whether the sign-in response should echo the request ID or the
  sign-in detail's ID. Desktop follows `dev-asf-parent` (the request ID when
  present) until upstream decides.
