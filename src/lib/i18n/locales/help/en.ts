export const helpEn = {
  'helpCenter.unlockRequired': 'Unlock your wallet to open this screen.',
  'helpCenter.title': 'Help',
  'helpCenter.homeTitle': 'What do you need help with?',
  'helpCenter.backHelp': 'Back to Help',
  'helpCenter.backCategory.basics': 'Back to getting started',
  'helpCenter.backCategory.payments': 'Back to sending and receiving',
  'helpCenter.backCategory.conversions': 'Back to conversions and cross-chain',
  'helpCenter.backCategory.identity': 'Back to VerusID and profiles',
  'helpCenter.backCategory.security': 'Back to security and recovery',
  'helpCenter.backCategory.data': 'Back to wallet data and settings',
  'helpCenter.search': 'Search help',
  'helpCenter.searchPlaceholder': 'Search questions or topics',
  'helpCenter.home': 'All topics',
  'helpCenter.suggested': 'Start here',
  'helpCenter.topics': 'Help topics',
  'helpCenter.backWallet': 'Back to wallet',
  'helpCenter.backWelcome': 'Back to welcome',
  'helpCenter.backUnlock': 'Back to sign in',
  'helpCenter.backResults': 'Back to results',
  'helpCenter.backArticle': 'Back to article',
  'helpCenter.backTopics': 'Back to topics',
  'helpCenter.results': 'Search results',
  'helpCenter.resultOne': '1 article',
  'helpCenter.resultCount': '{count} articles',
  'helpCenter.noResults': 'No matching articles',
  'helpCenter.noResultsHint': 'Try a shorter search or different words.',
  'helpCenter.related': 'Related articles',
  'helpCenter.community': 'Ask the community',
  'helpCenter.communityHint': 'Opens Verus Discord',
  'helpCenter.source.identity': 'More about VerusID',
  'helpCenter.source.conversions': 'More about Verus conversions',
  'helpCenter.source.bridge': 'More about the Verus-Ethereum Bridge',
  'helpCenter.category.basics': 'Getting started',
  'helpCenter.category.payments': 'Sending and receiving',
  'helpCenter.category.conversions': 'Conversions and cross-chain',
  'helpCenter.category.identity': 'VerusID and profiles',
  'helpCenter.category.security': 'Security and recovery',
  'helpCenter.category.data': 'Wallet data and settings',
  'helpCenter.article.wallet.title': 'How this wallet works',
  'helpCenter.article.wallet.summary':
    'Your wallet holds the keys that authorize transactions. Your balances are recorded on the networks you use.',
  'helpCenter.article.wallet.body':
    'Creating a wallet gives you a recovery secret and a local password. The password unlocks this installation; the recovery secret can restore access to the keys. Keep the recovery secret somewhere you can access without this device.\n\nA wallet can display several assets and networks. Each has its own addresses, fees and confirmation rules. A VerusID is optional for ordinary payments.',
  'helpCenter.article.wallet.keywords': 'self custody account new create import beginner keys',
  'helpCenter.article.assets.title': 'Adding or hiding an asset',
  'helpCenter.article.assets.summary':
    '[[manage-assets|Manage assets]] controls which currencies appear in your wallet. Hiding an asset does not move its funds.',
  'helpCenter.article.assets.body':
    'In Wallet, choose Assets to open Manage assets. Add a supported currency, or use Add custom asset when the currency is not listed. For custom assets, verify the network and currency ID or token contract using a source you trust. A name or ticker alone does not identify a currency.\n\nAdding an asset lets the wallet display it; it does not buy the asset or create a balance. If funds are missing, check the receiving address and network before adding another entry.',
  'helpCenter.article.assets.keywords':
    'coin token erc20 contract add custom hide remove portfolio',
  'helpCenter.article.networks.title': 'Choosing the right network',
  'helpCenter.article.networks.summary':
    'The asset, destination address and receiving network must agree.',
  'helpCenter.article.networks.body':
    'Mainnet and testnet are separate networks. Test coins are for testing and cannot be sent as mainnet coins. An address can look valid while belonging to the wrong network or an unsupported service.\n\nFor a payment, use the network specified by the recipient. For a cross-chain transfer, check the destination network in the review. A familiar ticker or the same-looking address does not guarantee that the recipient supports that asset on that network.',
  'helpCenter.article.networks.keywords':
    'mainnet testnet vrsctest sepolia wrong network address chain',
  'helpCenter.article.receive.title': 'Receiving a payment',
  'helpCenter.article.receive.summary':
    'Share the receiving address for the asset and network you want to receive.',
  'helpCenter.article.receive.body':
    'Open [[receive|Receive]], choose the asset and receiving option, then copy the address or show its QR code. Check the network with the sender, especially for tokens that exist on several chains. A QR code is another way to share the payment details; it does not confirm a payment.\n\nYou can share a public receiving address. Keep recovery phrases, private keys and spending keys private. To check whether a payment arrived, open the asset in Wallet and inspect its transaction history.',
  'helpCenter.article.receive.keywords': 'deposit receive qr code receiving address payment',
  'helpCenter.article.send.title': 'Sending a payment',
  'helpCenter.article.send.summary':
    'Review the recipient, asset, network, amount and fees before you approve a transfer.',
  'helpCenter.article.send.body':
    'Choose [[send|Send]] from Wallet or an asset. Select the source, enter the amount and recipient, then open Review. For supported Verus routes you can use a VerusID; check the identity and resolved destination shown by the wallet.\n\nThe review can change the sendable amount to leave room for fees. Read the final values before sending. Once a transaction is confirmed, the wallet cannot undo it. Returning from Help preserves your form, but a transfer may require a fresh review before submission.',
  'helpCenter.article.send.keywords': 'send transfer recipient address review max amount cancel',
  'helpCenter.article.pending.title': 'My transfer has not arrived',
  'helpCenter.article.pending.summary':
    'Submission and confirmation are separate steps. Cross-chain transfers also need processing on the destination network.',
  'helpCenter.article.pending.body':
    'Open the asset in Wallet and check its transaction history. If a transaction ID and explorer link are available, inspect the transaction on the correct network. Check the recipient address and whether the receiving service requires more confirmations.\n\nConfirmation times depend on the network, fees and current conditions. A confirmed source transaction does not by itself prove that a cross-chain payment has arrived. If submission is uncertain, check the existing transaction before sending another payment.',
  'helpCenter.article.pending.keywords':
    'stuck pending waiting slow missing transfer not arrived confirmation history activity',
  'helpCenter.article.fees.title': 'Why a transfer needs fees',
  'helpCenter.article.fees.summary':
    'Fees pay for network processing. The currency used for fees depends on the route.',
  'helpCenter.article.fees.body':
    'Sending an Ethereum token requires ETH on that Ethereum network for gas, in addition to the token balance. Conversions and cross-chain transfers can include conversion and bridge fees as well as the network fee.\n\nUse the review to check the total debited and the currency for each fee. An estimate can change before submission. Where a maximum fee is shown, it is an upper bound for that fee, not a promise that the full maximum will be spent. Fee choices, when available, affect priority; they do not guarantee a confirmation time.',
  'helpCenter.article.fees.keywords':
    'gas eth insufficient funds fee maximum economy standard expensive cost',
  'helpCenter.article.uncertain.title': 'Submission could not be confirmed',
  'helpCenter.article.uncertain.summary':
    'The wallet may have lost the response after sending a transaction. That does not prove the transaction failed.',
  'helpCenter.article.uncertain.body':
    'Check the asset transaction history and any transaction ID already shown. Avoid starting a duplicate payment while the outcome is unknown.\n\nFor an Ethereum transfer, use the saved transfer recovery screen when it appears. Review the saved details and choose Continue transfer or Show submitted transfer as offered. This follows the existing operation. If the result remains unclear, ask for help with the network, error message and public transaction ID.',
  'helpCenter.article.uncertain.keywords':
    'uncertain broadcast timeout failed retry duplicate ethereum recovery submission',
  'helpCenter.article.convert.title': 'How conversions work',
  'helpCenter.article.convert.summary':
    'A conversion exchanges one currency for another through a supported Verus currency basket.',
  'helpCenter.article.convert.body':
    'A basket holds reserve currencies. The protocol calculates conversion prices from its reserves and processes conversions together at consensus. The amount you receive depends on the route, amount, reserve balances and fees.\n\nChoose [[convert|Convert]], select what you pay and receive, and review the destination and estimate. A conversion can also deliver to another supported network. Only routes available for the chosen source and destination appear; adding a token does not automatically make it convertible.',
  'helpCenter.article.convert.keywords':
    'swap exchange trade convert conversion basket reserve defi pbaas',
  'helpCenter.article.conversion-estimate.title': 'Why the conversion amount can change',
  'helpCenter.article.conversion-estimate.summary':
    'The receive amount is an estimate until the network processes the conversion.',
  'helpCenter.article.conversion-estimate.body':
    'Other conversions can change the basket reserves between your quote and settlement. The size of your conversion also affects the price. This price movement is often called slippage. The wallet displays the route and estimated result before you approve.\n\nRead the latest review, including conversion and network fees. Refresh an expired review. The fiat value in your portfolio is a separate market estimate and is not the rate used to settle your conversion.',
  'helpCenter.article.conversion-estimate.keywords':
    'slippage estimate quote rate price minimum receive final amount basket',
  'helpCenter.article.cross-chain.title': 'How cross-chain transfers work',
  'helpCenter.article.cross-chain.summary':
    'A cross-chain transfer moves value from one network to another through a supported route.',
  'helpCenter.article.cross-chain.body':
    'The source network first records the transfer. Proof of that transfer then needs to be accepted and processed on the destination network. These stages can take longer than a payment that stays on one chain.\n\nCheck the destination network and receiving currency in Review. A source transaction ID or source confirmation does not establish destination arrival. Use the available explorer details to inspect progress, and allow for additional verification and any requirements of the receiving service. Availability depends on the route and network; this article is not a live bridge-status report.',
  'helpCenter.article.cross-chain.keywords':
    'bridge cross chain cross-chain export import destination pbaas delayed',
  'helpCenter.article.ethereum.title': 'Ethereum bridge transfers and approvals',
  'helpCenter.article.ethereum.summary':
    'An Ethereum token bridge transfer may need a token approval before the transfer itself.',
  'helpCenter.article.ethereum.body':
    'An approval gives the bridge contract permission to use the token amount. It is a separate on-chain operation and can incur gas. An approval transaction alone does not mean the bridge transfer has been submitted or received. Keep enough ETH on the source network for the required steps.\n\nFollow the wallet review and any saved transfer recovery steps. If approval succeeds but the next step fails, continue the saved operation when offered. Bridge routes may be disabled or unavailable in a build or on a network; do not assume every Ethereum token can be bridged.',
  'helpCenter.article.ethereum.keywords':
    'eth ethereum erc20 approval allowance gas bridge continue recovery',
  'helpCenter.article.verusid.title': 'What is a VerusID?',
  'helpCenter.article.verusid.summary':
    'A VerusID is an on-chain identity with a name, an identity address and rules for who can control it.',
  'helpCenter.article.verusid.body':
    'You can use a VerusID as a payment destination on supported routes and with applications that support VerusID. Its primary addresses and signing rules determine who can authorize actions. Revocation and recovery authorities provide separate controls.\n\nA public profile can add images and a description to the identity. Applications choose which identity and profile features they display. A name or profile image does not by itself prove someone is trustworthy.',
  'helpCenter.article.verusid.keywords':
    'identity name handle at address ownership account portable',
  'helpCenter.article.link-identity.title': 'Linking and unlinking a VerusID',
  'helpCenter.article.link-identity.summary':
    'Linking adds an existing identity to this wallet. It does not register a new identity or transfer ownership.',
  'helpCenter.article.link-identity.body':
    'Open [[verusid|VerusID]] and use Link VerusID to find an existing identity on the selected network. The wallet checks the identity and what this wallet is allowed to do. Seeing an identity or its balance does not necessarily mean you control it.\n\nUnlink removes the local link. The identity, its funds and its public profile remain on-chain. To change the identity itself, an authorized on-chain update is required.',
  'helpCenter.article.link-identity.keywords':
    'link unlink find lookup register registration owner read only',
  'helpCenter.article.public-profile.title': 'What becomes public in my profile?',
  'helpCenter.article.public-profile.summary':
    'Publishing stores your chosen profile images and description on the public Verus blockchain.',
  'helpCenter.article.public-profile.body':
    'Anyone can read published profile data. Apps that support this profile can display it using your VerusID, so the information is not tied to this installation. Each app decides which fields it supports.\n\nLater changes or removals update the current profile; earlier versions remain in blockchain history. Publish only information you intend to make public. Your private contact notes and recovery secrets are not included in a profile update.',
  'helpCenter.article.public-profile.keywords':
    'onchain on chain on-chain public permanent privacy profile picture photo avatar header description delete remove history',
  'helpCenter.article.publish-profile.title': 'Editing and publishing a profile',
  'helpCenter.article.publish-profile.summary':
    'Edits start as a local draft. Publishing requires review, a network fee and an authorized signature.',
  'helpCenter.article.publish-profile.body':
    'Open your linked [[verusid|VerusID]] and choose Edit profile when available. Add or change the avatar, header image or description, then review the changes and cost. This build publishes profiles for supported VRSCTEST identities only. An active identity and a supported control setup are required.\n\nThe review may offer one update or two, depending on the images. With two updates, the first must confirm before you review and approve the second. Ordinary unpublished drafts last for the current unlock session. A saved continuation after the first update has a separate recovery flow.',
  'helpCenter.article.publish-profile.keywords':
    'edit publish profile images avatar header fee draft testnet vrsctest two updates',
  'helpCenter.article.profile-pending.title': 'Why my profile still shows the old version',
  'helpCenter.article.profile-pending.summary':
    'The wallet keeps showing the last confirmed profile while an update is waiting for confirmation.',
  'helpCenter.article.profile-pending.body':
    'Use the submitted changes and transaction details to see what was sent. A submitted update has not yet established the new confirmed profile. If a confirmation check fails, retry the check before publishing again.\n\nFor a two-update publication, the avatar and description can confirm before the header. Review and approve the remaining header update when offered. Other apps may refresh their profile data at different times, so their display can lag behind the chain.',
  'helpCenter.article.profile-pending.keywords':
    'profile pending old image unchanged waiting confirm header second update saved later',
  'helpCenter.article.profile-unavailable.title': 'Why a profile cannot be shown or edited',
  'helpCenter.article.profile-unavailable.summary':
    'A missing profile, an unreadable profile and a profile you cannot edit are different states.',
  'helpCenter.article.profile-unavailable.body':
    'An identity may have no public profile yet. If profile data cannot be loaded or verified, the wallet can still show core identity details; retry when your connection is available.\n\nEditing requires this wallet to control a supported active identity. Mainnet publishing, token-controlled identities and unsupported signing arrangements are not available in this profile editor. Read the reason shown by the wallet. Linking the identity again does not grant signing permission.',
  'helpCenter.article.profile-unavailable.keywords':
    'profile unavailable read only cannot edit missing permission unsupported mainnet',
  'helpCenter.article.recovery.title': 'What should I back up?',
  'helpCenter.article.recovery.summary':
    'Keep the recovery secret for every set of funds you use, including a separate Private Verus secret if you created or imported one.',
  'helpCenter.article.recovery.body':
    'Open Settings, then [[profile-security|Profile and security]], to view recovery information. Private Verus recovery information is also accessible from its settings. Follow the wallet prompts and keep an accurate offline copy. A separately imported key may protect only its own addresses.\n\nAnyone with a recovery phrase or spending key can potentially use the funds it controls. Do not share it in chat, screenshots or a support form. Your local password and your recovery secret serve different purposes; knowing the password alone does not restore a lost installation.',
  'helpCenter.article.recovery.keywords':
    'backup seed phrase secret private spending key 24 words safety lost device',
  'helpCenter.article.forgot-password.title': 'I cannot unlock my wallet',
  'helpCenter.article.forgot-password.summary':
    'A forgotten local password cannot be recovered by the wallet or the community.',
  'helpCenter.article.forgot-password.body':
    'If you have the original recovery phrase or another supported recovery secret, import it as a wallet and set a new local password. Choose the import method that matches your backup and use the correct network. You may need to restore a separate Private Verus secret as well.\n\nKeep the old installation and its files until you have checked your restored addresses and balances. Importing keys does not recreate local contacts or notes. VerusID recovery is a separate mechanism and is only useful when the required authority can still authorize recovery.',
  'helpCenter.article.forgot-password.keywords':
    'forgot password lost access locked unlock reset restore import device',
  'helpCenter.article.guard.title': 'What VerusID Guard can recover',
  'helpCenter.article.guard.summary':
    'VerusID Guard helps an authorized revocation or recovery authority act on a VerusID.',
  'helpCenter.article.guard.body':
    'Revocation disables normal use of the identity once the revocation takes effect on-chain. Recovery can assign new primary control to a revoked identity. Both require the appropriate authority and an on-chain transaction; selecting an authority does not help if you also lose access to its keys.\n\nGuard is available from Welcome and Unlock. It does not reset a local password or recover arbitrary Bitcoin, Ethereum or ordinary address funds. If keys are compromised, identity recovery only addresses what that identity controls. Check the identity, authorities, network and proposed changes before approving.',
  'helpCenter.article.guard.keywords':
    'revoke recover guard stolen compromised freeze authority identity lost keys',
  'helpCenter.article.private-verus.title': 'Setting up Private Verus',
  'helpCenter.article.private-verus.summary':
    'Private Verus uses a shielded address and needs its own recovery material and synchronization.',
  'helpCenter.article.private-verus.body':
    'Open Settings, then [[private-verus|Private Verus]]. Depending on your wallet, you can reuse the primary recovery phrase, create a new privacy recovery secret, or import one. If you create or import a separate secret, back it up separately. The primary wallet backup alone may not restore those private funds.\n\nLet private synchronization finish before sending. It scans the chain for funds the private keys can use. A new installation or restored privacy secret may need time to find the full balance. Follow any setup or restart instructions shown by the wallet.',
  'helpCenter.article.private-verus.keywords':
    'privacy shielded zs sapling dlight sync synchronize private setup seed',
  'helpCenter.article.private-send.title': 'Private addresses and transaction memos',
  'helpCenter.article.private-send.summary':
    'A shielded address protects different information from a public receiving address. Check both ends of the transfer.',
  'helpCenter.article.private-send.body':
    'Private Verus sends can target supported shielded or transparent Verus destinations. Sending to a public address exposes the receiving side on-chain. Using the private source does not make a public destination private.\n\nPrivate memos are supported for shielded destinations. The wallet must finish private synchronization and prepare the transaction proof before it can submit. Proof generation can take time. If submission becomes uncertain, check history before trying another send.',
  'helpCenter.article.private-send.keywords':
    'memo message shielded transparent zs private proof slow privacy',
  'helpCenter.article.requests.title': 'Reviewing a request from an app',
  'helpCenter.article.requests.summary':
    'A request asks the wallet to perform specific actions. Check the requester and each action before approving.',
  'helpCenter.article.requests.body':
    'Use Open request for a supported request, or open a supported wallet link. The wallet verifies the request and shows the actions it understands. These can include authentication and identity changes; some actions also require funding and a fee.\n\nA valid signature identifies the signer, but does not make the request desirable. Check the identity, permissions, destination and changes yourself. Reject an unexpected request. The Apps section is not currently an app directory, and Activity has no transaction history in this version; inspect an asset for its transactions.',
  'helpCenter.article.requests.keywords':
    'app apps request deeplink link sign signature authentication login permissions activity',
  'helpCenter.article.contacts.title': 'Contacts and public profiles',
  'helpCenter.article.contacts.summary':
    '[[contacts|Contacts]] store recipients and private notes in this wallet. A linked public profile comes from the VerusID.',
  'helpCenter.article.contacts.body':
    "Save a contact with a supported address, or associate a VerusID. A public profile can help you recognize the identity; your own note stays local. Check the selected receiving address and network each time you pay.\n\nDeleting a contact removes the local entry. It does not delete the person's VerusID, public profile or blockchain transactions. Contacts are encrypted in this installation and are not recreated just by importing the wallet recovery phrase on another device.",
  'helpCenter.article.contacts.keywords':
    'contact address book recipient private note save delete profile',
  'helpCenter.article.watchlist.title': 'What the Watchlist shows',
  'helpCenter.article.watchlist.summary':
    'The [[watchlist|Watchlist]] follows public holdings of supported Verus addresses and identities.',
  'helpCenter.article.watchlist.body':
    'Adding an address or identity lets you inspect its available public balances. It does not import a key or grant permission to spend. Watched funds are separate from funds this wallet controls.\n\nResults depend on the supported networks and the data that can be reached. An unavailable or partially loaded result is not proof of a zero balance. Private balances cannot be discovered from a public address lookup. Removing a watched entry changes your local list only.',
  'helpCenter.article.watchlist.keywords':
    'watch watchlist read only public balance address identity holdings',
  'helpCenter.article.local-data.title': 'What stays on this device?',
  'helpCenter.article.local-data.summary':
    'Keys and local wallet records serve a different purpose from public blockchain records.',
  'helpCenter.article.local-data.body':
    'Contacts, private notes, watchlist choices and display settings are local wallet data. Publishing a VerusID profile does not publish these records. Importing a recovery secret restores the keys it represents, not a copy of every local setting or contact.\n\nPublic transactions and published profile history remain on their networks. Locking the wallet protects local access; it does not hide information that is already public. Keep the original wallet files until you have verified any move to a new installation.',
  'helpCenter.article.local-data.keywords':
    'local data device privacy restore backup sync contacts notes storage public',
  'helpCenter.article.settings.title': 'Display, language and wallet locking',
  'helpCenter.article.settings.summary':
    'Settings controls appearance, display currency, language, auto-lock and access to recovery information.',
  'helpCenter.article.settings.body':
    'Use [[display-language|Display and language]] to choose a theme, language or fiat display currency. Changing the display currency changes estimates in the interface; it does not convert your funds.\n\n[[profile-security|Profile and security]] includes the inactivity lock interval and recovery access. Lock ends the current unlocked session, so finish or safely leave your work first. [[private-verus|Private Verus]] has its own setup page. [[about-support|About and support]] shows the app version, which is useful when reporting a problem.',
  'helpCenter.article.settings.keywords':
    'settings theme dark light language fiat euro currency auto lock version',
  'helpCenter.article.support.title': 'Getting help from the community',
  'helpCenter.article.support.summary':
    'Describe what you tried, what you expected and the message the wallet showed.',
  'helpCenter.article.support.body':
    'Include the app version, network and asset. A public transaction ID can help with a transfer problem, but it can also reveal transaction details. Review screenshots before sharing them and remove anything private.\n\nUse Ask the community to open the Verus Discord. Never send a recovery phrase, private key, spending key or password to someone offering help. Core Help articles work offline; community links, live balances and network lookups require a connection.',
  'helpCenter.article.support.keywords':
    'support community discord error problem report bug offline help',
};
