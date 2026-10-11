import { en } from './en';

export const es: Record<string, string> = {
  ...en,
  'helpCenter.unlockRequired': 'Desbloquea tu cartera para abrir esta pantalla.',
  'common.loading': 'Cargando…',
  'common.cancel': 'Cancelar',
  'common.continue': 'Continuar',
  'common.back': 'Atras',
  'common.close': 'Cerrar',
  'common.clearSearch': 'Borrar búsqueda',
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
  'unlock.importMethods.back': 'Volver a las opciones de creación',
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
  'genericRequest.error.generic': 'This request could not be opened right now.',
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
  'genericRequest.error.complete': 'Could not finish this request.',
  'genericRequest.summary.toggle': 'Show summary',
  'genericRequest.summary.provisioningRequested':
    'A new VerusID can be provisioned from this request.',
  'genericRequest.auth.title': 'Sign-in request',
  'genericRequest.auth.description': 'Choose the VerusID you want to use for this request.',
  'genericRequest.auth.requiredId': 'Required VerusID',
  'genericRequest.auth.requiredSystem': 'Required system',
  'genericRequest.auth.requiredParent': 'Required parent',
  'genericRequest.auth.identities': 'Eligible VerusIDs',
  'genericRequest.auth.loading': 'Loading linked VerusIDs…',
  'genericRequest.auth.selectorDescription':
    'Choose the linked VerusID you want to use for this sign-in request.',
  'genericRequest.auth.requesterFallback': 'Requesting app',
  'genericRequest.auth.linkIdentity': 'Link VerusID',
  'genericRequest.update.preflightLoading': 'Preparing the identity update…',
  'genericRequest.update.highRiskTitle': 'Confirm high-risk changes',
  'genericRequest.update.highRiskDescription':
    'These changes affect ownership, recovery, revocation, status, or other high-risk identity fields.',
  'genericRequest.update.highRiskDescriptionContentClear':
    'After this update, apps will no longer see this data on your VerusID, but it will still be visible on the blockchain.',
  'genericRequest.update.highRiskAcknowledge':
    'I understand that these high-risk identity changes may be difficult or impossible to undo.',
  'genericRequest.update.beforeValue': 'Before',
  'genericRequest.update.afterValue': 'After',
  'genericRequest.update.currentValue': 'Current value',
  'genericRequest.update.emptyValue': 'Empty',
  'genericRequest.update.privateAddress': 'Private address',
  'genericRequest.update.contentPrefix': 'Content',
  'genericRequest.update.content.currentIdentityContent': 'Current identity content',
  'genericRequest.update.content.credentials': 'Credenciales (guardadas cifradas)',
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
  'genericRequest.update.fundingSource': 'Wallet to pay from',
  'genericRequest.update.fundingSourcePlaceholder': 'Choose a funding source',
  'genericRequest.update.fundingSourcesLoading': 'Loading wallet balances…',
  'genericRequest.update.fundingCalculating': 'Calculating fee…',
  'genericRequest.update.fundingEmpty':
    'No eligible wallets on this network are available to pay from.',
  'genericRequest.complete.updateBroadcast': 'Identity update broadcast: {txid}',
  'genericRequest.complete.txidLabel': 'Transaction id',
  'genericRequest.provisioning.title': 'Provision requested VerusID',
  'genericRequest.provisioning.description':
    'Create the requested VerusID, then finish this request after it appears in your wallet.',
  'genericRequest.provisioning.ready': 'Provisioning can start from this request.',
  'genericRequest.provisioning.webhookMissing':
    'This request did not include a provisioning webhook.',
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
  'wallet.settings.home.category.profileSecurity': 'Perfil y seguridad',
  'wallet.settings.home.category.privateVerus': 'Private Verus',
  'wallet.settings.home.category.aboutSupport': 'Acerca de y soporte',
  'wallet.settings.home.summary.displayLanguage': '{appearance} · {currency} · {language}',
  'wallet.settings.home.summary.profileSecurity': 'Recuperación y claves',
  'wallet.settings.home.summary.privateConfigured': 'Configurado',
  'wallet.settings.home.summary.privateNotConfigured': 'Sin configurar',
  'wallet.settings.home.summary.version': 'Versión {version}',
  'wallet.settings.backLabel': 'Volver a ajustes',
  'wallet.settings.recovery.backToProfileSecurity': 'Volver a perfil y seguridad',
  'wallet.settings.recovery.backToPrivateVerus': 'Volver a Private Verus',
  'wallet.settings.display.title': 'Pantalla e idioma',
  'wallet.settings.display.description': 'Elige tu moneda de visualizacion y el idioma de la app.',
  'wallet.settings.display.currency.label': 'Moneda de visualización',
  'wallet.settings.display.currency.current': 'Actual: {code}',
  'wallet.settings.display.currency.quickPicks': 'Sugerencias rapidas',
  'wallet.settings.display.currency.otherAction': 'Elegir otra moneda',
  'wallet.settings.display.currency.sheetTitle': 'Moneda de visualización',
  'wallet.settings.display.currency.searchPlaceholder': 'Buscar monedas',
  'wallet.settings.display.currency.allCurrencies': 'Todas las monedas',
  'wallet.settings.display.currency.resultCount.one': '{count} moneda',
  'wallet.settings.display.currency.resultCount.other': '{count} monedas',
  'wallet.settings.display.currency.noResults': 'Ninguna moneda coincide con tu búsqueda.',
  'wallet.settings.display.language.label': 'Idioma de visualización',
  'wallet.settings.display.language.description': 'Se aplica en toda la app.',
  'wallet.settings.display.appearance.label': 'Apariencia',
  'wallet.settings.display.appearance.light': 'Claro',
  'wallet.settings.display.appearance.dark': 'Oscuro',
  'wallet.settings.display.appearance.system': 'Sistema',
  'wallet.settings.display.appearance.systemDescription':
    'Usa la apariencia clara u oscura de tu dispositivo.',

  'wallet.settings.profile.title': 'Perfil y seguridad',
  'wallet.settings.profile.description':
    'Administra el bloqueo automatico y el acceso de recuperacion para esta cartera.',
  'wallet.settings.profile.autoLock.title': 'Bloquear tras inactividad',
  'wallet.settings.profile.autoLock.description':
    'Bloquea esta cartera tras inactividad. La opcion desactivado no esta disponible.',
  'wallet.settings.profile.autoLock.option': '{minutes} min',
  'wallet.settings.profile.recovery.title': 'Recuperación y claves',
  'wallet.settings.profile.recovery.description':
    'Muestra secretos de recuperacion y claves derivadas tras confirmar la contrasena.',

  'wallet.transfer.error.dlightBroadcastUncertain':
    'No se pudo confirmar el envío. Revisa el historial de transacciones antes de volver a intentarlo.',
  'wallet.transfer.error.ethBroadcastUncertain':
    'No se pudo confirmar el envío de Ethereum. Continúa la transferencia guardada en vez de volver a enviar.',
  'wallet.transfer.error.ethBroadcastRecovered':
    'Se recuperó la transferencia de Ethereum guardada.',
  'wallet.transfer.error.ethRecoveryRequired':
    'Termina la transferencia de Ethereum guardada antes de iniciar otra.',
  'wallet.transfer.ethRecovery.title': 'Recuperar transferencia de Ethereum',
  'wallet.transfer.ethRecovery.description':
    'Revisa los detalles de la transacción antes de continuar.',
  'wallet.transfer.ethRecovery.continue': 'Continuar transferencia',
  'wallet.transfer.ethRecovery.showResult': 'Mostrar transferencia enviada',
  'wallet.transfer.ethRecovery.recovering': 'Recuperando…',
  'wallet.transfer.ethRecovery.error.stageAdvanced':
    'La transferencia avanzó, pero no se pudo enviar el siguiente paso. Revisa los datos actualizados y vuelve a continuar.',
  'wallet.transfer.ethRecovery.error.retry':
    'No se pudo continuar la transferencia. Revisa estos datos e inténtalo de nuevo.',
  'wallet.transfer.ethRecovery.error.noPending':
    'La recuperación ya no está pendiente. Revisa los datos de la transferencia antes de intentarlo de nuevo.',
  'wallet.transfer.ethRecovery.error.refreshFailed':
    'No se pudo actualizar el estado de recuperación. Revisa los últimos datos conocidos e inténtalo de nuevo.',
  'wallet.transfer.ethRecovery.network': 'Red',
  'wallet.transfer.ethRecovery.chain': 'Cadena {value}',
  'wallet.transfer.ethRecovery.stage': 'Fase de recuperación',
  'wallet.transfer.ethRecovery.assetContract': 'Contrato del activo',
  'wallet.transfer.ethRecovery.bridgeContract': 'Contrato del puente',
  'wallet.transfer.ethRecovery.mappedCurrency': 'Moneda vinculada',
  'wallet.transfer.ethRecovery.destinationSystem': 'Sistema de destino',
  'wallet.transfer.ethRecovery.stage.eth': 'Transferencia nativa',
  'wallet.transfer.ethRecovery.stage.erc20': 'Transferencia de token',
  'wallet.transfer.ethRecovery.stage.bridge_zero_approval': 'Restablecer aprobación del token',
  'wallet.transfer.ethRecovery.stage.bridge_approval': 'Aprobar acceso al puente',
  'wallet.transfer.ethRecovery.stage.bridge_transfer': 'Enviar transferencia del puente',
  'wallet.transfer.ethRecovery.stage.unknown': 'Transferencia guardada',
  'wallet.settings.privateVerus.title': 'Privacidad',
  'wallet.settings.privateVerus.description':
    'Configura privacidad shielded para {label}. Puedes reutilizar tu frase secreta de recuperacion principal o definir un secreto de recuperacion de privacidad separado.',
  'wallet.settings.privateVerus.statusConfigured': 'La privacidad esta configurada.',
  'wallet.settings.privateVerus.statusNotConfigured': 'La privacidad aun no esta configurada.',
  'wallet.settings.privateVerus.statusAddress': 'Direccion shielded',
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
  'wallet.settings.recovery.copyFailed': 'No se pudo copiar este valor. Intentalo de nuevo.',
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
  'wallet.assetDetails.back': 'Volver a la cartera',
  'wallet.assetDetails.loadOlderTransactions': 'Consultar historial anterior',
  'wallet.assetDetails.errorLoadScopes': 'No se pudieron cargar los alcances de subcartera.',
  'wallet.assetDetails.scopeUnavailable': 'No hay alcance disponible para este activo.',
  'wallet.assetDetails.scopePicker': 'Cambiar direccion y red',
  'wallet.assetDetails.readOnlyHelper':
    'Enviar y convertir solo estan disponibles desde tu direccion principal.',
  'wallet.assetDetails.privateSyncInlineHelper': 'Sincronizando balance',
  'wallet.assetDetails.sendCapabilityInline': 'Sync de envio {percent}%',
  'wallet.manageAssets.back': 'Volver a gestionar activos',
  'wallet.manageAssets.foundCount.one': '{count} activo encontrado en tu cartera',
  'wallet.manageAssets.foundCount.other': '{count} activos encontrados en tu cartera',
  'wallet.manageAssets.foundOutsidePortfolioCount.one':
    '{count} activo encontrado fuera de tu portafolio',
  'wallet.manageAssets.foundOutsidePortfolioCount.other':
    '{count} activos encontrados fuera de tu portafolio',
  'help.backToTopics': 'Volver a los temas de ayuda',
  'genericRequest.flow.close': 'Cerrar solicitud',
  'genericRequest.flow.requestFrom': 'Solicitud de {app}',
  'genericRequest.flow.signedBy': 'Firmada por',
  'genericRequest.flow.notSigned': 'Sin firmar',
  'genericRequest.flow.expired': 'Caducada',
  'genericRequest.flow.timeLeft.minutes': 'Quedan {count} min',
  'genericRequest.flow.timeLeft.hours': 'Quedan unas {count} horas',
  'genericRequest.flow.responds': 'responde',
  'genericRequest.flow.stepsLabel': 'Pasos de la solicitud',
  'genericRequest.flow.respondAs': 'Responder como',
  'genericRequest.flow.chooseIdentity': 'Elige una VerusID',
  'genericRequest.flow.noMatchingIdentity': 'Ninguna VerusID coincide',
  'genericRequest.flow.noMatchingIdentityHint':
    'Vincula una VerusID que cumpla las reglas de esta solicitud.',
  'genericRequest.flow.payFeeFrom': 'Pagar la comisión desde',
  'genericRequest.flow.chooseSource': 'Elige un origen',
  'genericRequest.flow.feeAfterChoice': 'La comisión aparece cuando eliges',
  'genericRequest.flow.networkFee': 'Comisión de red',
  'genericRequest.flow.feeValue': '{fee} {currency}',
  'genericRequest.flow.highRisk': 'Riesgo alto',
  'genericRequest.flow.impact.signs': 'Firma como tú',
  'genericRequest.flow.impact.shares': 'Comparte datos',
  'genericRequest.flow.impact.spends': 'Gasta fondos',
  'genericRequest.flow.impact.control': 'Cambia el control',
  'genericRequest.flow.step.signIn': 'Iniciar sesión',
  'genericRequest.flow.step.newIdentity': 'Nueva VerusID',
  'genericRequest.flow.step.updateChanges': 'Actualizar identidad',
  'genericRequest.flow.step.networkFee': 'Comisión de red',
  'genericRequest.flow.step.confirm': 'Confirmar',
  'genericRequest.flow.step.done': 'Listo',
  'genericRequest.flow.title.signIn': 'Iniciar sesión en {app}',
  'genericRequest.flow.title.newIdentity': 'Obtener una nueva VerusID',
  'genericRequest.flow.title.updateChanges': 'Revisa los cambios en {identity}',
  'genericRequest.flow.title.networkFee': 'Elige quién paga la comisión',
  'genericRequest.flow.title.confirm': 'Confirmar y enviar',
  'genericRequest.flow.title.confirmNoDelivery': 'Confirmar',
  'genericRequest.flow.signIn.site': 'Sitio',
  'genericRequest.flow.signIn.learns': 'El sitio recibe',
  'genericRequest.flow.signIn.learnsValue': 'El nombre y el ID de tu VerusID',
  'genericRequest.flow.signIn.requirements': 'Requisitos',
  'genericRequest.flow.signIn.requirementsNone': 'Ninguno',
  'genericRequest.flow.newIdentity.offer': 'Nueva VerusID',
  'genericRequest.flow.newIdentity.chooseName': 'Nombre de tu nueva VerusID',
  'genericRequest.flow.newIdentity.nameHint': 'Elige un nombre que aún esté libre.',
  'genericRequest.flow.newIdentity.nameInvalid':
    'Un nombre no puede tener \\ / : * ? " < > | @ ni punto, ni empezar o terminar con un espacio.',
  'genericRequest.flow.newIdentity.nameChecking': 'Comprobando el nombre…',
  'genericRequest.flow.newIdentity.nameTaken': 'Este nombre está ocupado. Elige otro.',
  'genericRequest.flow.newIdentity.nameCheckFailed':
    'No se pudo comprobar el nombre. Vuelve a intentarlo.',
  'genericRequest.flow.newIdentity.nameAvailable': 'Este nombre está libre.',
  'genericRequest.flow.newIdentity.chosen': 'Una nueva VerusID, {name}, responde a esta solicitud.',
  'genericRequest.flow.newIdentity.name': 'Nueva VerusID',
  'genericRequest.flow.newIdentity.issuer': 'Emitida por',
  'genericRequest.flow.newIdentity.network': 'Red',
  'genericRequest.flow.newIdentity.note':
    '{issuer} la registra en la cadena. Tarda unos minutos. Después la vinculas y el inicio de sesión termina.',
  'genericRequest.flow.plan.signIn': 'Iniciar sesión en {app} como {identity}',
  'genericRequest.flow.plan.newIdentity': 'Pedir {name} a {issuer}',
  'genericRequest.flow.plan.update': 'Actualizar {identity}',
  'genericRequest.flow.plan.changes.one': '{count} cambio',
  'genericRequest.flow.plan.changes.other': '{count} cambios',
  'genericRequest.flow.plan.withHighRisk': '{changes}, {count} de riesgo alto',
  'genericRequest.flow.delivery.post': 'La respuesta va a {app}',
  'genericRequest.flow.delivery.redirect': 'Abre {app} en tu navegador',
  'genericRequest.flow.delivery.none': 'No se envía ninguna respuesta',
  'genericRequest.flow.delivery.webhook': 'La solicitud va a {issuer}',
  'genericRequest.flow.action.signIn': 'Iniciar sesión',
  'genericRequest.flow.action.signInAndUpdate': 'Iniciar sesión y actualizar',
  'genericRequest.flow.action.update': 'Actualizar identidad',
  'genericRequest.flow.action.newIdentity': 'Solicitar {name}',
  'genericRequest.flow.working.signIn': 'Iniciando sesión…',
  'genericRequest.flow.working.update': 'Enviando la actualización…',
  'genericRequest.flow.working.newIdentity': 'Enviando la solicitud…',
  'genericRequest.flow.decline': 'Rechazar',
  'genericRequest.flow.stage.transaction.pending': 'Enviar la actualización',
  'genericRequest.flow.stage.transaction.running': 'Enviando la actualización…',
  'genericRequest.flow.stage.transaction.done': 'Actualización enviada',
  'genericRequest.flow.stage.transaction.failed': 'Actualización no enviada',
  'genericRequest.flow.stage.signing.pending': 'Firmar la respuesta',
  'genericRequest.flow.stage.signing.running': 'Firmando la respuesta…',
  'genericRequest.flow.stage.signing.done': 'Respuesta firmada',
  'genericRequest.flow.stage.signing.failed': 'Respuesta sin firmar',
  'genericRequest.flow.stage.delivery.pending': 'Enviar a {app}',
  'genericRequest.flow.stage.delivery.running': 'Enviando a {app}…',
  'genericRequest.flow.stage.delivery.done': 'Entregada a {app}',
  'genericRequest.flow.stage.delivery.failed': 'Firmada, sin entregar',
  'genericRequest.flow.stage.redirect.pending': 'Abrir {app}',
  'genericRequest.flow.stage.redirect.running': 'Abriendo {app}…',
  'genericRequest.flow.stage.redirect.done': '{app} abierta',
  'genericRequest.flow.stage.redirect.failed': 'Firmada, {app} sin abrir',
  'genericRequest.flow.stage.webhook.pending': 'Enviar la solicitud a {issuer}',
  'genericRequest.flow.stage.webhook.running': 'Enviando la solicitud a {issuer}…',
  'genericRequest.flow.stage.webhook.done': 'Solicitud enviada a {issuer}',
  'genericRequest.flow.stage.webhook.failed': 'Solicitud no enviada',
  'genericRequest.flow.outcome.failed': 'Solicitud sin terminar',
  'genericRequest.flow.outcome.signedIn': 'Has iniciado sesión en {app}',
  'genericRequest.flow.outcome.updated': '{identity} está actualizada',
  'genericRequest.flow.outcome.newIdentity': '{name} está en camino',
  'genericRequest.flow.outcome.newIdentityAfter':
    'Vincula {name} cuando esté lista. Entonces termina el inicio de sesión.',
  'genericRequest.flow.outcome.declined': 'Solicitud rechazada',
  'genericRequest.flow.outcome.declinedBody':
    'No se firmó ni se envió nada. {app} no recibió aviso.',
  'genericRequest.flow.outcome.expired': 'Esta solicitud caducó',
  'genericRequest.flow.outcome.expiredBody': 'Empieza de nuevo en el sitio.',
  'genericRequest.flow.receipt.signedInAs': 'Sesión iniciada como',
  'genericRequest.flow.copyResponse': 'Copiar la respuesta',
  'genericRequest.flow.copyTransaction': 'Copiar el ID de transacción',
  'genericRequest.flow.openSite': 'Abrir el sitio',
  'genericRequest.flow.stageError.transaction': 'No se pudo enviar la actualización.',
  'genericRequest.flow.stageError.signing': 'No se pudo firmar la respuesta.',
  'genericRequest.flow.stageError.delivery': '{app} no aceptó la respuesta.',
  'genericRequest.flow.stageError.webhook': '{issuer} no aceptó la solicitud.',
  'genericRequest.flow.noChanges':
    'Esta actualización no cambia nada que la cartera pueda mostrar.',
  'genericRequest.flow.secondLinkIgnored':
    'Termina la solicitud actual y luego abre el enlace nuevo otra vez.',
  'genericRequest.flow.step.payInvoice': 'Pagar factura',
  'genericRequest.flow.title.payInvoice': 'Pagar a {app}',
  'genericRequest.flow.plan.pay': 'Pagar {amount} a {to}',
  'genericRequest.flow.action.pay': 'Pagar {amount}',
  'genericRequest.flow.action.signInAndPay': 'Iniciar sesión y pagar',
  'genericRequest.flow.working.pay': 'Enviando pago…',
  'genericRequest.flow.stage.payment.pending': 'Enviar pago',
  'genericRequest.flow.stage.payment.running': 'Enviando pago…',
  'genericRequest.flow.stage.payment.done': 'Pago enviado',
  'genericRequest.flow.stage.payment.failed': 'Pago no enviado',
  'genericRequest.flow.outcome.paid': 'Pagaste {amount} a {to}',
  'genericRequest.flow.invoice.amount': 'Importe',
  'genericRequest.flow.invoice.to': 'Para',
  'genericRequest.flow.invoice.amountInput': 'Importe en {currency}',
  'genericRequest.flow.invoice.payFrom': 'Pagar desde',
  'genericRequest.flow.invoice.noSource': 'Ninguna dirección de esta cartera tiene {currency}.',
  'genericRequest.flow.invoice.total': 'Total',
  'genericRequest.flow.invoice.totalSplit': '{amount} y {fee}',
  'genericRequest.flow.invoice.paidTo': 'Pagado a',
  'genericRequest.flow.invoice.loading': 'Leyendo la factura…',
  'genericRequest.flow.invoice.error.review': 'No se pudo leer esta factura.',
  'genericRequest.flow.invoice.error.funds':
    'No hay suficiente {currency} para pagar esta factura y su comisión. Un pago aún sin confirmar puede retener parte de tu saldo durante un minuto aproximadamente.',
  'genericRequest.flow.invoice.error.preflight': 'No se pudo preparar este pago.',
  'genericRequest.rejected.invoiceTagged':
    'Esta factura pide un pago con etiqueta, que esta cartera no envía.',
  'genericRequest.rejected.invoiceUnpayable': 'Esta factura no se puede pagar en la cadena Verus.',
  'genericRequest.rejected.invoicePrivate':
    'Esta factura pide un pago privado, que esta cartera no envía.',
  'genericRequest.rejected.invoiceNoDestination':
    'Esta factura no indica ninguna dirección de pago.',
  'genericRequest.rejected.invoiceCurrency':
    'La moneda de esta factura es desconocida en esta red.',
  'genericRequest.rejected.invoiceCurrencyMissing':
    'Añade la moneda de esta factura a tu cartera para pagarla.',
  'genericRequest.rejected.invoicePreconvertLaunched':
    'Esta factura preconvierte a una moneda que ya se lanzó.',
  'genericRequest.rejected.alreadyPaid': 'Ya pagaste esta factura.',
  'genericRequest.flow.step.signData': 'Firmar datos',
  'genericRequest.flow.title.signData': 'Firmar para {app}',
  'genericRequest.flow.signData.intro':
    '{app} te pide firmar cada elemento de abajo. Cada firma muestra que estás de acuerdo con ese elemento.',
  'genericRequest.flow.signData.message': 'Mensaje',
  'genericRequest.flow.signData.data': 'Datos',
  'genericRequest.flow.signData.statement': 'Declaración',
  'genericRequest.flow.signData.size': '{size} bytes',
  'genericRequest.flow.signData.bytes': 'Bytes',
  'genericRequest.flow.signData.hash': 'SHA-256 que firmas',
  'genericRequest.flow.signData.consent': 'Firmar {item}',
  'genericRequest.flow.signData.allNeeded': 'Marca cada elemento para continuar.',
  'genericRequest.flow.plan.signData.one': 'Firmar {count} elemento para {app} como {identity}',
  'genericRequest.flow.plan.signData.other': 'Firmar {count} elementos para {app} como {identity}',
  'genericRequest.flow.action.sign': 'Firmar',
  'genericRequest.flow.action.signInAndSign': 'Iniciar sesión y firmar',
  'genericRequest.flow.working.sign': 'Firmando…',
  'genericRequest.flow.outcome.signedData.one': 'Firmaste {count} elemento para {app}',
  'genericRequest.flow.outcome.signedData.other': 'Firmaste {count} elementos para {app}',
  'genericRequest.flow.receipt.signedAs': 'Firmado como',
  'genericRequest.rejected.dataTransmittal':
    'Esta solicitud te envía datos en lugar de pedir una firma. Esta cartera no lo acepta.',
  'genericRequest.rejected.dataNotForSignature': 'Esta solicitud no te pide firmar nada.',
  'genericRequest.flow.step.encryptionKey': 'Clave de cifrado',
  'genericRequest.flow.title.encryptionKey': 'Compartir una clave de cifrado con {app}',
  'genericRequest.flow.encryptionKey.app': 'App',
  'genericRequest.flow.encryptionKey.number': 'Número de clave',
  'genericRequest.flow.encryptionKey.shares': 'Comparte',
  'genericRequest.flow.encryptionKey.sharesViewing':
    'Una clave de visualización y una dirección privada para esta app',
  'genericRequest.flow.encryptionKey.sharesWithSpendingKey':
    'Una clave de visualización, una dirección privada y su clave de gasto',
  'genericRequest.flow.encryptionKey.checking': 'Comprobando las direcciones privadas…',
  'genericRequest.flow.encryptionKey.setupTitle': 'Configura primero las direcciones privadas',
  'genericRequest.flow.encryptionKey.setupBody':
    'Esta app necesita una dirección privada de esta cartera. Configura las direcciones privadas y luego abre otra vez el enlace de la solicitud.',
  'genericRequest.flow.encryptionKey.setupAction': 'Configurar direcciones privadas',
  'genericRequest.flow.encryptionKey.spendingKeyTitle': 'Esta app también pide una clave de gasto',
  'genericRequest.flow.encryptionKey.spendingKeyBody':
    'Con ella, la app puede gastar todo lo que llegue a esta dirección privada. Compártela solo con una app a la que confíes esos fondos.',
  'genericRequest.flow.encryptionKey.spendingKeyAcknowledge':
    'Entiendo que esta app puede gastar desde esta dirección.',
  'genericRequest.flow.encryptionKey.error.status':
    'No se pudieron comprobar las direcciones privadas.',
  'genericRequest.flow.plan.encryptionKey':
    'Compartir una clave de cifrado con {app} como {identity}',
  'genericRequest.flow.action.shareKey': 'Compartir clave',
  'genericRequest.flow.working.shareKey': 'Compartiendo la clave…',
  'genericRequest.flow.outcome.keyShared': '{app} tiene una clave de cifrado',
  'genericRequest.flow.receipt.sharedAs': 'Compartida como',
  'genericRequest.rejected.encryptionKeyAddress':
    'Esta solicitud indica una dirección de cifrado no válida, o pide una clave de gasto sin ninguna.',
  'genericRequest.rejected.encryptionKeyIdentity':
    'La identidad para la que es esta clave no existe.',
  'genericRequest.rejected.privateNotSetUp': 'Configura primero las direcciones privadas.',
  'genericRequest.flow.step.shareData': 'Compartir datos',
  'genericRequest.flow.title.shareData': 'Compartir credenciales con {app}',
  'genericRequest.flow.shareData.intro':
    '{app} pide estas credenciales de {identity}. Se envían cifradas, así que solo {app} puede leerlas.',
  'genericRequest.flow.shareData.loading': 'Buscando credenciales que coincidan…',
  'genericRequest.flow.shareData.consent': 'Compartir {credential}',
  'genericRequest.flow.shareData.allNeeded': 'Marca cada credencial para continuar.',
  'genericRequest.flow.shareData.missing.one':
    'No se encontró {count} credencial solicitada. No se envía.',
  'genericRequest.flow.shareData.missing.other':
    'No se encontraron {count} credenciales solicitadas. No se envían.',
  'genericRequest.flow.shareData.none':
    'No se encontraron credenciales que coincidan en {identity}.',
  'genericRequest.flow.shareData.nothingToSend':
    'No hay nada que compartir. Rechaza para cerrar esta solicitud.',
  'genericRequest.flow.shareData.error.lookup': 'No se pudieron leer las credenciales.',
  'genericRequest.flow.plan.shareData.one':
    'Compartir {count} credencial con {app} como {identity}',
  'genericRequest.flow.plan.shareData.other':
    'Compartir {count} credenciales con {app} como {identity}',
  'genericRequest.flow.action.share': 'Compartir',
  'genericRequest.flow.action.signInAndShare': 'Iniciar sesión y compartir',
  'genericRequest.flow.working.share': 'Compartiendo…',
  'genericRequest.flow.outcome.sharedData.one': 'Compartiste {count} credencial con {app}',
  'genericRequest.flow.outcome.sharedData.other': 'Compartiste {count} credenciales con {app}',
  'genericRequest.rejected.credentialsOnly':
    'Esta solicitud pide datos distintos de credenciales completas. Esta cartera solo comparte credenciales completas.',
  'genericRequest.rejected.credentialsNotEncrypted':
    'Esta solicitud pide credenciales, pero no indica una dirección para cifrarlas, así que no se envían.',
  'genericRequest.rejected.credentialAddress':
    'La dirección privada de esta VerusID no proviene de esta cartera, así que no se pueden guardar credenciales en ella.',
  'genericRequest.rejected.loginContext':
    'Esta solicitud de inicio de sesión pide compartir datos de sesión adicionales, algo que esta cartera no admite.',
  'genericRequest.rejected.loginPermission':
    'Esta solicitud de inicio de sesión pide más que el nombre y el ID de tu VerusID. Esta cartera solo comparte esos datos.',
  'genericRequest.flow.step.giftCard': 'Tarjeta regalo',
  'genericRequest.flow.title.giftCard': 'Canjear una tarjeta regalo',
  'genericRequest.flow.giftCard.passwordIntro':
    'Esta tarjeta regalo tiene contraseña. Escríbela para ver lo que contiene.',
  'genericRequest.flow.giftCard.password': 'Contraseña de la tarjeta regalo',
  'genericRequest.flow.giftCard.unlock': 'Abrir tarjeta regalo',
  'genericRequest.flow.giftCard.loading': 'Leyendo la tarjeta regalo…',
  'genericRequest.flow.giftCard.empty': 'Esta tarjeta regalo está vacía. No hay nada que canjear.',
  'genericRequest.flow.giftCard.intro': 'Todo lo de abajo pasa a esta cartera.',
  'genericRequest.flow.giftCard.holds': 'Esta tarjeta regalo contiene:',
  'genericRequest.flow.giftCard.cannotMove': 'No puede pasar a esta cartera',
  'genericRequest.flow.giftCard.moves': 'Su control pasa a esta cartera',
  'genericRequest.flow.giftCard.blocked':
    '{identity} necesita más de una firma o no está activa, así que esta tarjeta regalo no se puede canjear aquí.',
  'genericRequest.flow.giftCard.authorities':
    '{identity} conserva sus autoridades de revocación y recuperación. Quien las controle aún puede revocarla o recuperarla.',
  'genericRequest.flow.giftCard.preparing': 'Preparando el canje…',
  'genericRequest.flow.giftCard.claimed': 'Canjeado',
  'genericRequest.flow.giftCard.next': 'Siguiente paso',
  'genericRequest.flow.giftCard.linkAfter':
    'Vincula la VerusID desde VerusID cuando se confirme el cambio.',
  'genericRequest.flow.giftCard.error.review': 'No se pudo leer la tarjeta regalo.',
  'genericRequest.flow.giftCard.error.preflight': 'No se pudo preparar el canje.',
  'genericRequest.flow.giftCard.error.fee':
    'La tarjeta regalo no tiene suficiente para pagar la comisión de red.',
  'genericRequest.flow.plan.giftCard': 'Canjear {items} en esta cartera',
  'genericRequest.flow.action.claim': 'Canjear',
  'genericRequest.flow.working.claim': 'Canjeando…',
  'genericRequest.flow.outcome.claimed': 'Tarjeta regalo canjeada',
  'genericRequest.flow.stage.claim.pending': 'Enviar canje',
  'genericRequest.flow.stage.claim.running': 'Enviando canje…',
  'genericRequest.flow.stage.claim.done': 'Canje enviado',
  'genericRequest.flow.stage.claim.failed': 'Canje sin terminar',
  'genericRequest.flow.stageError.claim':
    'Parte del canje no se envió. Vuelve a intentarlo para enviar el resto.',
  'genericRequest.rejected.giftCardInvalid':
    'Esta tarjeta regalo tiene un formato que esta cartera no puede leer.',
  'genericRequest.rejected.giftCardPassword':
    'La contraseña no es correcta para esta tarjeta regalo.',
  'genericRequest.rejected.giftCardEmpty':
    'Esta tarjeta regalo está vacía. No hay nada que canjear.',
  'genericRequest.rejected.giftCardIdentity':
    'Una VerusID de esta tarjeta regalo no se puede mover aquí, así que la tarjeta no se puede canjear.',
  'genericRequest.rejected.unknownDetail':
    'Esta solicitud contiene una parte que esta cartera no reconoce.',
  'genericRequest.rejected.detailOrder':
    'Esta solicitud no está bien construida. Pide al sitio web un enlace nuevo.',
  'genericRequest.rejected.unsigned':
    'Esta solicitud no está firmada. Solo los pagos y las tarjetas regalo pueden ir sin firma.',
  'genericRequest.rejected.walletBackup':
    'Las copias de seguridad de la cartera en tarjetas NFC necesitan Verus Mobile.',
  'genericRequest.rejected.network':
    'Esta solicitud es para una red distinta a la de la cartera abierta.',
  'genericRequest.rejected.encryptedDetails':
    'Esta solicitud está cifrada, así que esta cartera no puede leerla.',
  'genericRequest.rejected.tooLong':
    'Este enlace es demasiado largo. Pide al sitio web uno más corto.',
  'genericRequest.rejected.expired': 'Esta solicitud ha caducado. Pide al sitio web una nueva.',
  'genericRequest.rejected.delegatedSigner':
    'Esta solicitud la firmó para otra app una VerusID que esta cartera no controla.',
  'genericRequest.rejected.signature': 'No se pudo verificar la firma de la solicitud.',
  'genericRequest.rejected.notOpen': 'Esta solicitud ya no está abierta. Abre el enlace de nuevo.',
  'genericRequest.rejected.responder':
    'La VerusID elegida no cumple los requisitos de esta solicitud.',
  'genericRequest.rejected.unsupportedDetail':
    'Una parte de esta solicitud aún no funciona en escritorio.',
  'genericRequest.rejected.responseAddress':
    'La respuesta solo puede ir a una dirección que indique la solicitud.',
  'genericRequest.rejected.insecureDelivery':
    'El sitio web no ofreció una conexión segura, así que la respuesta no se envió.',
  'genericRequest.flow.error.expired': 'Esta solicitud caducó. Empieza de nuevo en el sitio.',
  'genericRequest.update.privateAddressChange.title': 'Cambiar la dirección privada',
  'genericRequest.update.privateAddressChange.description':
    'Los pagos privados a esta VerusID pueden ir a esta dirección.',
};
