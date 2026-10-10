# Lite wallet language

Shared product language for contacts, VerusID profiles, and requests from
websites and apps.

## Language

### Contacts and profiles

**Contact**: A private record of a person, organization, or destination saved by
the wallet user. A contact can have receiving addresses and optional associated
VerusIDs. _Avoid_: Profile or VerusID as synonyms for a contact.

**Contact name**: The resolved VerusID name for an identity-backed contact, such
as `alex.example@`. It is not a separately editable nickname. A contact without
a VerusID has a local name chosen by the wallet user, such as "Mom".

**VerusID**: A blockchain identity with its own name and identity address,
identified within its network and chain. Associating one with a contact does not
grant control of that identity.

**Public profile**: Information published for a VerusID, presented with that
identity's name: avatar, short description, and an optional website connection
under the verified-websites extension. Its publisher controls this information
independently of other people's saved addresses and private notes. _Avoid_:
Wallet profile or account as synonyms for a public profile.

**Website connection**: A public association between a VerusID and a specific
website address. Verification establishes that connection at a recorded time,
not the identity holder's real-world identity or trustworthiness.

**Website proof**: A public statement signed by a VerusID and hosted on the
website it identifies. _Avoid_: Verification badge as a synonym for the proof
itself.

**Profile preview**: A compact view of a VerusID's public profile available
beside an identity mention. _Avoid_: Tooltip for a preview that contains
actions.

**Saved contact state**: A local relationship in the active wallet. The wallet
shows an unsaved ID with a neutral mark and a saved one with its avatar and a
blue name. Saving from the preview is one action; the green check confirms a
successful local save, not verification of the ID's owner.

**Receiving address**: A destination on a specified network to which a contact
can receive a payment. A manually saved receiving address and one published by a
VerusID have different sources, even if their values match.

**Contact profile**: The selected VerusID's name, avatar, and description shown
with the contact's manually saved receiving addresses. Other associated
identities retain their own names and profiles wherever shown individually.

### Requests

**Request**: A message from a website or app that asks the wallet to do one or
more things. Its requester signs it; only a request whose details are all
invoices, gift card claims, or wallet backups may be unsigned. _Avoid_:
Deeplink, envelope, generic request in UI copy.

**Detail**: One thing a request asks for, such as signing in, paying an invoice,
or updating a VerusID. A request holds its details in a fixed order. _Avoid_:
Part, step, item.

**Requester**: The VerusID that signed a request. _Avoid_: Signer, app.

**Responder**: The VerusID in this wallet that answers a request. _Avoid_:
Signer, active identity.

**Response**: The answer the wallet signs and returns for a request. A request
that only pays an invoice has no response. _Avoid_: Reply, result.

**Response address**: The web or app address that receives a response. A website
connection to the requester links it to that VerusID; without one it is only a
destination. _Avoid_: Site, callback, origin.

**Delivery**: Sending a response to its response address. A response is
delivered only when that address accepts it; signing alone is not delivery.
_Avoid_: Submit, callback.

**Approval**: The user's single, final consent to a request. Before approval,
the wallet signs, broadcasts, and sends nothing for that request. _Avoid_:
Confirm, accept.

**Decline**: The user's refusal of a request. The wallet signs and sends
nothing, so the requester is not told. _Avoid_: Cancel, reject.

**Impact**: What a detail can do to the user: sign as them, share their data,
spend their funds, or change control of a VerusID. _Avoid_: Weight, risk level.

**Receipt**: The record of what the wallet did for a request: what it signed,
broadcast, and delivered. _Avoid_: Result, history item.

**Credential**: Data stored encrypted in a VerusID's own on-chain content, so
that only the wallet that controls that VerusID can read it. A request can ask
the wallet to share credentials. _Avoid_: Record, attestation, document.

**Credential scope**: The VerusIDs that a credential may be shared with.
_Avoid_: Audience, permission.

**Gift card claim**: A detail that asks the wallet to move funds from a key the
request carries into the wallet. _Avoid_: Spendable key in UI copy, voucher.
