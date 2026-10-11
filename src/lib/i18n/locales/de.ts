import { en } from './en';

export const de: Record<string, string> = {
  ...en,
  'helpCenter.unlockRequired': 'Entsperre deine Wallet, um diesen Bildschirm zu öffnen.',
  'common.loading': 'Wird geladen…',
  'common.cancel': 'Abbrechen',
  'common.continue': 'Weiter',
  'common.back': 'Zuruck',
  'common.close': 'Schließen',
  'common.clearSearch': 'Suche löschen',
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
    'Erzeuge eine neue geheime Wiederherstellungsphrase und richte eine neue Wallet ein.',
  'unlock.create.importTitle': 'Bestehende Wallet importieren',
  'unlock.create.importDescription':
    'Verwende deine bestehende geheime Wiederherstellungsphrase, um eine Wallet auf diesem Gerat wiederherzustellen.',
  'unlock.importMethods.title': 'Wallet importieren',
  'unlock.importMethods.back': 'Zurück zu den Erstellungsoptionen',
  'unlock.importMethods.seed24Title': 'Geheime Wiederherstellungsphrase importieren',
  'unlock.importMethods.seed24Description':
    'Gib deine geheime Wiederherstellungsphrase mit 24 Wortern ein, um diese Wallet wiederherzustellen.',
  'unlock.importMethods.textTitle': 'Private Key oder benutzerdefinierten Seed einfugen',
  'unlock.importMethods.textDescription':
    'Fuge einen Private Key, eine geheime Wiederherstellungsphrase oder einen benutzerdefinierten Seed manuell ein.',

  'guard.mode.revoke': 'Widerrufen',
  'guard.mode.recover': 'Wiederherstellen',
  'guard.mode.revokeLower': 'widerrufen',
  'guard.mode.recoverLower': 'wiederherstellen',
  'guard.flow.review.fee': 'Geschaftzte Gebuhr',
  'guard.flow.result.txidLabel': 'Transaktions-ID',

  'languageGate.title': 'Wähle deine Sprache',
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
  'wallet.sidebar.identities': 'VerusID',
  'wallet.sidebar.apps': 'Apps',
  'wallet.sidebar.activity': 'Aktivitat',
  'wallet.sidebar.addressBook': 'Adressbuch',
  'wallet.sidebar.openRequest': 'Anfrage öffnen',
  'wallet.topbar.settings': 'Einstellungen',
  'wallet.topbar.lockWallet': 'Sperren',
  'genericRequest.import.title': 'Anfrage öffnen',
  'genericRequest.import.label': 'Request-Link oder Code',
  'genericRequest.import.placeholder': 'Verus:-Link oder Request-Code einfügen',
  'genericRequest.import.help': 'Mit Cmd/Ctrl + Enter öffnen.',
  'genericRequest.import.pasteTooltip': 'Aus Zwischenablage einfügen',
  'genericRequest.import.submit': 'Öffnen',
  'genericRequest.import.error.empty': 'Füge eine Anfrage ein, um fortzufahren.',
  'genericRequest.import.error.invalid': 'Diese Anfrage konnte nicht gelesen werden.',
  'genericRequest.error.generic': 'Diese Anfrage kann gerade nicht geöffnet werden.',
  'genericRequest.error.identityOwnershipMismatch':
    'Diese Wallet erfüllt die Eigentumsanforderungen dieser Anfrage nicht.',
  'genericRequest.error.identityRequestExpired':
    'Diese Anfrage zur Identitätsänderung ist abgelaufen.',
  'genericRequest.error.identityUnsupportedAuthority':
    'Diese Anfrage braucht eine Art von Autorität, die der Desktop noch nicht unterstützt.',
  'genericRequest.error.identityUnsupportedActiveCurrencyChange':
    'Eine Anfrage kann die aktive Währung einer Identität nicht ändern.',
  'genericRequest.error.identityUnsupportedTokenizedControlChange':
    'Eine Anfrage kann die tokenisierte Kontrolle einer ID nicht ändern.',
  'genericRequest.error.insufficientFunds':
    'Es ist nicht genug VRSC verfügbar, um die Netzwerkgebühr für diese Anfrage zu zahlen.',
  'genericRequest.error.linkedIdentities':
    'Die verknüpften VerusIDs für diese Anfrage konnten nicht geladen werden.',
  'genericRequest.error.fundingSources':
    'Die Wallets, die für diese Anfrage zahlen können, konnten nicht geladen werden.',
  'genericRequest.error.noSourceChannel': 'Für diese Anfrage ist keine Zahlungsquelle verfügbar.',
  'genericRequest.error.updatePreflight':
    'Diese Identitätsänderung konnte nicht vorbereitet werden.',
  'genericRequest.error.complete': 'Diese Anfrage konnte nicht abgeschlossen werden.',
  'genericRequest.auth.requiredId': 'Erforderliche VerusID',
  'genericRequest.auth.requiredSystem': 'Erforderliches System',
  'genericRequest.auth.requiredParent': 'Erforderliche übergeordnete ID',
  'genericRequest.auth.loading': 'Verknüpfte VerusIDs werden geladen…',
  'genericRequest.auth.selectorDescription':
    'Wähle die verknüpfte VerusID, die du für diese Anmeldung verwenden möchtest.',
  'genericRequest.auth.requesterFallback': 'Anfragende App',
  'genericRequest.auth.linkIdentity': 'VerusID verknüpfen',
  'genericRequest.update.preflightLoading': 'Identitätsänderung wird vorbereitet…',
  'genericRequest.update.highRiskTitle': 'Riskante Änderungen bestätigen',
  'genericRequest.update.highRiskDescription':
    'Diese Änderungen betreffen Eigentum, Wiederherstellung, Widerruf, Status oder andere riskante Felder der Identität.',
  'genericRequest.update.highRiskDescriptionContentClear':
    'Nach dieser Änderung sehen Apps diese Daten auf deiner VerusID nicht mehr, aber sie bleiben auf der Blockchain sichtbar.',
  'genericRequest.update.highRiskAcknowledge':
    'Ich verstehe, dass diese riskanten Identitätsänderungen schwer oder gar nicht rückgängig zu machen sind.',
  'genericRequest.update.beforeValue': 'Vorher',
  'genericRequest.update.afterValue': 'Nachher',
  'genericRequest.update.currentValue': 'Aktueller Wert',
  'genericRequest.update.emptyValue': 'Leer',
  'genericRequest.update.privateAddress': 'Private Adresse',
  'genericRequest.update.contentPrefix': 'Inhalt',
  'genericRequest.update.content.currentIdentityContent': 'Aktueller Inhalt der Identität',
  'genericRequest.update.content.credentials': 'Nachweise (verschlüsselt gespeichert)',
  'genericRequest.update.content.noneAfterUpdate': 'Kein Inhalt nach der Änderung',
  'genericRequest.update.content.currentValueLabel': 'Aktueller Wert',
  'genericRequest.update.content.currentValuesLabel': 'Aktuelle Werte',
  'genericRequest.update.content.existingLabel': 'Vorhanden',
  'genericRequest.update.content.newLabel': 'Wird hinzugefügt',
  'genericRequest.update.content.addingLabel': 'Wird hinzugefügt',
  'genericRequest.update.content.afterUpdateLabel': 'Nach dieser Änderung',
  'genericRequest.update.content.badge.add': 'Wird hinzugefügt',
  'genericRequest.update.content.badge.append': 'Wird hinzugefügt',
  'genericRequest.update.content.badge.remove': 'Wird entfernt',
  'genericRequest.update.content.inspectTitle': 'Inhaltsdetails: {label}',
  'genericRequest.update.content.viewDetails': 'Details ansehen',
  'genericRequest.update.content.requestedValue': 'Angefragter Wert',
  'genericRequest.update.content.keyLine': 'Schlüssel: {label}',
  'genericRequest.update.content.removeValuePreview':
    'Apps sehen diesen Wert auf deiner VerusID nicht mehr.',
  'genericRequest.update.content.removeMatchingValuesPreview':
    'Apps sehen die passenden Werte auf deiner VerusID nicht mehr.',
  'genericRequest.update.content.removeAllValuesPreview':
    'Apps sehen diese Werte auf deiner VerusID nicht mehr.',
  'genericRequest.update.contentClearTitle': 'Aktuellen Inhalt der Identität löschen',
  'genericRequest.update.contentClearDescription':
    'Apps sehen diese Daten auf deiner VerusID nicht mehr, aber sie bleiben auf der Blockchain sichtbar.',
  'genericRequest.update.contentRemoveValueTitle': 'Einen aktuellen Wert unter {label} entfernen',
  'genericRequest.update.contentRemoveValueTitleGeneric':
    'Einen aktuellen Wert unter diesem Schlüssel entfernen',
  'genericRequest.update.contentRemoveAllValuesTitle':
    'Alle aktuellen Werte unter {label} entfernen',
  'genericRequest.update.contentRemoveAllValuesTitleGeneric':
    'Alle aktuellen Werte unter diesem Schlüssel entfernen',
  'genericRequest.update.contentRemoveMatchingValuesTitle':
    'Passende aktuelle Werte unter {label} entfernen',
  'genericRequest.update.contentRemoveMatchingValuesTitleGeneric':
    'Passende aktuelle Werte unter diesem Schlüssel entfernen',
  'genericRequest.update.contentRemoveHistory': 'Sie bleiben auf der Blockchain auffindbar.',
  'genericRequest.update.contentRemoveHistoryHash':
    'Diese Anfrage betrifft Werte mit dem Hash {hash}. Sie bleiben auf der Blockchain auffindbar.',
  'genericRequest.update.contentKeyTitle': 'Inhaltsschlüssel: {label}',
  'genericRequest.update.outcome.keepControlTitle': 'Die Wallet behält die Kontrolle',
  'genericRequest.update.outcome.keepControlDescription':
    'Nach dieser Änderung kontrolliert diese Wallet weiterhin alle Primäradressen.',
  'genericRequest.update.outcome.shareControlTitle': 'Die Kontrolle wird geteilt',
  'genericRequest.update.outcome.shareControlDescription':
    'Diese Änderung behält mindestens eine Primäradresse dieser Wallet, fügt aber auch eine externe hinzu.',
  'genericRequest.update.outcome.loseControlTitle': 'Die Wallet verliert die Kontrolle',
  'genericRequest.update.outcome.loseControlDescription':
    'Nach dieser Änderung kontrolliert diese Wallet keine Primäradresse mehr.',
  'genericRequest.update.outcome.clearContentTitle': 'Aktueller Inhalt wird gelöscht',
  'genericRequest.update.outcome.clearContentDescription':
    'Diese Anfrage löscht den aktuellen Inhalt der Identität, ohne die primäre Kontrolle zu ändern.',
  'genericRequest.update.outcome.reviewRequiredTitle': 'Angefragte Identitätsänderungen prüfen',
  'genericRequest.update.outcome.reviewRequiredDescription':
    'Diese Anfrage ändert sensible Felder der Identität. Prüfe sie sorgfältig.',
  'genericRequest.update.badge.inWallet': 'In der Wallet',
  'genericRequest.update.badge.external': 'Extern',
  'genericRequest.update.authority.changeRevocation': 'Widerrufsautorität ändern',
  'genericRequest.update.authority.changeRecovery': 'Wiederherstellungsautorität ändern',
  'genericRequest.update.authority.infoTitle': 'Autoritäten ändern sich',
  'genericRequest.update.authority.infoTitleBoth': 'Wiederherstellung und Widerruf ändern sich',
  'genericRequest.update.authority.infoTitleRecovery':
    'Die Wiederherstellungsautorität ändert sich',
  'genericRequest.update.authority.infoTitleRevocation': 'Die Widerrufsautorität ändert sich',
  'genericRequest.update.authority.infoBody':
    'Änderungen der Autoritäten ändern, wer diese Identität widerrufen oder wiederherstellen kann.',
  'genericRequest.update.authority.infoBodyBoth':
    'Die Wiederherstellungs- und die Widerrufsautorität ändern sich beide. Das ändert, wer diese Identität sperren oder wiederherstellen kann.',
  'genericRequest.update.authority.infoBodyRecovery':
    'Die Wiederherstellungsautorität kann den Zugang nach einem Widerruf wiederherstellen und bei der Wiederherstellung die Kontrolle übernehmen.',
  'genericRequest.update.authority.infoBodyRevocation':
    'Die Widerrufsautorität kann die Identität sperren und eine Wiederherstellung erzwingen, bevor sie wieder nutzbar ist.',
  'genericRequest.update.primaryAddress.afterUpdateTitle': 'Primäradressen nach der Änderung',
  'genericRequest.update.primaryAddress.addTitle': 'Primäradresse hinzufügen',
  'genericRequest.update.primaryAddress.addWalletDescription':
    'Die aktive Wallet kontrolliert diese Primäradresse.',
  'genericRequest.update.primaryAddress.addSharedDescription':
    'Diese Primäradresse ist extern, aber die Wallet kontrolliert nach der Änderung noch eine andere Primäradresse.',
  'genericRequest.update.primaryAddress.addLoseControlDescription':
    'Diese Primäradresse ist extern. Nach der Änderung kontrolliert die Wallet keine Primäradresse mehr.',
  'genericRequest.update.primaryAddress.removeTitle': 'Primäradresse entfernen',
  'genericRequest.update.primaryAddress.removeDescription':
    'Diese Primäradresse kontrolliert die Identität nach der Änderung nicht mehr.',
  'genericRequest.update.status.title': 'Identitätsstatus',
  'genericRequest.update.status.revokedDescription':
    'Diese Änderung markiert die Identität als widerrufen.',
  'genericRequest.update.status.activeDescription':
    'Diese Änderung macht die Identität wieder aktiv.',
  'genericRequest.update.status.revoked': 'Widerrufen',
  'genericRequest.update.status.active': 'Aktiv',
  'genericRequest.update.fundingSourcesLoading': 'Wallet-Guthaben werden geladen…',
  'genericRequest.update.fundingCalculating': 'Gebühr wird berechnet…',
  'genericRequest.update.fundingEmpty':
    'Auf diesem Netzwerk ist keine passende Wallet zum Bezahlen verfügbar.',
  'genericRequest.complete.txidLabel': 'Transaktions-ID',
  'genericRequest.provisioning.error.noSigningAddress':
    'Es ist keine aktive VRSC-Adresse zum Signieren verfügbar.',
  'genericRequest.provisioning.error.noWebhook':
    'Diese Anfrage enthält keine Adresse eines ausstellenden Dienstes.',
  'genericRequest.provisioning.error.invalidRequest':
    'Dieser Anfrage für eine neue VerusID fehlen Pflichtfelder.',
  'genericRequest.provisioning.error.webhook': 'Der ausstellende Dienst hat die Anfrage abgelehnt.',
  'genericRequest.provisioning.error.invalidResponse':
    'Der ausstellende Dienst hat eine ungültige Antwort gesendet.',
  'genericRequest.provisioning.error.invalidResponseSignature':
    'Die Signatur der Antwort des ausstellenden Dienstes konnte nicht geprüft werden.',
  'genericRequest.provisioning.error.failed': 'Der ausstellende Dienst hat einen Fehler gemeldet.',
  'genericRequest.provisioning.error.identityMismatch':
    'Der ausstellende Dienst hat eine andere Identität als angefragt zurückgegeben.',
  'genericRequest.provisioning.error.nameMismatch':
    'Der ausstellende Dienst hat einen anderen Namen als angefragt zurückgegeben.',
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
  'wallet.settings.home.category.profileSecurity': 'Profil und Sicherheit',
  'wallet.settings.home.category.privateVerus': 'Private Verus',
  'wallet.settings.home.category.aboutSupport': 'Info und Support',
  'wallet.settings.home.summary.displayLanguage': '{appearance} · {currency} · {language}',
  'wallet.settings.home.summary.profileSecurity': 'Wiederherstellung und Schlüssel',
  'wallet.settings.home.summary.privateConfigured': 'Eingerichtet',
  'wallet.settings.home.summary.privateNotConfigured': 'Nicht eingerichtet',
  'wallet.settings.home.summary.version': 'Version {version}',
  'wallet.settings.backLabel': 'Zurück zu den Einstellungen',
  'wallet.settings.recovery.backToProfileSecurity': 'Zurück zu Profil und Sicherheit',
  'wallet.settings.recovery.backToPrivateVerus': 'Zurück zu Private Verus',
  'wallet.settings.display.title': 'Anzeige und Sprache',
  'wallet.settings.display.description': 'Wahle deine Anzeigewahrung und App-Sprache.',
  'wallet.settings.display.currency.label': 'Anzeigewährung',
  'wallet.settings.display.currency.current': 'Aktuell: {code}',
  'wallet.settings.display.currency.quickPicks': 'Schnellauswahl',
  'wallet.settings.display.currency.otherAction': 'Andere Wahrung wahlen',
  'wallet.settings.display.currency.sheetTitle': 'Anzeigewährung',
  'wallet.settings.display.currency.searchPlaceholder': 'Währungen suchen',
  'wallet.settings.display.currency.allCurrencies': 'Alle Währungen',
  'wallet.settings.display.currency.resultCount.one': '{count} Währung',
  'wallet.settings.display.currency.resultCount.other': '{count} Währungen',
  'wallet.settings.display.currency.noResults': 'Keine Währung entspricht deiner Suche.',
  'wallet.settings.display.language.label': 'Anzeigesprache',
  'wallet.settings.display.language.description': 'Gilt fur die gesamte App.',
  'wallet.settings.display.appearance.label': 'Erscheinungsbild',
  'wallet.settings.display.appearance.light': 'Hell',
  'wallet.settings.display.appearance.dark': 'Dunkel',
  'wallet.settings.display.appearance.system': 'System',
  'wallet.settings.display.appearance.systemDescription':
    'Verwende das helle oder dunkle Erscheinungsbild deines Geräts.',

  'wallet.settings.profile.title': 'Profil und Sicherheit',
  'wallet.settings.profile.description':
    'Verwalte Auto-Sperre und Wiederherstellungszugang fur diese Wallet.',
  'wallet.settings.profile.autoLock.title': 'Automatisch sperren nach Inaktivität',
  'wallet.settings.profile.autoLock.description':
    'Sperrt diese Wallet nach Inaktivitat. Aus ist nicht verfugbar.',
  'wallet.settings.profile.autoLock.option': '{minutes} Min',
  'wallet.settings.profile.recovery.title': 'Wiederherstellung und Schlüssel',
  'wallet.settings.profile.recovery.description':
    'Zeige Wiederherstellungsgeheimnisse und abgeleitete Schlussel nach Passwortbestatigung an.',

  'wallet.transfer.error.dlightBroadcastUncertain':
    'Die Übermittlung konnte nicht bestätigt werden. Prüfe den Transaktionsverlauf, bevor du es erneut versuchst.',
  'wallet.transfer.error.ethBroadcastUncertain':
    'Die Ethereum-Übermittlung konnte nicht bestätigt werden. Setze die gespeicherte Übertragung fort, statt erneut zu senden.',
  'wallet.transfer.error.ethBroadcastRecovered':
    'Die gespeicherte Ethereum-Übertragung wurde wiederhergestellt.',
  'wallet.transfer.error.ethRecoveryRequired':
    'Schließe die gespeicherte Ethereum-Übertragung ab, bevor du eine neue startest.',
  'wallet.transfer.ethRecovery.title': 'Ethereum-Übertragung wiederherstellen',
  'wallet.transfer.ethRecovery.description': 'Prüfe die Transaktionsdetails, bevor du fortfährst.',
  'wallet.transfer.ethRecovery.continue': 'Übertragung fortsetzen',
  'wallet.transfer.ethRecovery.showResult': 'Gesendete Übertragung anzeigen',
  'wallet.transfer.ethRecovery.recovering': 'Wiederherstellung…',
  'wallet.transfer.ethRecovery.error.stageAdvanced':
    'Die Übertragung wurde fortgesetzt, aber der nächste Schritt konnte nicht gesendet werden. Prüfe die aktualisierten Details und fahre dann erneut fort.',
  'wallet.transfer.ethRecovery.error.retry':
    'Die Übertragung konnte nicht fortgesetzt werden. Prüfe diese Details und versuche es erneut.',
  'wallet.transfer.ethRecovery.error.noPending':
    'Es steht keine Wiederherstellung mehr aus. Prüfe deine Übertragungsdetails, bevor du es erneut versuchst.',
  'wallet.transfer.ethRecovery.error.refreshFailed':
    'Der Wiederherstellungsstatus konnte nicht aktualisiert werden. Prüfe die zuletzt bekannten Details und versuche es erneut.',
  'wallet.transfer.ethRecovery.network': 'Netzwerk',
  'wallet.transfer.ethRecovery.chain': 'Chain {value}',
  'wallet.transfer.ethRecovery.stage': 'Wiederherstellungsphase',
  'wallet.transfer.ethRecovery.assetContract': 'Asset-Contract',
  'wallet.transfer.ethRecovery.bridgeContract': 'Bridge-Contract',
  'wallet.transfer.ethRecovery.mappedCurrency': 'Zugeordnete Währung',
  'wallet.transfer.ethRecovery.destinationSystem': 'Zielsystem',
  'wallet.transfer.ethRecovery.stage.eth': 'Native Übertragung',
  'wallet.transfer.ethRecovery.stage.erc20': 'Token-Übertragung',
  'wallet.transfer.ethRecovery.stage.bridge_zero_approval': 'Token-Freigabe zurücksetzen',
  'wallet.transfer.ethRecovery.stage.bridge_approval': 'Bridge-Zugriff freigeben',
  'wallet.transfer.ethRecovery.stage.bridge_transfer': 'Bridge-Übertragung senden',
  'wallet.transfer.ethRecovery.stage.unknown': 'Gespeicherte Übertragung',
  'wallet.settings.privateVerus.title': 'Privatsphare',
  'wallet.settings.privateVerus.description':
    'Konfiguriere shielded Privatsphare fur {label}. Du kannst deine primare geheime Wiederherstellungsphrase wiederverwenden oder ein separates Privacy-Wiederherstellungsgeheimnis setzen.',
  'wallet.settings.privateVerus.statusConfigured': 'Privatsphare ist konfiguriert.',
  'wallet.settings.privateVerus.statusNotConfigured': 'Privatsphare ist noch nicht konfiguriert.',
  'wallet.settings.privateVerus.statusAddress': 'Shielded-Adresse',
  'wallet.settings.privateVerus.statusLoadError':
    'Privatsphare-Status konnte nicht geladen werden.',
  'wallet.settings.privateVerus.reusePrimary':
    'Primare geheime Wiederherstellungsphrase wiederverwenden',
  'wallet.settings.privateVerus.createNew': 'Neues Privacy-Wiederherstellungsgeheimnis erstellen',
  'wallet.settings.privateVerus.importLabel':
    'Privacy-Wiederherstellungsgeheimnis oder Spending Key importieren',
  'wallet.settings.privateVerus.importPlaceholder':
    'Geheime Wiederherstellungsphrase oder secret-extended-key-main einfugen…',
  'wallet.settings.privateVerus.importAction': 'Privacy-Wiederherstellungsgeheimnis importieren',
  'wallet.settings.privateVerus.settingUp': 'Privatsphare wird konfiguriert…',
  'wallet.settings.privateVerus.generatedSeedTitle': 'Neue geheime Wiederherstellungsphrase',
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
  'wallet.settings.recovery.copyFailed':
    'Dieser Wert konnte nicht kopiert werden. Versuchen Sie es erneut.',
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
  'wallet.settings.recovery.field.dlightSecret':
    'Privacy-Wiederherstellungsgeheimnis oder Spending Key',
  'wallet.settings.recovery.field.dlightShieldedAddress': 'Privacy shielded Adresse',
  'wallet.settings.recovery.field.dlightDerivedSpendingKey': 'Abgeleiteter privater Spending Key',
  'wallet.settings.recovery.kind.seedText': 'Benutzerdefinierter Seed',
  'wallet.settings.recovery.kind.wif': 'WIF',
  'wallet.settings.recovery.kind.privateKeyHex': 'Private Key (hex)',
  'wallet.settings.recovery.kind.dlightMnemonic': 'Geheime Wiederherstellungsphrase',
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
  'wallet.assetDetails.back': 'Zurück zur Wallet',
  'wallet.assetDetails.loadOlderTransactions': 'Alteren Verlauf prufen',
  'wallet.assetDetails.errorLoadScopes': 'Subwallet-Scopes konnten nicht geladen werden.',
  'wallet.assetDetails.scopeUnavailable': 'Fur dieses Asset ist kein Scope verfugbar.',
  'wallet.assetDetails.scopePicker': 'Adresse und Netzwerk andern',
  'wallet.assetDetails.readOnlyHelper':
    'Senden und Konvertieren sind nur von deiner primaren Adresse verfugbar.',
  'wallet.assetDetails.privateSyncInlineHelper': 'Bestand wird synchronisiert',
  'wallet.assetDetails.sendCapabilityInline': 'Send-Sync {percent}%',
  'wallet.manageAssets.back': 'Zurück zur Asset-Verwaltung',
  'wallet.manageAssets.foundCount.one': '{count} Asset in deiner Wallet gefunden',
  'wallet.manageAssets.foundCount.other': '{count} Assets in deiner Wallet gefunden',
  'wallet.manageAssets.foundOutsidePortfolioCount.one':
    '{count} Asset außerhalb deines Portfolios gefunden',
  'wallet.manageAssets.foundOutsidePortfolioCount.other':
    '{count} Assets außerhalb deines Portfolios gefunden',
  'help.backToTopics': 'Zurück zu den Hilfethemen',
  'genericRequest.flow.close': 'Anfrage schließen',
  'genericRequest.flow.requestFrom': 'Anfrage von {app}',
  'genericRequest.flow.notSigned': 'Nicht signiert',
  'genericRequest.flow.signatureChecked': 'Signatur geprüft',
  'genericRequest.flow.expired': 'Abgelaufen',
  'genericRequest.flow.timeLeft.minutes': 'Noch {count} Min.',
  'genericRequest.flow.timeLeft.hours': 'Noch etwa {count} Stunden',
  'genericRequest.flow.stepsLabel': 'Schritte der Anfrage',
  'genericRequest.flow.respondsAs': 'als {identity}',
  'genericRequest.flow.respondAs': 'Antworten als',
  'genericRequest.flow.chooseIdentity': 'VerusID wählen',
  'genericRequest.flow.noMatchingIdentity': 'Keine passende VerusID',
  'genericRequest.flow.noMatchingIdentityHint':
    'Verknüpfe eine VerusID, die die Regeln dieser Anfrage erfüllt.',
  'genericRequest.flow.payFeeFrom': 'Gebühr zahlen von',
  'genericRequest.flow.chooseSource': 'Quelle wählen',
  'genericRequest.flow.feeAfterChoice': 'Die Gebühr erscheint nach deiner Wahl',
  'genericRequest.flow.warning.noEffect': 'Diese Anfrage ändert deine VerusID nicht.',
  'genericRequest.flow.warning.dustChange':
    'Das Wechselgeld ist zu klein für eine Rückzahlung und wird der Netzwerkgebühr hinzugefügt.',
  'genericRequest.flow.warning.other': 'Prüfe diese Anfrage sorgfältig, bevor du fortfährst.',
  'genericRequest.flow.networkFee': 'Netzwerkgebühr',
  'genericRequest.flow.feeValue': '{fee} {currency}',
  'genericRequest.flow.highRisk': 'Hohes Risiko',
  'genericRequest.flow.step.signIn': 'Anmelden',
  'genericRequest.flow.step.newIdentity': 'Neue VerusID',
  'genericRequest.flow.step.updateChanges': 'Identität aktualisieren',
  'genericRequest.flow.step.networkFee': 'Netzwerkgebühr',
  'genericRequest.flow.step.confirm': 'Bestätigen',
  'genericRequest.flow.title.signIn': 'Bei {app} anmelden',
  'genericRequest.flow.title.newIdentity': 'Neue VerusID erhalten',
  'genericRequest.flow.title.updateChanges': 'Änderungen an {identity} prüfen',
  'genericRequest.flow.title.networkFee': 'Wähle, wer die Gebühr zahlt',
  'genericRequest.flow.title.confirm': 'Bestätigen',
  'genericRequest.flow.signIn.learns': 'Die Website erfährt Name und ID deiner VerusID.',
  'genericRequest.flow.signIn.requirements': 'Anforderungen',
  'genericRequest.flow.newIdentity.offer': 'Neue VerusID',
  'genericRequest.flow.newIdentity.chooseName': 'Name deiner neuen VerusID',
  'genericRequest.flow.newIdentity.nameHint': 'Wähle einen Namen, der noch frei ist.',
  'genericRequest.flow.newIdentity.nameInvalid':
    'Ein Name darf kein \\ / : * ? " < > | @ oder keinen Punkt enthalten und nicht mit einem Leerzeichen beginnen oder enden.',
  'genericRequest.flow.newIdentity.nameChecking': 'Name wird geprüft…',
  'genericRequest.flow.newIdentity.nameTaken': 'Dieser Name ist vergeben. Wähle einen anderen.',
  'genericRequest.flow.newIdentity.nameCheckFailed':
    'Der Name konnte nicht geprüft werden. Versuche es erneut.',
  'genericRequest.flow.newIdentity.nameAvailable': 'Dieser Name ist frei.',
  'genericRequest.flow.newIdentity.chosen': 'Eine neue VerusID, {name}, beantwortet diese Anfrage.',
  'genericRequest.flow.newIdentity.chosenUnnamed': 'Eine neue VerusID beantwortet diese Anfrage.',
  'genericRequest.flow.newIdentity.name': 'Neue VerusID',
  'genericRequest.flow.newIdentity.issuer': 'Ausgestellt von',
  'genericRequest.flow.newIdentity.network': 'Netzwerk',
  'genericRequest.flow.newIdentity.note':
    '{issuer} registriert sie auf der Blockchain. Das dauert einige Minuten. Danach verknüpfst du sie, und die Anmeldung wird abgeschlossen.',
  'genericRequest.flow.plan.signIn': 'Bei {app} als {identity} anmelden',
  'genericRequest.flow.plan.newIdentity': '{issuer} um {name} bitten',
  'genericRequest.flow.plan.update': '{identity} aktualisieren',
  'genericRequest.flow.plan.changes.one': '{count} Änderung',
  'genericRequest.flow.plan.changes.other': '{count} Änderungen',
  'genericRequest.flow.delivery.redirect': 'Öffnet {app} in deinem Browser',
  'genericRequest.flow.delivery.none': 'Es wird keine Antwort gesendet',
  'genericRequest.flow.delivery.webhook': 'Die Anfrage geht an {issuer}',
  'genericRequest.flow.action.signIn': 'Anmelden',
  'genericRequest.flow.action.signInAndUpdate': 'Anmelden und aktualisieren',
  'genericRequest.flow.action.update': 'Identität aktualisieren',
  'genericRequest.flow.action.newIdentity': '{name} anfordern',
  'genericRequest.flow.working.signIn': 'Anmeldung läuft…',
  'genericRequest.flow.working.update': 'Update wird gesendet…',
  'genericRequest.flow.working.newIdentity': 'Anfrage wird gesendet…',
  'genericRequest.flow.decline': 'Ablehnen',
  'genericRequest.flow.stage.transaction.pending': 'Update senden',
  'genericRequest.flow.stage.transaction.running': 'Update wird gesendet…',
  'genericRequest.flow.stage.transaction.done': 'Update gesendet',
  'genericRequest.flow.stage.transaction.failed': 'Update nicht gesendet',
  'genericRequest.flow.stage.signing.pending': 'Antwort signieren',
  'genericRequest.flow.stage.signing.running': 'Antwort wird signiert…',
  'genericRequest.flow.stage.signing.done': 'Antwort signiert',
  'genericRequest.flow.stage.signing.failed': 'Antwort nicht signiert',
  'genericRequest.flow.stage.delivery.pending': 'An {app} senden',
  'genericRequest.flow.stage.delivery.running': 'Wird an {app} gesendet…',
  'genericRequest.flow.stage.delivery.done': 'Antwort zugestellt',
  'genericRequest.flow.stage.delivery.failed': 'Signiert, nicht zugestellt',
  'genericRequest.flow.stage.redirect.pending': '{app} öffnen',
  'genericRequest.flow.stage.redirect.running': '{app} wird geöffnet…',
  'genericRequest.flow.stage.redirect.done': '{app} geöffnet',
  'genericRequest.flow.stage.redirect.failed': 'Signiert, {app} nicht geöffnet',
  'genericRequest.flow.stage.webhook.pending': 'Anfrage an {issuer} senden',
  'genericRequest.flow.stage.webhook.running': 'Anfrage wird an {issuer} gesendet…',
  'genericRequest.flow.stage.webhook.done': 'Anfrage an {issuer} gesendet',
  'genericRequest.flow.stage.webhook.failed': 'Anfrage nicht gesendet',
  'genericRequest.flow.outcome.failed': 'Anfrage nicht abgeschlossen',
  'genericRequest.flow.outcome.signedIn': 'Du bist bei {app} angemeldet',
  'genericRequest.flow.outcome.updated': '{identity} ist aktualisiert',
  'genericRequest.flow.outcome.newIdentity': '{name} ist unterwegs',
  'genericRequest.flow.outcome.newIdentityAfter':
    'Verknüpfe {name}, sobald sie bereit ist. Dann wird die Anmeldung abgeschlossen.',
  'genericRequest.flow.outcome.declined': 'Anfrage abgelehnt',
  'genericRequest.flow.outcome.declinedBody':
    'Nichts wurde signiert oder gesendet. {app} wurde nicht informiert.',
  'genericRequest.flow.outcome.expired': 'Diese Anfrage ist abgelaufen',
  'genericRequest.flow.outcome.expiredBody': 'Beginne auf der Website neu.',
  'genericRequest.flow.receipt.signedInAs': 'Angemeldet als',
  'genericRequest.flow.copyResponse': 'Antwort kopieren',
  'genericRequest.flow.copyTransaction': 'Transaktions-ID kopieren',
  'genericRequest.flow.openSite': 'Stattdessen Website öffnen',
  'genericRequest.flow.stageError.transaction': 'Das Update konnte nicht gesendet werden.',
  'genericRequest.flow.stageError.signing': 'Die Antwort konnte nicht signiert werden.',
  'genericRequest.flow.stageError.delivery': '{app} hat die Antwort nicht angenommen.',
  'genericRequest.flow.stageError.webhook': '{issuer} hat die Anfrage nicht angenommen.',
  'genericRequest.flow.noChanges': 'Dieses Update ändert nichts, was die Wallet anzeigen kann.',
  'genericRequest.flow.secondLinkIgnored':
    'Schließe die aktuelle Anfrage ab und öffne den neuen Link dann erneut.',
  'genericRequest.flow.step.payInvoice': 'Rechnung bezahlen',
  'genericRequest.flow.title.payInvoice': '{app} bezahlen',
  'genericRequest.flow.plan.pay': '{amount} an {to} zahlen',
  'genericRequest.flow.action.pay': '{amount} zahlen',
  'genericRequest.flow.action.signInAndPay': 'Anmelden und zahlen',
  'genericRequest.flow.working.pay': 'Zahlung wird gesendet…',
  'genericRequest.flow.stage.payment.pending': 'Zahlung senden',
  'genericRequest.flow.stage.payment.running': 'Zahlung wird gesendet…',
  'genericRequest.flow.stage.payment.done': 'Zahlung gesendet',
  'genericRequest.flow.stage.payment.failed': 'Zahlung nicht gesendet',
  'genericRequest.flow.outcome.paid': 'Du hast {amount} an {to} gezahlt',
  'genericRequest.flow.invoice.amount': 'Betrag',
  'genericRequest.flow.invoice.to': 'An',
  'genericRequest.flow.invoice.amountInput': 'Betrag in {currency}',
  'genericRequest.flow.invoice.payFrom': 'Zahlen von',
  'genericRequest.flow.invoice.noSource': 'Keine Adresse in dieser Wallet hat {currency}.',
  'genericRequest.flow.invoice.total': 'Gesamt',
  'genericRequest.flow.invoice.totalSplit': '{amount} und {fee}',
  'genericRequest.flow.invoice.paidTo': 'Gezahlt an',
  'genericRequest.flow.invoice.loading': 'Rechnung wird gelesen…',
  'genericRequest.flow.invoice.error.review': 'Diese Rechnung konnte nicht gelesen werden.',
  'genericRequest.flow.invoice.error.funds':
    'Nicht genug {currency} für diese Rechnung und die Gebühr. Eine noch nicht bestätigte Zahlung kann einen Teil deines Guthabens etwa eine Minute lang binden.',
  'genericRequest.flow.invoice.error.preflight': 'Diese Zahlung konnte nicht vorbereitet werden.',
  'genericRequest.rejected.invoiceTagged':
    'Diese Rechnung verlangt eine Zahlung mit Tag, die diese Wallet nicht sendet.',
  'genericRequest.rejected.invoiceUnpayable':
    'Diese Rechnung kann nicht auf der Verus-Chain bezahlt werden.',
  'genericRequest.rejected.invoicePrivate':
    'Diese Rechnung verlangt eine private Zahlung, die diese Wallet nicht sendet.',
  'genericRequest.rejected.invoiceNoDestination':
    'Diese Rechnung nennt keine Adresse für die Zahlung.',
  'genericRequest.rejected.invoiceCurrency':
    'Die Währung dieser Rechnung ist in diesem Netzwerk unbekannt.',
  'genericRequest.rejected.invoiceCurrencyMissing':
    'Füge die Währung dieser Rechnung deiner Wallet hinzu, um sie zu bezahlen.',
  'genericRequest.rejected.invoicePreconvertLaunched':
    'Diese Rechnung konvertiert vorab in eine Währung, die bereits gestartet ist.',
  'genericRequest.rejected.alreadyPaid': 'Du hast diese Rechnung bereits bezahlt.',
  'genericRequest.flow.step.signData': 'Daten signieren',
  'genericRequest.flow.title.signData': 'Für {app} signieren',
  'genericRequest.flow.signData.intro':
    '{app} bittet dich, jeden Eintrag unten zu signieren. Jede Signatur zeigt, dass du diesem Eintrag zustimmst.',
  'genericRequest.flow.signData.message': 'Nachricht',
  'genericRequest.flow.signData.data': 'Daten',
  'genericRequest.flow.signData.statement': 'Erklärung',
  'genericRequest.flow.signData.size': '{size} Bytes',
  'genericRequest.flow.signData.bytes': 'Bytes',
  'genericRequest.flow.signData.hash': 'SHA-256, den du signierst',
  'genericRequest.flow.signData.consent': '{item} signieren',
  'genericRequest.flow.signData.allNeeded': 'Hake jeden Eintrag an, um fortzufahren.',
  'genericRequest.flow.plan.signData.one': '{count} Eintrag für {app} als {identity} signieren',
  'genericRequest.flow.plan.signData.other': '{count} Einträge für {app} als {identity} signieren',
  'genericRequest.flow.action.sign': 'Signieren',
  'genericRequest.flow.action.signInAndSign': 'Anmelden und signieren',
  'genericRequest.flow.working.sign': 'Wird signiert…',
  'genericRequest.flow.outcome.signedData.one': 'Du hast {count} Eintrag für {app} signiert',
  'genericRequest.flow.outcome.signedData.other': 'Du hast {count} Einträge für {app} signiert',
  'genericRequest.flow.receipt.signedAs': 'Signiert als',
  'genericRequest.rejected.dataTransmittal':
    'Diese Anfrage sendet dir Daten, statt um eine Signatur zu bitten. Diese Wallet nimmt das nicht an.',
  'genericRequest.rejected.dataNotForSignature':
    'Diese Anfrage bittet dich nicht, etwas zu signieren.',
  'genericRequest.flow.step.encryptionKey': 'Verschlüsselungsschlüssel',
  'genericRequest.flow.title.encryptionKey': 'Einen Verschlüsselungsschlüssel mit {app} teilen',
  'genericRequest.flow.encryptionKey.app': 'App',
  'genericRequest.flow.encryptionKey.number': 'Schlüsselnummer',
  'genericRequest.flow.encryptionKey.shares': 'Teilt',
  'genericRequest.flow.encryptionKey.sharesViewing':
    'Einen Anzeigeschlüssel und eine private Adresse für diese App',
  'genericRequest.flow.encryptionKey.sharesWithSpendingKey':
    'Einen Anzeigeschlüssel, eine private Adresse und ihren Ausgabeschlüssel',
  'genericRequest.flow.encryptionKey.checking': 'Private Adressen werden geprüft…',
  'genericRequest.flow.encryptionKey.setupTitle': 'Richte zuerst private Adressen ein',
  'genericRequest.flow.encryptionKey.setupBody':
    'Diese App braucht eine private Adresse aus dieser Wallet. Richte private Adressen ein und öffne den Link der Anfrage dann erneut.',
  'genericRequest.flow.encryptionKey.setupAction': 'Private Adressen einrichten',
  'genericRequest.flow.encryptionKey.spendingKeyTitle':
    'Diese App bittet auch um einen Ausgabeschlüssel',
  'genericRequest.flow.encryptionKey.spendingKeyBody':
    'Damit kann die App alles ausgeben, was an diese private Adresse geht. Teile ihn nur mit einer App, der du dieses Geld anvertraust.',
  'genericRequest.flow.encryptionKey.spendingKeyAcknowledge':
    'Ich verstehe, dass diese App von dieser Adresse ausgeben kann.',
  'genericRequest.flow.encryptionKey.error.status':
    'Private Adressen konnten nicht geprüft werden.',
  'genericRequest.flow.plan.encryptionKey':
    'Einen Verschlüsselungsschlüssel mit {app} als {identity} teilen',
  'genericRequest.flow.action.shareKey': 'Schlüssel teilen',
  'genericRequest.flow.working.shareKey': 'Schlüssel wird geteilt…',
  'genericRequest.flow.outcome.keyShared': '{app} hat einen Verschlüsselungsschlüssel',
  'genericRequest.flow.receipt.sharedAs': 'Geteilt als',
  'genericRequest.rejected.encryptionKeyAddress':
    'Diese Anfrage nennt eine ungültige Verschlüsselungsadresse oder bittet ohne eine um einen Ausgabeschlüssel.',
  'genericRequest.rejected.encryptionKeyIdentity':
    'Die Identität, für die dieser Schlüssel ist, existiert nicht.',
  'genericRequest.rejected.encryptionKeyChannel':
    'Dieser Schlüssel würde auch die auf dieser VerusID gespeicherten Nachweise entsperren, daher teilt die Wallet ihn nicht.',
  'genericRequest.rejected.encryptionKeyNumber':
    'Diese Anfrage fragt nach einer ungültigen Schlüsselnummer.',
  'genericRequest.rejected.privateNotSetUp': 'Richte zuerst private Adressen ein.',
  'genericRequest.flow.step.shareData': 'Daten teilen',
  'genericRequest.flow.title.shareData': 'Nachweise mit {app} teilen',
  'genericRequest.flow.shareData.intro':
    '{app} fragt diese Nachweise von {identity} an. Sie werden verschlüsselt gesendet, sodass nur {app} sie lesen kann.',
  'genericRequest.flow.shareData.loading': 'Passende Nachweise werden gesucht…',
  'genericRequest.flow.shareData.consent': '{credential} teilen',
  'genericRequest.flow.shareData.allNeeded': 'Hake jeden Nachweis an, um fortzufahren.',
  'genericRequest.flow.shareData.missing.one':
    '{count} angefragter Nachweis wurde nicht gefunden. Er wird nicht gesendet.',
  'genericRequest.flow.shareData.missing.other':
    '{count} angefragte Nachweise wurden nicht gefunden. Sie werden nicht gesendet.',
  'genericRequest.flow.shareData.none': 'Auf {identity} wurden keine passenden Nachweise gefunden.',
  'genericRequest.flow.shareData.nothingToSend':
    'Es gibt nichts zu teilen. Lehne ab, um diese Anfrage zu schließen.',
  'genericRequest.flow.shareData.error.lookup': 'Die Nachweise konnten nicht gelesen werden.',
  'genericRequest.flow.plan.shareData.one': '{count} Nachweis mit {app} als {identity} teilen',
  'genericRequest.flow.plan.shareData.other': '{count} Nachweise mit {app} als {identity} teilen',
  'genericRequest.flow.action.share': 'Teilen',
  'genericRequest.flow.action.signInAndShare': 'Anmelden und teilen',
  'genericRequest.flow.working.share': 'Wird geteilt…',
  'genericRequest.flow.outcome.sharedData.one': 'Du hast {count} Nachweis mit {app} geteilt',
  'genericRequest.flow.outcome.sharedData.other': 'Du hast {count} Nachweise mit {app} geteilt',
  'genericRequest.rejected.credentialsOnly':
    'Diese Anfrage fragt nach anderen Daten als vollständigen Nachweisen. Diese Wallet teilt nur vollständige Nachweise.',
  'genericRequest.rejected.credentialsNotEncrypted':
    'Diese Anfrage fragt nach Nachweisen, nennt aber keine Adresse zum Verschlüsseln. Sie werden daher nicht gesendet.',
  'genericRequest.rejected.credentialAddress':
    'Die private Adresse dieser VerusID stammt nicht aus dieser Wallet, daher können darauf keine Nachweise gespeichert werden.',
  'genericRequest.rejected.loginContext':
    'Diese Anmeldeanfrage möchte zusätzliche Sitzungsdaten teilen. Diese Wallet unterstützt das nicht.',
  'genericRequest.rejected.loginPermission':
    'Diese Anmeldeanfrage fragt nach mehr als Name und ID deiner VerusID. Diese Wallet teilt nur diese.',
  'genericRequest.flow.step.giftCard': 'Geschenkkarte',
  'genericRequest.flow.title.giftCard': 'Eine Geschenkkarte einlösen',
  'genericRequest.flow.giftCard.passwordIntro':
    'Diese Geschenkkarte hat ein Passwort. Gib es ein, um zu sehen, was die Karte enthält.',
  'genericRequest.flow.giftCard.password': 'Passwort der Geschenkkarte',
  'genericRequest.flow.giftCard.unlock': 'Geschenkkarte öffnen',
  'genericRequest.flow.giftCard.loading': 'Geschenkkarte wird gelesen…',
  'genericRequest.flow.giftCard.empty': 'Diese Geschenkkarte ist leer. Es gibt nichts einzulösen.',
  'genericRequest.flow.giftCard.intro': 'Alles unten geht an diese Wallet.',
  'genericRequest.flow.giftCard.holds': 'Diese Geschenkkarte enthält:',
  'genericRequest.flow.giftCard.cannotMove': 'Kann nicht in diese Wallet',
  'genericRequest.flow.giftCard.moves': 'Die Kontrolle geht an diese Wallet',
  'genericRequest.flow.giftCard.blocked':
    '{identity} braucht mehr als eine Signatur oder ist nicht aktiv. Diese Geschenkkarte kann hier daher nicht eingelöst werden.',
  'genericRequest.flow.giftCard.authorities':
    '{identity} behält ihre Widerrufs- und Wiederherstellungsautorität. Wer diese kontrolliert, kann sie weiterhin widerrufen oder wiederherstellen.',
  'genericRequest.flow.giftCard.preparing': 'Einlösung wird vorbereitet…',
  'genericRequest.flow.giftCard.claimed': 'Eingelöst',
  'genericRequest.flow.giftCard.next': 'Nächster Schritt',
  'genericRequest.flow.giftCard.linkAfter':
    'Verknüpfe die VerusID unter VerusID, nachdem die Änderung bestätigt ist.',
  'genericRequest.flow.giftCard.error.review': 'Die Geschenkkarte konnte nicht gelesen werden.',
  'genericRequest.flow.giftCard.error.preflight': 'Die Einlösung konnte nicht vorbereitet werden.',
  'genericRequest.flow.giftCard.error.fee':
    'Die Geschenkkarte enthält nicht genug für die Netzwerkgebühr.',
  'genericRequest.flow.plan.giftCard': '{items} in diese Wallet einlösen',
  'genericRequest.flow.action.claim': 'Einlösen',
  'genericRequest.flow.working.claim': 'Wird eingelöst…',
  'genericRequest.flow.outcome.claimed': 'Geschenkkarte eingelöst',
  'genericRequest.flow.stage.claim.pending': 'Einlösung senden',
  'genericRequest.flow.stage.claim.running': 'Einlösung wird gesendet…',
  'genericRequest.flow.stage.claim.done': 'Einlösung gesendet',
  'genericRequest.flow.stage.claim.failed': 'Einlösung nicht abgeschlossen',
  'genericRequest.flow.stageError.claim':
    'Ein Teil der Einlösung wurde nicht gesendet. Versuche es erneut, um den Rest zu senden.',
  'genericRequest.rejected.giftCardInvalid':
    'Diese Geschenkkarte hat ein Format, das diese Wallet nicht lesen kann.',
  'genericRequest.rejected.giftCardPassword': 'Das Passwort passt nicht zu dieser Geschenkkarte.',
  'genericRequest.rejected.giftCardEmpty':
    'Diese Geschenkkarte ist leer. Es gibt nichts einzulösen.',
  'genericRequest.rejected.giftCardIdentity':
    'Eine VerusID auf dieser Geschenkkarte kann hier nicht übertragen werden. Die Geschenkkarte kann daher nicht eingelöst werden.',
  'genericRequest.rejected.unknownDetail':
    'Diese Anfrage enthält einen Teil, den diese Wallet nicht erkennt.',
  'genericRequest.rejected.detailOrder':
    'Diese Anfrage ist nicht richtig aufgebaut. Bitte die Website um einen neuen Link.',
  'genericRequest.rejected.unsigned':
    'Diese Anfrage ist nicht signiert. Nur Zahlungen und Geschenkkarten dürfen unsigniert sein.',
  'genericRequest.rejected.walletBackup':
    'Für Wallet-Backups auf NFC-Karten wird Verus Mobile benötigt.',
  'genericRequest.rejected.network':
    'Diese Anfrage ist für ein anderes Netzwerk als die geöffnete Wallet.',
  'genericRequest.rejected.encryptedDetails':
    'Diese Anfrage ist verschlüsselt, daher kann diese Wallet sie nicht lesen.',
  'genericRequest.rejected.tooLong':
    'Dieser Link ist zu lang. Bitte die Website um einen kürzeren Link.',
  'genericRequest.rejected.expired':
    'Diese Anfrage ist abgelaufen. Bitte die Website um eine neue Anfrage.',
  'genericRequest.rejected.delegatedSigner':
    'Diese Anfrage wurde für eine andere App von einer VerusID signiert, die diese Wallet nicht verwaltet.',
  'genericRequest.rejected.signature': 'Die Signatur der Anfrage konnte nicht geprüft werden.',
  'genericRequest.rejected.notOpen':
    'Diese Anfrage ist nicht mehr geöffnet. Öffne den Link erneut.',
  'genericRequest.rejected.responder':
    'Die gewählte VerusID erfüllt die Anforderungen dieser Anfrage nicht.',
  'genericRequest.rejected.unsupportedDetail':
    'Ein Teil dieser Anfrage wird auf dem Desktop noch nicht unterstützt.',
  'genericRequest.rejected.responseAddress':
    'Die Antwort kann nur an eine Adresse gehen, die in der Anfrage steht.',
  'genericRequest.rejected.insecureDelivery':
    'Die Website bot keine sichere Verbindung an, daher wurde die Antwort nicht gesendet.',
  'genericRequest.flow.error.expired': 'Diese Anfrage ist abgelaufen. Beginne auf der Website neu.',
  'genericRequest.update.privateAddressChange.title': 'Private Adresse ändern',
  'genericRequest.update.privateAddressChange.description':
    'Private Zahlungen an diese VerusID können an diese Adresse gehen.',
};
