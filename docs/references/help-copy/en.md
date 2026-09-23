---
owner: lite-wallet-team
last_reviewed: 2026-09-22
---

# Wallet Help copy in English

All 29 Help articles are below, grouped as they are in the app. Edit the titles
and paragraphs directly. Keep the reference comments in place so the edits can
be matched to the right article.

Save this file, then tell Codex: “I changed the Help copy. Please apply my edits
to the app.” Editing this document does not change the app until you ask to
apply it.

[Dutch copy](./nl.md)

<!-- help-category: basics -->

## Getting started

<!-- help-article: wallet -->

### How this wallet works

Your wallet holds the keys that authorize transactions. Your balances are
recorded on the networks you use.

Creating a wallet gives you a recovery secret and a local password. The password
unlocks this installation; the recovery secret can restore access to the keys.
Keep the recovery secret somewhere you can access without this device.

A wallet can display several assets and networks. Each has its own addresses,
fees and confirmation rules. A VerusID is optional for ordinary payments.

<!-- help-article: assets -->

### Adding or hiding an asset

Manage assets controls which currencies appear in your wallet. Hiding an asset
does not move its funds.

In Wallet, choose Assets to open Manage assets. Add a supported currency, or use
Add custom asset when the currency is not listed. For custom assets, verify the
network and currency ID or token contract using a source you trust. A name or
ticker alone does not identify a currency.

Adding an asset lets the wallet display it; it does not buy the asset or create
a balance. If funds are missing, check the receiving address and network before
adding another entry.

<!-- help-article: networks -->

### Choosing the right network

The asset, destination address and receiving network must agree.

Mainnet and testnet are separate networks. Test coins are for testing and cannot
be sent as mainnet coins. An address can look valid while belonging to the wrong
network or an unsupported service.

For a payment, use the network specified by the recipient. For a cross-chain
transfer, check the destination network in the review. A familiar ticker or the
same-looking address does not guarantee that the recipient supports that asset
on that network.

<!-- help-category: payments -->

## Sending and receiving

<!-- help-article: receive -->

### Receiving a payment

Share the receiving address for the asset and network you want to receive.

Open Receive, choose the asset and receiving option, then copy the address or
show its QR code. Check the network with the sender, especially for tokens that
exist on several chains. A QR code is another way to share the payment details;
it does not confirm a payment.

You can share a public receiving address. Keep recovery phrases, private keys
and spending keys private. To check whether a payment arrived, open the asset in
Wallet and inspect its transaction history.

<!-- help-article: send -->

### Sending a payment

Review the recipient, asset, network, amount and fees before you approve a
transfer.

Choose Send from Wallet or an asset. Select the source, enter the amount and
recipient, then open Review. For supported Verus routes you can use a VerusID;
check the identity and resolved destination shown by the wallet.

The review can change the sendable amount to leave room for fees. Read the final
values before sending. Once a transaction is confirmed, the wallet cannot undo
it. Returning from Help preserves your form, but a transfer may require a fresh
review before submission.

<!-- help-article: pending -->

### My transfer has not arrived

Submission and confirmation are separate steps. Cross-chain transfers also need
processing on the destination network.

Open the asset in Wallet and check its transaction history. If a transaction ID
and explorer link are available, inspect the transaction on the correct network.
Check the recipient address and whether the receiving service requires more
confirmations.

Confirmation times depend on the network, fees and current conditions. A
confirmed source transaction does not by itself prove that a cross-chain payment
has arrived. If submission is uncertain, check the existing transaction before
sending another payment.

<!-- help-article: fees -->

### Why a transfer needs fees

Fees pay for network processing. The currency used for fees depends on the
route.

Sending an Ethereum token requires ETH on that Ethereum network for gas, in
addition to the token balance. Conversions and cross-chain transfers can include
conversion and bridge fees as well as the network fee.

Use the review to check the total debited and the currency for each fee. An
estimate can change before submission. Where a maximum fee is shown, it is an
upper bound for that fee, not a promise that the full maximum will be spent. Fee
choices, when available, affect priority; they do not guarantee a confirmation
time.

<!-- help-article: uncertain -->

### Submission could not be confirmed

The wallet may have lost the response after sending a transaction. That does not
prove the transaction failed.

Check the asset transaction history and any transaction ID already shown. Avoid
starting a duplicate payment while the outcome is unknown.

For an Ethereum transfer, use the saved transfer recovery screen when it
appears. Review the saved details and choose Continue transfer or Show submitted
transfer as offered. This follows the existing operation. If the result remains
unclear, ask for help with the network, error message and public transaction ID.

<!-- help-category: conversions -->

## Conversions and cross-chain

<!-- help-article: convert -->

### How conversions work

A conversion exchanges one currency for another through a supported Verus
currency basket.

A basket holds reserve currencies. The protocol calculates conversion prices
from its reserves and processes conversions together at consensus. The amount
you receive depends on the route, amount, reserve balances and fees.

Choose Convert, select what you pay and receive, and review the destination and
estimate. A conversion can also deliver to another supported network. Only
routes available for the chosen source and destination appear; adding a token
does not automatically make it convertible.

<!-- help-article: conversion-estimate -->

### Why the conversion amount can change

The receive amount is an estimate until the network processes the conversion.

Other conversions can change the basket reserves between your quote and
settlement. The size of your conversion also affects the price. This price
movement is often called slippage. The wallet displays the route and estimated
result before you approve.

Read the latest review, including conversion and network fees. Refresh an
expired review. The fiat value in your portfolio is a separate market estimate
and is not the rate used to settle your conversion.

<!-- help-article: cross-chain -->

### How cross-chain transfers work

A cross-chain transfer moves value from one network to another through a
supported route.

The source network first records the transfer. Proof of that transfer then needs
to be accepted and processed on the destination network. These stages can take
longer than a payment that stays on one chain.

Check the destination network and receiving currency in Review. A source
transaction ID or source confirmation does not establish destination arrival.
Use the available explorer details to inspect progress, and allow for additional
verification and any requirements of the receiving service. Availability depends
on the route and network; this article is not a live bridge-status report.

<!-- help-article: ethereum -->

### Ethereum bridge transfers and approvals

An Ethereum token bridge transfer may need a token approval before the transfer
itself.

An approval gives the bridge contract permission to use the token amount. It is
a separate on-chain operation and can incur gas. An approval transaction alone
does not mean the bridge transfer has been submitted or received. Keep enough
ETH on the source network for the required steps.

Follow the wallet review and any saved transfer recovery steps. If approval
succeeds but the next step fails, continue the saved operation when offered.
Bridge routes may be disabled or unavailable in a build or on a network; do not
assume every Ethereum token can be bridged.

<!-- help-category: identity -->

## VerusID and profiles

<!-- help-article: verusid -->

### What is a VerusID?

A VerusID is an on-chain identity with a name, an identity address and rules for
who can control it.

You can use a VerusID as a payment destination on supported routes and with
applications that support VerusID. Its primary addresses and signing rules
determine who can authorize actions. Revocation and recovery authorities provide
separate controls.

A public profile can add images and a description to the identity. Applications
choose which identity and profile features they display. A name or profile image
does not by itself prove someone is trustworthy.

<!-- help-article: link-identity -->

### Linking and unlinking a VerusID

Linking adds an existing identity to this wallet. It does not register a new
identity or transfer ownership.

Open VerusID and use Link VerusID to find an existing identity on the selected
network. The wallet checks the identity and what this wallet is allowed to do.
Seeing an identity or its balance does not necessarily mean you control it.

Unlink removes the local link. The identity, its funds and its public profile
remain on-chain. To change the identity itself, an authorized on-chain update is
required.

<!-- help-article: public-profile -->

### What becomes public in my profile?

Publishing stores your chosen profile images and description on the public Verus
blockchain.

Anyone can read published profile data. Apps that support this profile can
display it using your VerusID, so the information is not tied to this
installation. Each app decides which fields it supports.

Later changes or removals update the current profile; earlier versions remain in
blockchain history. Publish only information you intend to make public. Your
private contact notes and recovery secrets are not included in a profile update.

<!-- help-article: publish-profile -->

### Editing and publishing a profile

Edits start as a local draft. Publishing requires review, a network fee and an
authorized signature.

Open your linked VerusID and choose Edit profile when available. Add or change
the avatar, header image or description, then review the changes and cost. This
build publishes profiles for supported VRSCTEST identities only. An active
identity and a supported control setup are required.

The review may offer one update or two, depending on the images. With two
updates, the first must confirm before you review and approve the second.
Ordinary unpublished drafts last for the current unlock session. A saved
continuation after the first update has a separate recovery flow.

<!-- help-article: profile-pending -->

### Why my profile still shows the old version

The wallet keeps showing the last confirmed profile while an update is waiting
for confirmation.

Use the submitted changes and transaction details to see what was sent. A
submitted update has not yet established the new confirmed profile. If a
confirmation check fails, retry the check before publishing again.

For a two-update publication, the avatar and description can confirm before the
header. Review and approve the remaining header update when offered. Other apps
may refresh their profile data at different times, so their display can lag
behind the chain.

<!-- help-article: profile-unavailable -->

### Why a profile cannot be shown or edited

A missing profile, an unreadable profile and a profile you cannot edit are
different states.

An identity may have no public profile yet. If profile data cannot be loaded or
verified, the wallet can still show core identity details; retry when your
connection is available.

Editing requires this wallet to control a supported active identity. Mainnet
publishing, token-controlled identities and unsupported signing arrangements are
not available in this profile editor. Read the reason shown by the wallet.
Linking the identity again does not grant signing permission.

<!-- help-category: security -->

## Security and recovery

<!-- help-article: recovery -->

### What should I back up?

Keep the recovery secret for every set of funds you use, including a separate
Private Verus secret if you created or imported one.

Open Settings, then Profile and security, to view recovery information. Private
Verus recovery information is also accessible from its settings. Follow the
wallet prompts and keep an accurate offline copy. A separately imported key may
protect only its own addresses.

Anyone with a recovery phrase or spending key can potentially use the funds it
controls. Do not share it in chat, screenshots or a support form. Your local
password and your recovery secret serve different purposes; knowing the password
alone does not restore a lost installation.

<!-- help-article: forgot-password -->

### I cannot unlock my wallet

A forgotten local password cannot be recovered by the wallet or the community.

If you have the original recovery phrase or another supported recovery secret,
import it as a wallet and set a new local password. Choose the import method
that matches your backup and use the correct network. You may need to restore a
separate Private Verus secret as well.

Keep the old installation and its files until you have checked your restored
addresses and balances. Importing keys does not recreate local contacts or
notes. VerusID recovery is a separate mechanism and is only useful when the
required authority can still authorize recovery.

<!-- help-article: guard -->

### What VerusID Guard can recover

VerusID Guard helps an authorized revocation or recovery authority act on a
VerusID.

Revocation disables normal use of the identity once the revocation takes effect
on-chain. Recovery can assign new primary control to a revoked identity. Both
require the appropriate authority and an on-chain transaction; selecting an
authority does not help if you also lose access to its keys.

Guard is available from Welcome and Unlock. It does not reset a local password
or recover arbitrary Bitcoin, Ethereum or ordinary address funds. If keys are
compromised, identity recovery only addresses what that identity controls. Check
the identity, authorities, network and proposed changes before approving.

<!-- help-article: private-verus -->

### Setting up Private Verus

Private Verus uses a shielded address and needs its own recovery material and
synchronization.

Open Settings, then Private Verus. Depending on your wallet, you can reuse the
primary recovery phrase, create a new privacy recovery secret, or import one. If
you create or import a separate secret, back it up separately. The primary
wallet backup alone may not restore those private funds.

Let private synchronization finish before sending. It scans the chain for funds
the private keys can use. A new installation or restored privacy secret may need
time to find the full balance. Follow any setup or restart instructions shown by
the wallet.

<!-- help-article: private-send -->

### Private addresses and transaction memos

A shielded address protects different information from a public receiving
address. Check both ends of the transfer.

Private Verus sends can target supported shielded or transparent Verus
destinations. Sending to a public address exposes the receiving side on-chain.
Using the private source does not make a public destination private.

Private memos are supported for shielded destinations. The wallet must finish
private synchronization and prepare the transaction proof before it can submit.
Proof generation can take time. If submission becomes uncertain, check history
before trying another send.

<!-- help-article: requests -->

### Reviewing a request from an app

A request asks the wallet to perform specific actions. Check the requester and
each action before approving.

Use Open request for a supported request, or open a supported wallet link. The
wallet verifies the request and shows the actions it understands. These can
include authentication and identity changes; some actions also require funding
and a fee.

A valid signature identifies the signer, but does not make the request
desirable. Check the identity, permissions, destination and changes yourself.
Reject an unexpected request. The Apps section is not currently an app
directory, and Activity has no transaction history in this version; inspect an
asset for its transactions.

<!-- help-category: data -->

## Wallet data and settings

<!-- help-article: contacts -->

### Contacts and public profiles

Contacts store recipients and private notes in this wallet. A linked public
profile comes from the VerusID.

Save a contact with a supported address, or associate a VerusID. A public
profile can help you recognize the identity; your own note stays local. Check
the selected receiving address and network each time you pay.

Deleting a contact removes the local entry. It does not delete the person's
VerusID, public profile or blockchain transactions. Contacts are encrypted in
this installation and are not recreated just by importing the wallet recovery
phrase on another device.

<!-- help-article: watchlist -->

### What the Watchlist shows

The Watchlist follows public holdings of supported Verus addresses and
identities.

Adding an address or identity lets you inspect its available public balances. It
does not import a key or grant permission to spend. Watched funds are separate
from funds this wallet controls.

Results depend on the supported networks and the data that can be reached. An
unavailable or partially loaded result is not proof of a zero balance. Private
balances cannot be discovered from a public address lookup. Removing a watched
entry changes your local list only.

<!-- help-article: local-data -->

### What stays on this device?

Keys and local wallet records serve a different purpose from public blockchain
records.

Contacts, private notes, watchlist choices and display settings are local wallet
data. Publishing a VerusID profile does not publish these records. Importing a
recovery secret restores the keys it represents, not a copy of every local
setting or contact.

Public transactions and published profile history remain on their networks.
Locking the wallet protects local access; it does not hide information that is
already public. Keep the original wallet files until you have verified any move
to a new installation.

<!-- help-article: settings -->

### Display, language and wallet locking

Settings controls appearance, display currency, language, auto-lock and access
to recovery information.

Use Display and language to choose a theme, language or fiat display currency.
Changing the display currency changes estimates in the interface; it does not
convert your funds.

Profile and security includes the inactivity lock interval and recovery access.
Lock ends the current unlocked session, so finish or safely leave your work
first. Private Verus has its own setup page. About and support shows the app
version, which is useful when reporting a problem.

<!-- help-article: support -->

### Getting help from the community

Describe what you tried, what you expected and the message the wallet showed.

Include the app version, network and asset. A public transaction ID can help
with a transfer problem, but it can also reveal transaction details. Review
screenshots before sharing them and remove anything private.

Use Ask the community to open the Verus Discord. Never send a recovery phrase,
private key, spending key or password to someone offering help. Core Help
articles work offline; community links, live balances and network lookups
require a connection.

<!-- help-controls -->

## Help labels and messages

You can also edit the Text column below. Keep the Reference column and the
`{count}` placeholder unchanged.

| Reference                       | Text                                     |
| ------------------------------- | ---------------------------------------- |
| `helpCenter.title`              | Help                                     |
| `helpCenter.search`             | Search help                              |
| `helpCenter.searchPlaceholder`  | Search questions or topics               |
| `helpCenter.home`               | All topics                               |
| `helpCenter.suggested`          | Start here                               |
| `helpCenter.topics`             | Help topics                              |
| `helpCenter.backWallet`         | Back to wallet                           |
| `helpCenter.backWelcome`        | Back to welcome                          |
| `helpCenter.backUnlock`         | Back to sign in                          |
| `helpCenter.backResults`        | Back to results                          |
| `helpCenter.backArticle`        | Back to article                          |
| `helpCenter.backTopics`         | Back to topics                           |
| `helpCenter.results`            | Search results                           |
| `helpCenter.resultOne`          | 1 article                                |
| `helpCenter.resultCount`        | {count} articles                         |
| `helpCenter.noResults`          | No matching articles                     |
| `helpCenter.noResultsHint`      | Try a shorter search, or choose a topic. |
| `helpCenter.related`            | Related articles                         |
| `helpCenter.community`          | Ask the community                        |
| `helpCenter.communityHint`      | Opens Verus Discord                      |
| `helpCenter.source.identity`    | More about VerusID                       |
| `helpCenter.source.conversions` | More about Verus conversions             |
| `helpCenter.source.bridge`      | More about the Verus-Ethereum Bridge     |
| `help.link.needHelp`            | Get help                                 |
| `common.clearSearch`            | Clear search                             |
