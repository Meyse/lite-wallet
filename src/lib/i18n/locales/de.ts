import { en } from './en';

export const de: Record<string, string> = {
  ...en,
  'common.loading': 'Wird geladen…',
  'common.cancel': 'Abbrechen',
  'common.continue': 'Weiter',
  'common.back': 'Zuruck',
  'common.done': 'Fertig',
  'common.retry': 'Erneut versuchen',
  'common.unknownError': 'Unbekannter Fehler',
  'common.network.mainnet': 'Mainnet',
  'common.network.testnet': 'Testnet',

  'welcome.titleLine1': 'Dein Zugang zum',
  'welcome.titleLine2': 'neuen Internet',
  'welcome.cta.start': 'Neue Wallet erstellen',
  'welcome.cta.existing': 'Bestehende Wallet verwenden',

  'unlock.switch': 'Wechseln',
  'unlock.switchLabel': 'Wallet wechseln, aktuelle Wallet: {name}',
  'unlock.password': 'Passwort',
  'unlock.button.unlocking': 'Wird entsperrt…',
  'unlock.button.unlock': 'Entsperren',
  'unlock.button.createWallet': 'Neue Wallet erstellen',
  'unlock.error.invalidPassword': 'Falsches Passwort. Bitte versuche es erneut.',
  'unlock.error.operationFailed':
    'Wallet konnte auf diesem Gerat nicht entsperrt werden. Versuche es erneut oder erstelle sie neu.',
  'unlock.error.invalidArgs':
    'Die Entsperranfrage war ungueltig. Starte die App neu und versuche es erneut.',
  'unlock.error.generic': 'Wallet kann gerade nicht entsperrt werden. Bitte versuche es erneut.',
  'unlock.switcher.title': 'Wallet auswahlen',
  'unlock.create.title': 'Wallet erstellen oder importieren',
  'unlock.create.newTitle': 'Neue Wallet erstellen',
  'unlock.create.newDescription':
    'Erzeuge eine neue Wiederherstellungsphrase und richte eine neue Wallet ein.',
  'unlock.create.importTitle': 'Bestehende Wallet importieren',
  'unlock.create.importDescription':
    'Verwende deine bestehende Seed-Phrase, um eine Wallet auf diesem Gerat wiederherzustellen.',
  'unlock.importMethods.title': 'Wallet importieren',
  'unlock.importMethods.back': 'Zuruck',
  'unlock.importMethods.seed24Title': '24-Wort-Seed-Phrase importieren',
  'unlock.importMethods.seed24Description':
    'Gib deine 24 Wiederherstellungsworter ein, um diese Wallet wiederherzustellen.',
  'unlock.importMethods.textTitle': 'Private Key oder Seed-Text einfugen',
  'unlock.importMethods.textDescription':
    'Fuge einen Private Key, eine Seed-Phrase oder einen Seed-Text manuell ein.',

  'guard.mode.revoke': 'Widerrufen',
  'guard.mode.recover': 'Wiederherstellen',
  'guard.mode.revokeLower': 'widerrufen',
  'guard.mode.recoverLower': 'wiederherstellen',
  'guard.flow.review.fee': 'Geschaftzte Gebuhr',
  'guard.flow.result.txidLabel': 'Transaktions-ID',

  'languageGate.title': 'Wahle deine Sprache',
  'languageGate.button': 'Weiter',
  'languageGate.option.en': 'English',
  'languageGate.option.nl': 'Nederlands',
  'languageGate.option.de': 'Deutsch',
  'languageGate.option.es': 'Español',

  'wallet.layout.dismiss': 'Schliessen',
  'wallet.layout.noticeTitle': 'Wallet-Hinweis',
  'wallet.layout.backgroundError.title': 'Aktualisierung verzogert',
  'wallet.layout.backgroundError.balance':
    'Einige Guthaben konnten nicht aktualisiert werden. Die angezeigten Werte sind moglicherweise veraltet.',
  'wallet.layout.backgroundError.info':
    'Der Netzwerkstatus konnte nicht aktualisiert werden. Versuche es erneut, falls das Problem weiterhin besteht.',
  'wallet.layout.backgroundError.transactionsWarning':
    'Moglicherweise fehlen kurzlich erfolgte Aktivitaten.',
  'wallet.layout.backgroundError.transactions':
    'Die letzten Aktivitaten konnten nicht aktualisiert werden. Versuche es erneut, falls das Problem weiterhin besteht.',
  'wallet.layout.backgroundError.generic':
    'Die Wallet-Daten konnten nicht aktualisiert werden. Versuche es erneut, falls das Problem weiterhin besteht.',
  'wallet.sidebar.wallet': 'Wallet',
  'wallet.sidebar.identities': 'Identitaten',
  'wallet.sidebar.apps': 'Apps',
  'wallet.sidebar.activity': 'Aktivitat',
  'wallet.sidebar.addressBook': 'Adressbuch',
  'wallet.sidebar.openRequest': 'Open request',
  'wallet.topbar.settings': 'Einstellungen',
  'wallet.topbar.lockWallet': 'Sperren',
  'genericRequest.import.title': 'Open request',
  'genericRequest.import.heading': 'Paste a GenericRequest',
  'genericRequest.import.description':
    'Open a Verus app request from a full `verus:` link or the raw request hex.',
  'genericRequest.import.label': 'Request-Link oder Code',
  'genericRequest.import.placeholder': 'Verus:-Link oder Request-Code einfügen',
  'genericRequest.import.help': 'Mit Cmd/Ctrl + Enter öffnen.',
  'genericRequest.import.paste': 'Paste',
  'genericRequest.import.pasteTooltip': 'Aus Zwischenablage einfügen',
  'genericRequest.import.submit': 'Öffnen',
  'genericRequest.import.error.empty': 'Paste a request to continue.',
  'genericRequest.import.error.invalid': 'This request could not be parsed.',
  'genericRequest.import.error.networkMismatch':
    'This request is for a different network than the active wallet.',
  'genericRequest.unsupported.detail':
    'This request contains detail types that are not supported on desktop yet.',
  'genericRequest.unsupported.empty': 'This request does not contain any supported actions.',
  'genericRequest.unsupported.invalidGrouping':
    'This request combines actions in a way that desktop does not support.',
  'genericRequest.unsupported.provisioningNeedsAuth':
    'Provisioning requests must be attached to an authentication request.',
  'genericRequest.unsupported.unsigned': 'This request is not signed.',
  'genericRequest.error.generic': 'This request could not be opened right now.',
  'genericRequest.error.invalidSignature': 'The request signature could not be verified.',
  'genericRequest.error.walletLocked': 'Wallet is locked. Unlock and try again.',
  'genericRequest.error.identityOwnershipMismatch':
    'This wallet cannot satisfy the ownership requirements for this request.',
  'genericRequest.error.identityRequestExpired': 'This identity update request has expired.',
  'genericRequest.error.identityUnsupportedAuthority':
    'This request needs an authority type that desktop does not support yet.',
  'genericRequest.error.identityUnsupportedActiveCurrencyChange':
    'Changing active currency is not supported for GenericRequest identity updates.',
  'genericRequest.error.identityUnsupportedTokenizedControlChange':
    'Changing tokenized ID control is not supported for GenericRequest identity updates.',
  'genericRequest.error.insufficientFunds':
    'Not enough VRSC is available to pay the network fee for this request.',
  'genericRequest.error.linkedIdentities': 'Could not load linked VerusIDs for this request.',
  'genericRequest.error.fundingSources': 'Could not load wallets that can pay for this request.',
  'genericRequest.error.noSourceChannel': 'No funding source is available for this request.',
  'genericRequest.error.updatePreflight': 'Could not prepare this identity update.',
  'genericRequest.error.updateSend': 'Could not broadcast this identity update.',
  'genericRequest.error.complete': 'Could not finish this request.',
  'genericRequest.error.unsupportedPostCallback':
    'This app requested a POST callback that is not supported on desktop.',
  'genericRequest.steps.auth': 'Sign in',
  'genericRequest.steps.updateOverview': 'Review',
  'genericRequest.steps.updateRisk': 'High risk',
  'genericRequest.steps.updateContent': 'Content',
  'genericRequest.steps.updateFunding': 'Funding',
  'genericRequest.steps.complete': 'Finish',
  'genericRequest.summary.title': 'Request summary',
  'genericRequest.summary.toggle': 'Show summary',
  'genericRequest.summary.network': 'Network',
  'genericRequest.summary.signer': 'Signed by',
  'genericRequest.summary.destination': 'Callback',
  'genericRequest.summary.provisioningTitle': 'Provisioning included',
  'genericRequest.summary.provisioningRequested':
    'A new VerusID can be provisioned from this request.',
  'genericRequest.auth.title': 'Sign-in request',
  'genericRequest.auth.description': 'Choose the VerusID you want to use for this request.',
  'genericRequest.auth.constraints': 'Requirements',
  'genericRequest.auth.detailsTitle': 'Request details',
  'genericRequest.auth.requiredId': 'Required VerusID',
  'genericRequest.auth.requiredSystem': 'Required system',
  'genericRequest.auth.requiredParent': 'Required parent',
  'genericRequest.auth.identities': 'Eligible VerusIDs',
  'genericRequest.auth.loading': 'Loading linked VerusIDs…',
  'genericRequest.auth.empty':
    'No linked VerusIDs match this request. Link an existing VerusID or provision a new one.',
  'genericRequest.auth.useIdentity': 'Use VerusID',
  'genericRequest.auth.selectIdentity': 'Select VerusID',
  'genericRequest.auth.signInTo': 'Sign in to {app}',
  'genericRequest.auth.requestedBy': 'Requested by {requester}',
  'genericRequest.auth.noSelection': 'No VerusID selected yet. Choose one to continue.',
  'genericRequest.auth.selectedDescription':
    'This VerusID will sign the response sent back to the requester.',
  'genericRequest.auth.changeSelection': 'Change',
  'genericRequest.auth.expires': 'Expires',
  'genericRequest.auth.selectorDescription':
    'Choose the linked VerusID you want to use for this sign-in request.',
  'genericRequest.auth.requesterFallback': 'Requesting app',
  'genericRequest.auth.linkIdentity': 'Link VerusID',
  'genericRequest.auth.continue': 'Continue',
  'genericRequest.auth.submit': 'Sign in',
  'genericRequest.update.title': 'Review identity update',
  'genericRequest.update.description': 'Review the identity changes requested by this app.',
  'genericRequest.update.preflightLoading': 'Preparing the identity update…',
  'genericRequest.update.targetIdentity': 'Target VerusID',
  'genericRequest.update.summary': 'Change summary',
  'genericRequest.update.highRiskCount': 'High-risk changes',
  'genericRequest.update.contentCount': 'Content changes',
  'genericRequest.update.warnings': 'Warnings',
  'genericRequest.update.highRiskTitle': 'Confirm high-risk changes',
  'genericRequest.update.highRiskDescription':
    'These changes affect ownership, recovery, revocation, status, or other high-risk identity fields.',
  'genericRequest.update.highRiskDescriptionContentClear':
    'After this update, apps will no longer see this data on your VerusID, but it will still be visible on the blockchain.',
  'genericRequest.update.highRiskAcknowledge':
    'I understand that these high-risk identity changes may be difficult or impossible to undo.',
  'genericRequest.update.contentTitle': 'Review content changes',
  'genericRequest.update.contentDescription':
    'Review how these changes affect the data apps see on this VerusID before continuing.',
  'genericRequest.update.beforeValue': 'Before',
  'genericRequest.update.afterValue': 'After',
  'genericRequest.update.currentValue': 'Current value',
  'genericRequest.update.newValue': 'New value',
  'genericRequest.update.emptyValue': 'Empty',
  'genericRequest.update.privateAddress': 'Private address',
  'genericRequest.update.contentPrefix': 'Content',
  'genericRequest.update.content.currentIdentityContent': 'Current identity content',
  'genericRequest.update.content.noneAfterUpdate': 'No content after update',
  'genericRequest.update.content.currentLabel': 'Current',
  'genericRequest.update.content.currentValueLabel': 'Current value',
  'genericRequest.update.content.currentValuesLabel': 'Current values',
  'genericRequest.update.content.existingLabel': 'Existing',
  'genericRequest.update.content.newLabel': 'Will add',
  'genericRequest.update.content.addingLabel': 'Will add',
  'genericRequest.update.content.willRemoveLabel': 'Will remove',
  'genericRequest.update.content.afterUpdateLabel': 'After this update',
  'genericRequest.update.content.badge.add': 'Will add',
  'genericRequest.update.content.badge.append': 'Will add',
  'genericRequest.update.content.badge.remove': 'Removing',
  'genericRequest.update.content.inspectTitle': 'Content details: {label}',
  'genericRequest.update.content.viewDetails': 'View details',
  'genericRequest.update.content.requestedValue': 'Requested value',
  'genericRequest.update.content.keyLine': 'Key: {label}',
  'genericRequest.update.content.removeValuePreview':
    'Apps will no longer see this value on your VerusID.',
  'genericRequest.update.content.removeMatchingValuesPreview':
    'Apps will no longer see the matching values on your VerusID.',
  'genericRequest.update.content.removeAllValuesPreview':
    'Apps will no longer see these values on your VerusID.',
  'genericRequest.update.contentClearTitle': 'Clear current identity content',
  'genericRequest.update.contentClearDescription':
    'Apps will no longer see this data on your VerusID, but it will still be visible on the blockchain.',
  'genericRequest.update.contentRemoveValueTitle': 'Remove one current value under {label}',
  'genericRequest.update.contentRemoveValueTitleGeneric': 'Remove one current value under this key',
  'genericRequest.update.contentRemoveValueDescription':
    'This request removes one value from {label}.',
  'genericRequest.update.contentRemoveAllValuesTitle': 'Remove all current values under {label}',
  'genericRequest.update.contentRemoveAllValuesTitleGeneric':
    'Remove all current values under this key',
  'genericRequest.update.contentRemoveAllValuesDescription':
    'This request removes every current value stored under {label}.',
  'genericRequest.update.contentRemoveMatchingValuesTitle':
    'Remove matching current values under {label}',
  'genericRequest.update.contentRemoveMatchingValuesTitleGeneric':
    'Remove matching current values under this key',
  'genericRequest.update.contentRemoveMatchingValuesDescription':
    'This request removes matching values from {label}.',
  'genericRequest.update.contentRemoveEffect':
    'Removed values stop appearing in the current identity content after this update.',
  'genericRequest.update.contentRemoveHistory': 'They can still be found on the blockchain.',
  'genericRequest.update.contentRemoveHistoryHash':
    'This request targets values matching hash {hash}. They can still be found on the blockchain.',
  'genericRequest.update.contentKeyTitle': 'Content key: {label}',
  'genericRequest.update.outcome.keepControlTitle': 'Wallet keeps control',
  'genericRequest.update.outcome.keepControlDescription':
    'All primary addresses after this update are still controlled by this wallet.',
  'genericRequest.update.outcome.shareControlTitle': 'Control becomes shared',
  'genericRequest.update.outcome.shareControlDescription':
    'This update keeps at least one wallet-controlled primary address, but also adds an external one.',
  'genericRequest.update.outcome.loseControlTitle': 'Wallet loses control',
  'genericRequest.update.outcome.loseControlDescription':
    'No primary address after this update is controlled by this wallet.',
  'genericRequest.update.outcome.clearContentTitle': 'Current content is cleared',
  'genericRequest.update.outcome.clearContentDescription':
    'This request clears current identity content without changing primary control.',
  'genericRequest.update.outcome.reviewRequiredTitle': 'Review requested identity changes',
  'genericRequest.update.outcome.reviewRequiredDescription':
    'This request changes sensitive identity fields and should be reviewed carefully.',
  'genericRequest.update.badge.inWallet': 'In wallet',
  'genericRequest.update.badge.external': 'External',
  'genericRequest.update.authority.sectionTitle': 'Authority changes',
  'genericRequest.update.authority.sectionDescription':
    'Recovery and revocation authorities can change who is able to take over this identity.',
  'genericRequest.update.authority.learnMore': 'Learn more',
  'genericRequest.update.authority.changeRevocation': 'Change revocation authority',
  'genericRequest.update.authority.changeRecovery': 'Change recovery authority',
  'genericRequest.update.authority.infoTitle': 'Authority changes',
  'genericRequest.update.authority.infoTitleBoth': 'Recovery and revocation are changing',
  'genericRequest.update.authority.infoTitleRecovery': 'Recovery authority is changing',
  'genericRequest.update.authority.infoTitleRevocation': 'Revocation authority is changing',
  'genericRequest.update.authority.infoBody':
    'Authority changes can shift who is able to revoke or recover this identity.',
  'genericRequest.update.authority.infoBodyBoth':
    'Both recovery and revocation authority are changing, which changes who can freeze or restore this identity.',
  'genericRequest.update.authority.infoBodyRecovery':
    'The recovery authority can restore access after revocation and may be able to take control during recovery.',
  'genericRequest.update.authority.infoBodyRevocation':
    'The revocation authority can freeze the identity and force recovery before it can be used again.',
  'genericRequest.update.primaryAddress.sectionTitle': 'Primary address changes',
  'genericRequest.update.primaryAddress.afterUpdateTitle': 'Primary addresses after update',
  'genericRequest.update.primaryAddress.addTitle': 'Add primary address',
  'genericRequest.update.primaryAddress.addWalletDescription':
    'This primary address is controlled by the active wallet.',
  'genericRequest.update.primaryAddress.addSharedDescription':
    'This primary address is external, but the wallet still controls another primary address after the update.',
  'genericRequest.update.primaryAddress.addLoseControlDescription':
    'This primary address is external and leaves no wallet-controlled primary address after the update.',
  'genericRequest.update.primaryAddress.removeTitle': 'Remove primary address',
  'genericRequest.update.primaryAddress.removeDescription':
    'This primary address will no longer control the identity after the update.',
  'genericRequest.update.status.title': 'Identity status',
  'genericRequest.update.status.revokedDescription': 'This update marks the identity as revoked.',
  'genericRequest.update.status.activeDescription':
    'This update restores the identity to an active state.',
  'genericRequest.update.status.revoked': 'Revoked',
  'genericRequest.update.status.active': 'Active',
  'genericRequest.update.fundingTitle': 'Choose wallet to pay from',
  'genericRequest.update.fundingDescription':
    'Choose the wallet that will pay the network fee for this identity update.',
  'genericRequest.update.fundingSource': 'Wallet to pay from',
  'genericRequest.update.fundingSourcePlaceholder': 'Choose a funding source',
  'genericRequest.update.fundingSourcesLoading': 'Loading wallet balances…',
  'genericRequest.update.fundingCalculating': 'Calculating fee…',
  'genericRequest.update.fundingEmpty':
    'No eligible wallets on this network are available to pay from.',
  'genericRequest.update.fundingFeePlaceholder': 'Select a wallet to calculate the fee.',
  'genericRequest.update.feeTitle': 'Estimated fee',
  'genericRequest.update.feeLabel': 'Estimated fee: {fee} {currency}',
  'genericRequest.update.broadcast': 'Broadcast update',
  'genericRequest.complete.title': 'Finish request',
  'genericRequest.complete.description': 'This request is ready to finish.',
  'genericRequest.complete.descriptionWithCallback':
    'This request is ready to sign and send back to the requesting app.',
  'genericRequest.complete.ready': 'The response is ready.',
  'genericRequest.complete.deliveryNotice': 'Response will be sent to {destination}.',
  'genericRequest.complete.sent': 'The response was signed successfully.',
  'genericRequest.complete.updateBroadcast': 'Identity update broadcast: {txid}',
  'genericRequest.complete.txidLabel': 'Transaction id',
  'genericRequest.complete.cta': 'Abschließen',
  'genericRequest.provisioning.title': 'Provision requested VerusID',
  'genericRequest.provisioning.description':
    'Create the requested VerusID, then finish this request after it appears in your wallet.',
  'genericRequest.provisioning.cta': 'Provision VerusID',
  'genericRequest.provisioning.ready': 'Provisioning can start from this request.',
  'genericRequest.provisioning.webhookMissing':
    'This request did not include a provisioning webhook.',
  'genericRequest.provisioning.submitted':
    'Provisioning started. Finish the request from Identities after the VerusID is ready.',
  'genericRequest.provisioning.error.generic': 'Could not submit the provisioning request.',
  'genericRequest.provisioning.error.noSigningAddress':
    'No active VRSC signing address is available for provisioning.',
  'genericRequest.provisioning.error.noWebhook':
    'This request does not include a provisioning webhook.',
  'genericRequest.provisioning.error.invalidRequest':
    'This provisioning request is missing required fields.',
  'genericRequest.provisioning.error.webhook': 'The provisioning service rejected the request.',
  'genericRequest.provisioning.error.invalidResponse':
    'The provisioning service returned an invalid response.',
  'genericRequest.provisioning.error.invalidResponseSignature':
    'The provisioning response signature could not be verified.',
  'genericRequest.provisioning.error.failed': 'The provisioning service reported a failure.',
  'genericRequest.provisioning.error.identityMismatch':
    'The provisioning response returned a different identity than requested.',
  'genericRequest.provisioning.error.nameMismatch':
    'The provisioning response returned a different name than requested.',
  'wallet.identity.provisioning.title': 'Pending provisioning',
  'wallet.identity.provisioning.description':
    'Track VerusIDs requested by apps and finish linking once they are ready.',
  'wallet.identity.provisioning.refresh': 'Refresh',
  'wallet.identity.provisioning.loading': 'Loading provisioning requests…',
  'wallet.identity.provisioning.errorLoad': 'Could not load provisioning requests right now.',
  'wallet.identity.provisioning.callbackPending': 'Callback pending',
  'wallet.identity.provisioning.serviceLabel': 'Requesting app: {value}',
  'wallet.identity.provisioning.info': 'View status',
  'wallet.identity.provisioning.linkAndContinue': 'Link and continue',
  'wallet.identity.provisioning.linkIdentity': 'Link identity',
  'wallet.identity.provisioning.linked': 'Identity linked.',
  'wallet.identity.provisioning.linkAndContinueQueued':
    'Identity linked. The original request is ready to continue.',
  'wallet.identity.provisioning.status.pending': 'Pending',
  'wallet.identity.provisioning.status.ready': 'Ready',
  'wallet.identity.provisioning.status.expired': 'Expired',
  'wallet.identity.provisioning.status.linked': 'Linked',
  'wallet.private.syncingPercent': 'Synchronisiert {percent}%',

  'wallet.settings.home.title': 'Einstellungen',
  'wallet.settings.home.description': 'Verwalte App-Einstellungen und Sicherheitswerkzeuge.',
  'wallet.settings.home.category.displayLanguage': 'Anzeige und Sprache',
  'wallet.settings.home.category.profileSecurity': 'Backups und Sicherheit',
  'wallet.settings.home.category.privateVerus': 'Privatsphare',
  'wallet.settings.home.category.aboutSupport': 'Info und Support',
  'wallet.settings.home.summary.displayLanguage': '{currency} • {language}',
  'wallet.settings.home.summary.profileSecurity': 'Auto-Sperre: {minutes} Min',
  'wallet.settings.home.summary.privateConfigured': 'Aktiviert',
  'wallet.settings.home.summary.privateNotConfigured': 'Nicht aktiviert',
  'wallet.settings.home.summary.version': 'Version {version}',
  'wallet.settings.display.title': 'Anzeige und Sprache',
  'wallet.settings.display.description': 'Wahle deine Anzeigewahrung und App-Sprache.',
  'wallet.settings.display.currency.label': 'Anzeigewahrung',
  'wallet.settings.display.currency.current': 'Aktuell: {code}',
  'wallet.settings.display.currency.quickPicks': 'Schnellauswahl',
  'wallet.settings.display.currency.otherAction': 'Andere Wahrung wahlen',
  'wallet.settings.display.currency.sheetTitle': 'Anzeigewahrung',
  'wallet.settings.display.currency.searchPlaceholder': 'Wahrungen suchen',
  'wallet.settings.display.currency.noResults': 'Keine Wahrung passt zu deiner Suche.',
  'wallet.settings.display.language.label': 'Sprache',
  'wallet.settings.display.language.description': 'Gilt fur die gesamte App.',

  'wallet.settings.profile.title': 'Backups und Sicherheit',
  'wallet.settings.profile.description':
    'Verwalte Auto-Sperre und Wiederherstellungszugang fur diese Wallet.',
  'wallet.settings.profile.autoLock.title': 'Wallet automatisch sperren',
  'wallet.settings.profile.autoLock.description':
    'Sperrt diese Wallet nach Inaktivitat. Aus ist nicht verfugbar.',
  'wallet.settings.profile.autoLock.option': '{minutes} Min',
  'wallet.settings.profile.recovery.title': 'Wiederherstellung und Schlussel',
  'wallet.settings.profile.recovery.description':
    'Zeige Seed und abgeleitete Schlussel nach Passwortbestatigung an.',

  'wallet.settings.privateVerus.title': 'Privatsphare',
  'wallet.settings.privateVerus.description':
    'Konfiguriere shielded Privatsphare fur {label}. Du kannst den primaren 24-Wort-Seed wiederverwenden oder einen separaten Privacy-Seed setzen.',
  'wallet.settings.privateVerus.statusConfigured': 'Privatsphare ist konfiguriert.',
  'wallet.settings.privateVerus.statusNotConfigured': 'Privatsphare ist noch nicht konfiguriert.',
  'wallet.settings.privateVerus.statusAddress': 'Shielded-Adresse: {address}',
  'wallet.settings.privateVerus.statusLoadError':
    'Privatsphare-Status konnte nicht geladen werden.',
  'wallet.settings.privateVerus.reusePrimary': 'Primaren Wallet-Seed wiederverwenden',
  'wallet.settings.privateVerus.createNew': 'Neuen Privacy-Seed erstellen',
  'wallet.settings.privateVerus.importLabel': 'Privacy-Seed oder Spending Key importieren',
  'wallet.settings.privateVerus.importPlaceholder':
    '24-Wort-Mnemonic oder secret-extended-key-main einfugen…',
  'wallet.settings.privateVerus.importAction': 'Privacy-Seed importieren',
  'wallet.settings.privateVerus.settingUp': 'Privatsphare wird konfiguriert…',
  'wallet.settings.privateVerus.generatedSeedTitle': 'Neue Privacy-Seed-Phrase',
  'wallet.settings.privateVerus.setupSuccess': 'Privatsphare konfiguriert.',
  'wallet.settings.privateVerus.setupSuccessRelogin':
    'Privatsphare konfiguriert. Sperre und entsperre deine Wallet, um private Kanale zu aktivieren.',
  'wallet.settings.privateVerus.setupError': 'Privatsphare konnte nicht konfiguriert werden.',
  'wallet.settings.privateVerus.advancedToggleShow': 'Erweiterte Privacy-Aktionen anzeigen',
  'wallet.settings.privateVerus.advancedToggleHide': 'Erweiterte Privacy-Aktionen ausblenden',

  'wallet.settings.about.title': 'Info',
  'wallet.settings.about.description': 'Versionsdetails und Support-Links.',
  'wallet.settings.about.appName': 'App-Name',
  'wallet.settings.about.version': 'Version',
  'wallet.settings.about.community': 'Community-Hangout offnen',

  'wallet.settings.recovery.title': 'Wiederherstellung und Schlussel',
  'wallet.settings.recovery.warningInline':
    'Jeder mit diesen Werten kann deine Funds kontrollieren. Bewahre sie offline und privat auf.',
  'wallet.settings.recovery.revealCta': 'Wiederherstellungsgeheimnisse anzeigen',
  'wallet.settings.recovery.passwordPlaceholder': 'Wallet-Passwort',
  'wallet.settings.recovery.passwordInvalid': 'Passwort ist falsch.',
  'wallet.settings.recovery.revealLoading': 'Wird angezeigt…',
  'wallet.settings.recovery.revealConfirm': 'Anzeigen',
  'wallet.settings.recovery.reveal': 'Anzeigen',
  'wallet.settings.recovery.hide': 'Ausblenden',
  'wallet.settings.recovery.copy': 'Kopieren',
  'wallet.settings.recovery.copySuccess': 'Kopiert',
  'wallet.settings.recovery.copyFailed': 'Kopieren fehlgeschlagen',
  'wallet.settings.recovery.valueUnavailable': 'Nicht verfugbar',
  'wallet.settings.recovery.primaryKindLabel': 'Typ des primaren Secrets',
  'wallet.settings.recovery.primarySection': 'Primares Secret',
  'wallet.settings.recovery.derivedKeysSection': 'Abgeleitete Schlussel',
  'wallet.settings.recovery.addressesSection': 'Adressen',
  'wallet.settings.recovery.dlightSection': 'Privacy-Secret',
  'wallet.settings.recovery.dlightKindLabel': 'Typ des Privacy-Secrets',
  'wallet.settings.recovery.field.primarySecret': 'Primare Secret-Daten',
  'wallet.settings.recovery.field.verusWif': 'Verus WIF',
  'wallet.settings.recovery.field.btcWif': 'Bitcoin WIF',
  'wallet.settings.recovery.field.ethPrivateKey': 'Ethereum Private Key',
  'wallet.settings.recovery.field.verusAddress': 'Verus-Adresse',
  'wallet.settings.recovery.field.btcAddress': 'Bitcoin-Adresse',
  'wallet.settings.recovery.field.ethAddress': 'Ethereum-Adresse',
  'wallet.settings.recovery.field.dlightSecret': 'Privacy-Seed oder Spending Key',
  'wallet.settings.recovery.field.dlightShieldedAddress': 'Privacy shielded Adresse',
  'wallet.settings.recovery.field.dlightDerivedSpendingKey': 'Abgeleiteter privater Spending Key',
  'wallet.settings.recovery.kind.seedText': 'Seed-Text',
  'wallet.settings.recovery.kind.wif': 'WIF',
  'wallet.settings.recovery.kind.privateKeyHex': 'Private Key (hex)',
  'wallet.settings.recovery.kind.dlightMnemonic': 'Mnemonic-Seed',
  'wallet.settings.recovery.kind.dlightSpendingKey': 'Spending Key',
  'wallet.settings.recovery.kind.unknown': 'Unbekannt',
  'wallet.session.expired': 'Sitzung abgelaufen. Wallet gesperrt.',

  'wallet.overview.mainWallet': 'Haupt-Wallet',
  'wallet.overview.errorActiveAssetsFallback':
    'Aktive Assets konnten nicht geladen werden. Deine letzte Asset-Ansicht wird angezeigt.',
  'wallet.overview.send': 'Senden',
  'wallet.overview.receive': 'Empfangen',
  'wallet.overview.convert': 'Konvertieren',
  'wallet.overview.noChannel': 'Noch kein aktiver Kanal verfugbar.',
  'wallet.overview.hideHoldings': 'Bestande ausblenden',
  'wallet.overview.showHoldings': 'Bestande anzeigen',
  'wallet.overview.scrollHintMoreAssets': 'Scrollen fur weitere Assets',
  'wallet.overview.partialRatesNotice': 'Einige Asset-Kurse sind nicht verfugbar.',
  'wallet.overview.partialBalancesNotice':
    'Einige Bestande werden noch geladen oder sind nicht verfugbar.',
  'wallet.overview.partialTotalLabel': 'Teilweise',
  'wallet.overview.partialTotalDescription':
    'Einige Guthaben oder Wechselkurse sind nicht verfugbar, daher ist diese Summe unvollstandig.',
  'wallet.assetDetails.noTransactionsInRecentRange':
    'Keine Transaktionen im letzten Verlaufsbereich.',
  'wallet.assetDetails.loadOlderTransactions': 'Alteren Verlauf prufen',
  'wallet.assetDetails.errorLoadScopes': 'Subwallet-Scopes konnten nicht geladen werden.',
  'wallet.assetDetails.scopeUnavailable': 'Fur dieses Asset ist kein Scope verfugbar.',
  'wallet.assetDetails.scopePicker': 'Adresse und Netzwerk andern',
  'wallet.assetDetails.readOnlyHelper':
    'Senden und Konvertieren sind nur von deiner primaren Adresse verfugbar.',
  'wallet.assetDetails.privateSyncInlineHelper': 'Bestand wird synchronisiert',
  'wallet.assetDetails.sendCapabilityInline': 'Send-Sync {percent}%',
};
