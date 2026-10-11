---
owner: lite-wallet-team
last_reviewed: 2026-10-11
---

# Requests from websites and apps

- Status: implemented and verified on testnet, except the items the
  [plan](../plans/active/generic-requests-guided-flow.md) lists as not done.
- Parity reference: VerusCoin/Verus-Mobile `dev-asf-parent` at `7d74cd6a`
  (2026-10-08). Parity covers protocol and behaviour, not the flow
  ([ADR 0003](../adr/0003-mobile-parity-scope.md)).
- Terms: the "Requests" section of [`CONTEXT.md`](../../CONTEXT.md).
- Decisions: [ADR 0001](../adr/0001-approval-before-any-action.md) (Approval
  before any action), [ADR 0002](../adr/0002-request-parsing-boundary.md)
  (parsing boundary), [ADR 0003](../adr/0003-mobile-parity-scope.md) (parity
  scope).
- Design exploration: variation B, "Guided flow", of the
  [design review page](https://claude.ai/artifact/6HjeGM5qEkBXnYdBidPVXt). The
  shell (requester, rail, Confirm, Done) follows variation A, "Calm rail", of
  the
  [shell variations canvas](https://claude.ai/artifact/6J64SoTj18C6urGVdA5fgZ).

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
identity view and it shares no context, as on mobile.

- **Legacy links** use the form `<wallet scheme>://x-callback-url/<key>/...`,
  where the wallet scheme is `i5jtwbp6zymeay9llnraglgjqgdrffsau4`. A `verus://`
  link is always a request.
- **Legacy sign in** checks the requester's signature over its challenge. The
  response is a signed `LoginConsentResponse` that embeds the request. It goes
  to the request's redirect address when it has one, else to its webhook as
  JSON, as on mobile.
- **Legacy sign in with a new VerusID:** when the challenge's provisioning
  entries name a provisioning webhook, the picker also offers New VerusID, as
  mobile's "Request VerusID" does. The first entry of each key gives the
  webhook, system, parent, and name. The provisioning request uses the login's
  challenge ID and goes to that webhook. After the user links the new VerusID,
  the login opens again to finish the sign in.
- **Legacy invoices** pay as invoices do. A signed invoice must verify for its
  signer. No response is sent.

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

- **Sources:** an OS `verus://` or legacy link, or a paste into Open request.
  Paste accepts a `verus://` link, a legacy link, the raw QR text string, or
  hex. There is no QR image intake; websites offer a copy-link option.
- **One request at a time, in memory only.** Nothing about an unopened request
  is written to disk.
- **When it opens:** as soon as the wallet is unlocked, Help is closed, and no
  submission is running. Until then it waits in memory. Send and Convert drafts
  are kept.
- **A second link:** before Approval, it replaces the open request. After
  Approval, it is ignored with "Finish the current request, then open the new
  link again."

## Flow

The request takes over the window. The top bar holds only the close button.
Close declines the request. The requester is named once, in the rail or in the
single-screen header.

### Layout

- **Single screen:** a request with one detail whose impact is "Signs as you".
  It shows the requester header, the responder, the detail, the delivery line
  when it adds something (see "Confirm and Approval"), Decline, and the primary
  action.
- **Rail:** every other request. The rail runs from the top of the window, like
  the wallet sidebar. It shows the requester, one step per detail, Confirm, and
  the time left. A finished step shows what was chosen in it, such as the
  VerusID that answers. Done is a result, not a step. The main column is at most
  560px wide. The footer has Back or Decline on the left and Continue on the
  right.

### Requester header

- The signing VerusID shows with its own avatar and its name, as a mention with
  its profile preview. A quiet check after the name says the signature was
  checked. In the rail, the site it answers to shows below the name.
- A request that is not signed shows "Not signed" in amber. A gift card has no
  requester, so the rail shows none.
- **Colour:** a normal state gets no colour. Not signed, an expiry under 10
  minutes, and a high-risk change are amber. Blue is only for the main action.
- When the request names a delegated app identity, the header names both.
- When the request opens, the wallet runs the existing website-connection check
  against the response address. "Connected to" shows only after it succeeds.
  While it runs, or when it fails, nothing shows and nothing blocks.
- **Expiry:** one format, in one place: the single-screen header or the foot of
  the rail. Time-based: "9 min left". Block-based: "about 2 hours left". It
  turns amber under 10 minutes and is hidden after Approval finishes.

### Steps per detail

- **Sign in:**
  - The responder picker lists only VerusIDs in the open wallet that meet the
    request's rules (required ID, system, parent).
  - The step says what the website learns. It lists requirements only when they
    limit the choice: a required system that is the open wallet's network is not
    listed, because the wallet refuses other networks earlier.
  - The picker always offers Link VerusID. It also offers New VerusID when the
    request offers one, next to the linked VerusIDs, as on mobile.
- **New VerusID:** shows the name once it is known, the issuer, and the network.
  An offered VerusID or name that exists on chain fixes the name to that
  VerusID's name. Otherwise the user types a name under the request's parent;
  the wallet checks it and accepts only a free one. This is mobile's rule
  (`ProvisionIdentityForm.js`). After Approval the wallet signs the provisioning
  request with its address key (a bare recoverable signature over the challenge
  hash, as `signVerusIdProvisioningRequest` makes it), sends it to the request's
  provisioning address, and saves the job. The challenge ID is the request's own
  ID, else the sign-in detail's ID. The sign in finishes after the user links
  the new VerusID.
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
  - Each credential has its own consent checkbox. Continue needs consent to all
    of them.
  - Missing keys are listed.
  - A failed read of the identity's content is an error with Try again, never an
    empty answer. As on mobile, a stored value that does not decrypt with the
    identity's key is skipped: it cannot hold a credential this wallet wrote.
  - When nothing matches, the step says so. If the request asks for nothing
    else, nothing can be sent, and the user declines, as on mobile.
  - Only requests for whole credentials are accepted, as on mobile. They must
    name an address to encrypt the answer to.
- **Gift card claim:**
  - Asks for the card's password when it has one. A wrong password is refused.
  - Rust reads the card's key and shows what it holds on the wallet's chain:
    each currency, and each VerusID the key controls.
  - Claiming moves each VerusID to the wallet's primary address, with one
    signature and the wallet's private address (or none), paid from the card.
    Then everything else goes to the wallet's primary address in one
    transaction. Revocation and recovery authorities do not change; a warning
    says so when they are another VerusID.
  - A VerusID that the claim cannot move (not active, or more than one
    signature) blocks the claim, as on mobile.
  - The key and the signed transactions stay in memory with the open request.
    Try again sends only the transactions that were not sent yet. After a lock
    or quit, opening the card again claims what is left. Saving the request
    encrypted until the claim finishes is not built yet.
  - No response is sent.

### Confirm and Approval

- Confirm lists one line per detail. Only a line with a high-risk change is
  marked, with an amber "High risk".
- The delivery line shows only when it adds something: no response is sent, the
  site opens in the browser, or the request goes to the issuer of a new VerusID.
  A POST to the requester's own address needs no line, because the rail or the
  header already names that site.
- The primary action names the result, for example "Sign in and update".
- Approval needs no password, as with Send.

## After Approval

1. **On-chain transaction.** The identity update or the payment is broadcast.
2. **Signing.** The response is signed.
3. **Delivery.** The response is delivered.

- A failed stage shows its own state and retries alone. The wallet never
  broadcasts twice for one open request.
- Auto-lock pauses while the stages run.
- Done lists what was signed, broadcast, and delivered as separate lines. A
  finished stage is quiet; a failed stage is amber.
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
  byte order. The IVK is in the curve's own byte order, as the daemon and Verus
  Mobile's native module return it. An encrypted reply is a data response.
- **Sign data inside an update** (the update's sign data map) is refused as not
  supported on desktop yet. Verus Mobile has the daemon sign it.
- **Credentials written by an update:** before any network call, each entry is
  encrypted to the identity's own credential address, which the wallet derives
  from its private-address seed. The identity's private address must come from
  that seed. Each entry is stored under a key hashed from that address's viewing
  key, as on mobile. Plain credential values never leave the device.
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
