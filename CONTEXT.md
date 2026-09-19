# Lite wallet language

Shared product language for contacts and VerusID profiles.

## Language

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
