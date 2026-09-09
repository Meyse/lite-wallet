import { en } from './en';

export const es: Record<string, string> = {
  ...en,
  'common.loading': 'Cargando…',
  'common.cancel': 'Cancelar',
  'common.continue': 'Continuar',
  'common.back': 'Atras',
  'common.done': 'Listo',
  'common.retry': 'Intentar de nuevo',
  'common.unknownError': 'Error desconocido',
  'common.network.mainnet': 'Mainnet',
  'common.network.testnet': 'Testnet',

  'welcome.titleLine1': 'Tu acceso al',
  'welcome.titleLine2': 'nuevo internet',
  'welcome.cta.start': 'Crear nueva cartera',
  'welcome.cta.existing': 'Usar una cartera existente',

  'unlock.switch': 'Cambiar',
  'unlock.switchLabel': 'Cambiar cartera, cartera actual: {name}',
  'unlock.password': 'Contrasena',
  'unlock.button.unlocking': 'Desbloqueando…',
  'unlock.button.unlock': 'Desbloquear',
  'unlock.button.createWallet': 'Crear nueva cartera',
  'unlock.error.invalidPassword': 'Contrasena incorrecta. Intentalo de nuevo.',
  'unlock.error.operationFailed':
    'No se pudo desbloquear la cartera en este dispositivo. Intentalo de nuevo o vuelve a crearla.',
  'unlock.error.invalidArgs':
    'La solicitud de desbloqueo no era valida. Reinicia la app e intentalo de nuevo.',
  'unlock.error.generic': 'No se puede desbloquear la cartera ahora. Intentalo de nuevo.',
  'unlock.switcher.title': 'Elegir cartera',
  'unlock.create.title': 'Crear o importar cartera',
  'unlock.create.newTitle': 'Crear una cartera nueva',
  'unlock.create.newDescription':
    'Genera una nueva frase secreta de recuperacion y configura una cartera nueva.',
  'unlock.create.importTitle': 'Importar una cartera existente',
  'unlock.create.importDescription':
    'Usa tu frase secreta de recuperacion para restaurar una cartera en este dispositivo.',
  'unlock.importMethods.title': 'Importar cartera',
  'unlock.importMethods.back': 'Atras',
  'unlock.importMethods.seed24Title': 'Importar frase secreta de recuperacion',
  'unlock.importMethods.seed24Description':
    'Ingresa tu frase secreta de recuperacion de 24 palabras para restaurar esta cartera.',
  'unlock.importMethods.textTitle': 'Pegar clave privada o semilla personalizada',
  'unlock.importMethods.textDescription':
    'Pega manualmente una clave privada, frase secreta de recuperacion o semilla personalizada.',

  'guard.mode.revoke': 'Revocar',
  'guard.mode.recover': 'Recuperar',
  'guard.mode.revokeLower': 'revocar',
  'guard.mode.recoverLower': 'recuperar',
  'guard.flow.review.fee': 'Comision estimada',
  'guard.flow.result.txidLabel': 'ID de transaccion',

  'languageGate.title': 'Elige tu idioma',
  'languageGate.button': 'Continuar',
  'languageGate.option.en': 'English',
  'languageGate.option.nl': 'Nederlands',
  'languageGate.option.de': 'Deutsch',
  'languageGate.option.es': 'Español',

  'wallet.layout.dismiss': 'Cerrar',
  'wallet.layout.noticeTitle': 'Aviso de la cartera',
  'wallet.layout.backgroundError.title': 'Actualizacion retrasada',
  'wallet.layout.backgroundError.balance':
    'No se pudieron actualizar algunos saldos. Es posible que los valores mostrados esten desactualizados.',
  'wallet.layout.backgroundError.info':
    'No se pudo actualizar el estado de la red. Vuelve a intentarlo si el problema persiste.',
  'wallet.layout.backgroundError.transactionsWarning': 'Es posible que falte actividad reciente.',
  'wallet.layout.backgroundError.transactions':
    'No se pudo actualizar la actividad reciente. Vuelve a intentarlo si el problema persiste.',
  'wallet.layout.backgroundError.generic':
    'No se pudieron actualizar los datos de la cartera. Vuelve a intentarlo si el problema persiste.',
  'wallet.sidebar.wallet': 'Cartera',
  'wallet.sidebar.identities': 'VerusID',
  'wallet.sidebar.apps': 'Apps',
  'wallet.sidebar.activity': 'Actividad',
  'wallet.sidebar.addressBook': 'Libreta de direcciones',
  'wallet.sidebar.openRequest': 'Open request',
  'wallet.topbar.settings': 'Ajustes',
  'wallet.topbar.lockWallet': 'Bloquear',
  'genericRequest.import.title': 'Open request',
  'genericRequest.import.heading': 'Paste a GenericRequest',
  'genericRequest.import.description':
    'Open a Verus app request from a full `verus:` link or the raw request hex.',
  'genericRequest.import.label': 'Enlace o código de solicitud',
  'genericRequest.import.placeholder': 'Pega un enlace verus: o un código de solicitud',
  'genericRequest.import.help': 'Pulsa Cmd/Ctrl + Enter para abrir.',
  'genericRequest.import.paste': 'Paste',
  'genericRequest.import.pasteTooltip': 'Pegar desde el portapapeles',
  'genericRequest.import.submit': 'Abrir',
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
  'genericRequest.complete.cta': 'Completar',
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
  'wallet.private.syncingPercent': 'Sincronizando {percent}%',

  'wallet.settings.home.title': 'Ajustes',
  'wallet.settings.home.description':
    'Administra preferencias de la app y herramientas de seguridad.',
  'wallet.settings.home.category.displayLanguage': 'Pantalla e idioma',
  'wallet.settings.home.category.profileSecurity': 'Copias y seguridad',
  'wallet.settings.home.category.privateVerus': 'Privacidad',
  'wallet.settings.home.category.aboutSupport': 'Acerca de y soporte',
  'wallet.settings.home.summary.displayLanguage': '{currency} • {language}',
  'wallet.settings.home.summary.profileSecurity': 'Bloqueo automatico: {minutes} min',
  'wallet.settings.home.summary.privateConfigured': 'Activado',
  'wallet.settings.home.summary.privateNotConfigured': 'No activado',
  'wallet.settings.home.summary.version': 'Version {version}',
  'wallet.settings.display.title': 'Pantalla e idioma',
  'wallet.settings.display.description': 'Elige tu moneda de visualizacion y el idioma de la app.',
  'wallet.settings.display.currency.label': 'Moneda de visualizacion',
  'wallet.settings.display.currency.current': 'Actual: {code}',
  'wallet.settings.display.currency.quickPicks': 'Sugerencias rapidas',
  'wallet.settings.display.currency.otherAction': 'Elegir otra moneda',
  'wallet.settings.display.currency.sheetTitle': 'Moneda de visualizacion',
  'wallet.settings.display.currency.searchPlaceholder': 'Buscar monedas',
  'wallet.settings.display.currency.noResults': 'Ninguna moneda coincide con tu busqueda.',
  'wallet.settings.display.language.label': 'Idioma',
  'wallet.settings.display.language.description': 'Se aplica en toda la app.',

  'wallet.settings.profile.title': 'Copias y seguridad',
  'wallet.settings.profile.description':
    'Administra el bloqueo automatico y el acceso de recuperacion para esta cartera.',
  'wallet.settings.profile.autoLock.title': 'Bloqueo automatico de cartera',
  'wallet.settings.profile.autoLock.description':
    'Bloquea esta cartera tras inactividad. La opcion desactivado no esta disponible.',
  'wallet.settings.profile.autoLock.option': '{minutes} min',
  'wallet.settings.profile.recovery.title': 'Recuperacion y claves',
  'wallet.settings.profile.recovery.description':
    'Muestra secretos de recuperacion y claves derivadas tras confirmar la contrasena.',

  'wallet.transfer.error.dlightBroadcastUncertain':
    'No se pudo confirmar el envío. Revisa el historial de transacciones antes de volver a intentarlo.',
  'wallet.settings.privateVerus.title': 'Privacidad',
  'wallet.settings.privateVerus.description':
    'Configura privacidad shielded para {label}. Puedes reutilizar tu frase secreta de recuperacion principal o definir un secreto de recuperacion de privacidad separado.',
  'wallet.settings.privateVerus.statusConfigured': 'La privacidad esta configurada.',
  'wallet.settings.privateVerus.statusNotConfigured': 'La privacidad aun no esta configurada.',
  'wallet.settings.privateVerus.statusAddress': 'Direccion shielded: {address}',
  'wallet.settings.privateVerus.statusLoadError': 'No se pudo cargar el estado de privacidad.',
  'wallet.settings.privateVerus.reusePrimary': 'Reutilizar frase secreta de recuperacion principal',
  'wallet.settings.privateVerus.createNew': 'Crear nuevo secreto de recuperacion de privacidad',
  'wallet.settings.privateVerus.importLabel':
    'Importar secreto de recuperacion de privacidad o spending key',
  'wallet.settings.privateVerus.importPlaceholder':
    'Pega una frase secreta de recuperacion o secret-extended-key-main…',
  'wallet.settings.privateVerus.importAction': 'Importar secreto de recuperacion de privacidad',
  'wallet.settings.privateVerus.settingUp': 'Configurando privacidad…',
  'wallet.settings.privateVerus.generatedSeedTitle': 'Nueva frase secreta de recuperacion',
  'wallet.settings.privateVerus.setupSuccess': 'Privacidad configurada.',
  'wallet.settings.privateVerus.setupSuccessRelogin':
    'Privacidad configurada. Bloquea y desbloquea tu cartera para activar canales privados.',
  'wallet.settings.privateVerus.setupError': 'No se pudo configurar la privacidad.',
  'wallet.settings.privateVerus.advancedToggleShow': 'Mostrar acciones avanzadas de privacidad',
  'wallet.settings.privateVerus.advancedToggleHide': 'Ocultar acciones avanzadas de privacidad',

  'wallet.settings.about.title': 'Acerca de',
  'wallet.settings.about.description': 'Detalles de version y enlaces de soporte.',
  'wallet.settings.about.appName': 'Nombre de la app',
  'wallet.settings.about.version': 'Version',
  'wallet.settings.about.community': 'Abrir espacio de la comunidad',

  'wallet.settings.recovery.title': 'Recuperacion y claves',
  'wallet.settings.recovery.warningInline':
    'Cualquiera con estos valores puede controlar tus fondos. Guardalos sin conexion y en privado.',
  'wallet.settings.recovery.revealCta': 'Mostrar secretos de recuperacion',
  'wallet.settings.recovery.passwordPlaceholder': 'Contrasena de la cartera',
  'wallet.settings.recovery.passwordInvalid': 'La contrasena es incorrecta.',
  'wallet.settings.recovery.revealLoading': 'Mostrando…',
  'wallet.settings.recovery.revealConfirm': 'Mostrar',
  'wallet.settings.recovery.reveal': 'Mostrar',
  'wallet.settings.recovery.hide': 'Ocultar',
  'wallet.settings.recovery.copy': 'Copiar',
  'wallet.settings.recovery.copySuccess': 'Copiado',
  'wallet.settings.recovery.copyFailed': 'Fallo al copiar',
  'wallet.settings.recovery.valueUnavailable': 'No disponible',
  'wallet.settings.recovery.primaryKindLabel': 'Tipo de secreto principal',
  'wallet.settings.recovery.primarySection': 'Secreto principal',
  'wallet.settings.recovery.derivedKeysSection': 'Claves derivadas',
  'wallet.settings.recovery.addressesSection': 'Direcciones',
  'wallet.settings.recovery.dlightSection': 'Secreto de privacidad',
  'wallet.settings.recovery.dlightKindLabel': 'Tipo de secreto de privacidad',
  'wallet.settings.recovery.field.primarySecret': 'Material secreto principal',
  'wallet.settings.recovery.field.verusWif': 'Verus WIF',
  'wallet.settings.recovery.field.btcWif': 'Bitcoin WIF',
  'wallet.settings.recovery.field.ethPrivateKey': 'Clave privada de Ethereum',
  'wallet.settings.recovery.field.verusAddress': 'Direccion Verus',
  'wallet.settings.recovery.field.btcAddress': 'Direccion Bitcoin',
  'wallet.settings.recovery.field.ethAddress': 'Direccion Ethereum',
  'wallet.settings.recovery.field.dlightSecret':
    'Secreto de recuperacion de privacidad o spending key',
  'wallet.settings.recovery.field.dlightShieldedAddress': 'Direccion shielded de privacidad',
  'wallet.settings.recovery.field.dlightDerivedSpendingKey': 'Spending key privada derivada',
  'wallet.settings.recovery.kind.seedText': 'Semilla personalizada',
  'wallet.settings.recovery.kind.wif': 'WIF',
  'wallet.settings.recovery.kind.privateKeyHex': 'Clave privada (hex)',
  'wallet.settings.recovery.kind.dlightMnemonic': 'Frase secreta de recuperacion',
  'wallet.settings.recovery.kind.dlightSpendingKey': 'Spending key',
  'wallet.settings.recovery.kind.unknown': 'Desconocido',
  'wallet.session.expired': 'Sesion expirada. Cartera bloqueada.',

  'wallet.overview.mainWallet': 'Cartera principal',
  'wallet.overview.errorActiveAssetsFallback':
    'No se pudieron cargar los activos activos. Se muestra tu vista anterior de activos.',
  'wallet.overview.send': 'Enviar',
  'wallet.overview.receive': 'Recibir',
  'wallet.overview.convert': 'Convertir',
  'wallet.overview.noChannel': 'Aun no hay un canal activo disponible.',
  'wallet.overview.hideHoldings': 'Ocultar balances',
  'wallet.overview.showHoldings': 'Mostrar balances',
  'wallet.overview.scrollHintMoreAssets': 'Desplazate para ver mas activos',
  'wallet.overview.partialRatesNotice': 'Algunas cotizaciones de activos no estan disponibles.',
  'wallet.overview.partialBalancesNotice':
    'Algunos balances aun se estan cargando o no estan disponibles.',
  'wallet.overview.partialTotalLabel': 'Parcial',
  'wallet.overview.partialTotalDescription':
    'Algunos saldos o tipos de cambio no estan disponibles, por lo que este total esta incompleto.',
  'wallet.assetDetails.noTransactionsInRecentRange':
    'No hay transacciones en el historial reciente.',
  'wallet.assetDetails.loadOlderTransactions': 'Consultar historial anterior',
  'wallet.assetDetails.errorLoadScopes': 'No se pudieron cargar los alcances de subcartera.',
  'wallet.assetDetails.scopeUnavailable': 'No hay alcance disponible para este activo.',
  'wallet.assetDetails.scopePicker': 'Cambiar direccion y red',
  'wallet.assetDetails.readOnlyHelper':
    'Enviar y convertir solo estan disponibles desde tu direccion principal.',
  'wallet.assetDetails.privateSyncInlineHelper': 'Sincronizando balance',
  'wallet.assetDetails.sendCapabilityInline': 'Sync de envio {percent}%',
};
