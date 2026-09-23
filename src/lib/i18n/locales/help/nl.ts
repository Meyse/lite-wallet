import type { helpEn } from './en';

export const helpNl = {
  'helpCenter.unlockRequired': 'Ontgrendel je wallet om dit scherm te openen.',
  'helpCenter.title': 'Help',
  'helpCenter.homeTitle': 'Waar heb je hulp bij nodig?',
  'helpCenter.backHelp': 'Terug naar Help',
  'helpCenter.backCategory.basics': 'Terug naar aan de slag',
  'helpCenter.backCategory.payments': 'Terug naar versturen en ontvangen',
  'helpCenter.backCategory.conversions': 'Terug naar conversies en cross-chain',
  'helpCenter.backCategory.identity': 'Terug naar VerusID en profielen',
  'helpCenter.backCategory.security': 'Terug naar beveiliging en herstel',
  'helpCenter.backCategory.data': 'Terug naar walletgegevens en instellingen',
  'helpCenter.search': 'Zoeken in help',
  'helpCenter.searchPlaceholder': 'Zoek een vraag of onderwerp',
  'helpCenter.home': 'Alle onderwerpen',
  'helpCenter.suggested': 'Begin hier',
  'helpCenter.topics': 'Helponderwerpen',
  'helpCenter.backWallet': 'Terug naar wallet',
  'helpCenter.backWelcome': 'Terug naar welkom',
  'helpCenter.backUnlock': 'Terug naar inloggen',
  'helpCenter.backResults': 'Terug naar resultaten',
  'helpCenter.backArticle': 'Terug naar artikel',
  'helpCenter.backTopics': 'Terug naar onderwerpen',
  'helpCenter.results': 'Zoekresultaten',
  'helpCenter.resultOne': '1 artikel',
  'helpCenter.resultCount': '{count} artikelen',
  'helpCenter.noResults': 'Geen artikelen gevonden',
  'helpCenter.noResultsHint': 'Probeer een kortere zoekopdracht of andere woorden.',
  'helpCenter.related': 'Gerelateerde artikelen',
  'helpCenter.community': 'Vraag het de community',
  'helpCenter.communityHint': 'Opent Verus Discord',
  'helpCenter.source.identity': 'Meer over VerusID',
  'helpCenter.source.conversions': 'Meer over Verus-conversies',
  'helpCenter.source.bridge': 'Meer over de Verus-Ethereum Bridge',
  'helpCenter.category.basics': 'Aan de slag',
  'helpCenter.category.payments': 'Versturen en ontvangen',
  'helpCenter.category.conversions': 'Conversies en cross-chain',
  'helpCenter.category.identity': 'VerusID en profielen',
  'helpCenter.category.security': 'Beveiliging en herstel',
  'helpCenter.category.data': 'Walletgegevens en instellingen',
  'helpCenter.article.wallet.title': 'Hoe deze wallet werkt',
  'helpCenter.article.wallet.summary':
    'Je wallet bewaart de sleutels waarmee je transacties ondertekent. Je saldi staan op de netwerken die je gebruikt.',
  'helpCenter.article.wallet.body':
    'Bij het aanmaken krijg je een herstelgeheim en stel je een lokaal wachtwoord in. Het wachtwoord ontgrendelt deze installatie; het herstelgeheim kan de toegang tot de sleutels herstellen. Bewaar het op een plek die je zonder dit apparaat kunt bereiken.\n\nEen wallet kan meerdere munten en netwerken tonen. Elk heeft eigen adressen, kosten en bevestigingsregels. Een VerusID is optioneel voor gewone betalingen.',
  'helpCenter.article.wallet.keywords':
    'zelfbeheer account nieuw aanmaken importeren beginner sleutels',
  'helpCenter.article.assets.title': 'Een munt toevoegen of verbergen',
  'helpCenter.article.assets.summary':
    '[[manage-assets|Assets beheren]] bepaalt welke munten je wallet toont. Een munt verbergen verplaatst geen tegoed.',
  'helpCenter.article.assets.body':
    'Kies Assets in Wallet om Assets beheren te openen. Voeg een ondersteunde munt toe of kies Aangepaste asset toevoegen als de munt ontbreekt. Controleer bij aangepaste munten het netwerk en de valuta-ID of het tokencontract via een betrouwbare bron. Alleen de naam of ticker is niet voldoende.\n\nToevoegen maakt de munt zichtbaar; je koopt er niets mee en krijgt geen saldo. Ontbreekt er tegoed, controleer dan eerst het ontvangstadres en netwerk.',
  'helpCenter.article.assets.keywords':
    'munt token erc20 contract toevoegen aangepast verbergen verwijderen portfolio',
  'helpCenter.article.networks.title': 'Het juiste netwerk kiezen',
  'helpCenter.article.networks.summary':
    'De munt, het ontvangstadres en het ontvangende netwerk moeten bij elkaar passen.',
  'helpCenter.article.networks.body':
    'Mainnet en testnet zijn aparte netwerken. Testmunten zijn bedoeld om te testen en kunnen niet als mainnetmunten worden verstuurd. Een adres kan geldig lijken maar bij het verkeerde netwerk of een niet-ondersteunde dienst horen.\n\nGebruik voor een betaling het netwerk dat de ontvanger opgeeft. Controleer bij een cross-chain-transactie het bestemmingsnetwerk in het overzicht. Een bekende ticker of hetzelfde adres garandeert niet dat de ontvanger de munt op dat netwerk ondersteunt.',
  'helpCenter.article.networks.keywords':
    'mainnet testnet vrsctest sepolia verkeerd netwerk adres chain',
  'helpCenter.article.receive.title': 'Een betaling ontvangen',
  'helpCenter.article.receive.summary':
    'Deel het ontvangstadres voor de munt en het netwerk waarop je wilt ontvangen.',
  'helpCenter.article.receive.body':
    'Open [[receive|Ontvangen]], kies de munt en ontvangstmethode en kopieer het adres of toon de QR-code. Controleer het netwerk met de verzender, vooral bij tokens die op meerdere chains bestaan. Een QR-code deelt betaalgegevens; het bevestigt geen betaling.\n\nEen openbaar ontvangstadres mag je delen. Houd herstelzinnen, privésleutels en bestedingssleutels geheim. Open de munt in Wallet en bekijk de transactiegeschiedenis om te controleren of de betaling is aangekomen.',
  'helpCenter.article.receive.keywords': 'storten ontvangen qr code ontvangstadres betaling',
  'helpCenter.article.send.title': 'Een betaling versturen',
  'helpCenter.article.send.summary':
    'Controleer de ontvanger, munt, het netwerk, bedrag en de kosten voordat je goedkeurt.',
  'helpCenter.article.send.body':
    'Kies [[send|Versturen]] vanuit Wallet of een munt. Kies de bron, vul het bedrag en de ontvanger in en open Controleren. Bij ondersteunde Verus-routes kun je een VerusID gebruiken; controleer de identiteit en bestemming die de wallet toont.\n\nHet overzicht kan het verstuurbare bedrag aanpassen om ruimte voor kosten te houden. Lees de uiteindelijke waarden voordat je verstuurt. Na bevestiging kan de wallet de transactie niet terugdraaien. Bij terugkeer uit Help blijft je formulier behouden, maar kan opnieuw controleren nodig zijn.',
  'helpCenter.article.send.keywords':
    'versturen overboeken ontvanger adres controleren max bedrag annuleren',
  'helpCenter.article.pending.title': 'Mijn overboeking is niet aangekomen',
  'helpCenter.article.pending.summary':
    'Indienen en bevestigen zijn aparte stappen. Cross-chain-transacties moeten ook op het bestemmingsnetwerk worden verwerkt.',
  'helpCenter.article.pending.body':
    'Open de munt in Wallet en bekijk de transactiegeschiedenis. Zijn een transactie-ID en explorerlink beschikbaar, controleer de transactie dan op het juiste netwerk. Controleer het ontvangstadres en of de ontvangende dienst meer bevestigingen vereist.\n\nDe wachttijd hangt af van het netwerk, de kosten en de omstandigheden. Een bevestigde brontransactie bewijst op zichzelf niet dat een cross-chain-betaling is aangekomen. Is het indienen onzeker, controleer dan de bestaande transactie voordat je opnieuw betaalt.',
  'helpCenter.article.pending.keywords':
    'vast pending wachten langzaam ontbreekt overboeking niet aangekomen bevestiging geschiedenis activiteit',
  'helpCenter.article.fees.title': 'Waarom een transactie kosten heeft',
  'helpCenter.article.fees.summary':
    'Kosten betalen voor verwerking door het netwerk. De munt waarin je betaalt hangt af van de route.',
  'helpCenter.article.fees.body':
    'Voor het versturen van een Ethereum-token heb je naast het tokensaldo ook ETH op dat Ethereum-netwerk nodig voor gas. Conversies en cross-chain-transacties kunnen conversie- en bridgekosten hebben naast de netwerkkosten.\n\nControleer het totaal afgeschreven bedrag en de munt voor elke kostenpost in het overzicht. Een schatting kan voor het indienen veranderen. Een maximum is een bovengrens voor die kostenpost, geen belofte dat het volledig wordt besteed. Kostenkeuzes beïnvloeden de prioriteit, maar garanderen geen bevestigingstijd.',
  'helpCenter.article.fees.keywords':
    'gas eth onvoldoende tegoed kosten maximum economy standaard duur tarief',
  'helpCenter.article.uncertain.title': 'Indienen kon niet worden bevestigd',
  'helpCenter.article.uncertain.summary':
    'De wallet kan het antwoord na het versturen zijn kwijtgeraakt. Dat bewijst niet dat de transactie is mislukt.',
  'helpCenter.article.uncertain.body':
    'Controleer de transactiegeschiedenis van de munt en een eventueel getoonde transactie-ID. Begin geen dubbele betaling zolang de uitkomst onbekend is.\n\nGebruik bij Ethereum het herstelscherm voor de opgeslagen overboeking als dit verschijnt. Controleer de gegevens en kies de aangeboden optie om door te gaan of de ingediende transactie te bekijken. Dit vervolgt de bestaande handeling. Blijft de uitkomst onduidelijk, vraag dan hulp met het netwerk, de foutmelding en de openbare transactie-ID.',
  'helpCenter.article.uncertain.keywords':
    'onzeker broadcast timeout mislukt opnieuw dubbel ethereum herstel indienen',
  'helpCenter.article.convert.title': 'Hoe conversies werken',
  'helpCenter.article.convert.summary':
    'Een conversie wisselt een munt om voor een andere via een ondersteunde Verus-valutamand.',
  'helpCenter.article.convert.body':
    'Een valutamand bevat reservemunten. Het protocol berekent conversieprijzen uit de reserves en verwerkt conversies gezamenlijk via consensus. Wat je ontvangt hangt af van de route, het bedrag, de reservesaldi en de kosten.\n\nKies [[convert|Omwisselen]], selecteer wat je betaalt en ontvangt en controleer de bestemming en schatting. Een conversie kan ook naar een ander ondersteund netwerk leveren. Alleen beschikbare routes worden getoond; een token toevoegen maakt het niet automatisch converteerbaar.',
  'helpCenter.article.convert.keywords':
    'swap omwisselen wisselen handelen converteren conversie mand reserve defi pbaas',
  'helpCenter.article.conversion-estimate.title': 'Waarom het conversiebedrag kan veranderen',
  'helpCenter.article.conversion-estimate.summary':
    'Het te ontvangen bedrag is een schatting totdat het netwerk de conversie verwerkt.',
  'helpCenter.article.conversion-estimate.body':
    'Andere conversies kunnen de reserves wijzigen tussen je prijsopgave en de afwikkeling. Ook de grootte van je conversie beïnvloedt de prijs. Deze prijsbeweging heet vaak slippage. De wallet toont de route en geschatte uitkomst voordat je goedkeurt.\n\nLees het meest recente overzicht, inclusief conversie- en netwerkkosten. Vernieuw een verlopen overzicht. De fiatwaarde in je portfolio is een aparte marktschatting en niet de koers waartegen je conversie wordt afgewikkeld.',
  'helpCenter.article.conversion-estimate.keywords':
    'slippage schatting prijsopgave koers prijs minimum ontvangen uiteindelijk bedrag mand',
  'helpCenter.article.cross-chain.title': 'Hoe cross-chain-transacties werken',
  'helpCenter.article.cross-chain.summary':
    'Een cross-chain-transactie verplaatst waarde van het ene netwerk naar het andere via een ondersteunde route.',
  'helpCenter.article.cross-chain.body':
    'Het bronnetwerk legt de transactie eerst vast. Bewijs daarvan moet vervolgens op het bestemmingsnetwerk worden geaccepteerd en verwerkt. Dit kan langer duren dan een betaling binnen één chain.\n\nControleer het bestemmingsnetwerk en de te ontvangen munt in het overzicht. Een brontransactie-ID of bronbevestiging bewijst niet dat het bedrag is aangekomen. Gebruik de beschikbare explorergegevens en houd rekening met extra verificatie en de eisen van de ontvangende dienst. Beschikbaarheid hangt af van de route en het netwerk; dit artikel toont geen actuele bridgestatus.',
  'helpCenter.article.cross-chain.keywords':
    'bridge cross chain cross-chain export import bestemming pbaas vertraagd',
  'helpCenter.article.ethereum.title': 'Ethereum-bridge en tokengoedkeuringen',
  'helpCenter.article.ethereum.summary':
    'Een bridgeoverboeking van een Ethereum-token kan eerst een tokengoedkeuring vereisen.',
  'helpCenter.article.ethereum.body':
    'Een goedkeuring geeft het bridgecontract toestemming om het tokenbedrag te gebruiken. Dit is een aparte on-chain-handeling waarvoor gas nodig kan zijn. Alleen de goedkeuring betekent niet dat de overboeking is ingediend of ontvangen. Houd voldoende ETH op het bronnetwerk voor de benodigde stappen.\n\nVolg het overzicht en eventuele herstelstappen voor de opgeslagen transactie. Slaagt de goedkeuring maar mislukt de volgende stap, vervolg dan de opgeslagen handeling wanneer dat wordt aangeboden. Bridgeroutes kunnen uitgeschakeld of onbeschikbaar zijn; niet elk Ethereum-token kan worden overgezet.',
  'helpCenter.article.ethereum.keywords':
    'eth ethereum erc20 goedkeuring allowance gas bridge doorgaan herstel',
  'helpCenter.article.verusid.title': 'Wat is een VerusID?',
  'helpCenter.article.verusid.summary':
    'Een VerusID is een on-chain-identiteit met een naam, identiteitsadres en regels voor wie deze kan beheren.',
  'helpCenter.article.verusid.body':
    'Je kunt een VerusID gebruiken als betaalbestemming op ondersteunde routes en bij apps die VerusID ondersteunen. De primaire adressen en ondertekeningsregels bepalen wie handelingen mag goedkeuren. Intrekkings- en herstelbevoegdheden bieden aparte controles.\n\nEen openbaar profiel kan afbeeldingen en een beschrijving toevoegen. Apps bepalen zelf welke identiteits- en profielonderdelen ze tonen. Een naam of profielfoto bewijst op zichzelf niet dat iemand betrouwbaar is.',
  'helpCenter.article.verusid.keywords':
    'identiteit naam handle adres eigendom account meeneembaar',
  'helpCenter.article.link-identity.title': 'Een VerusID koppelen of ontkoppelen',
  'helpCenter.article.link-identity.summary':
    'Koppelen voegt een bestaande identiteit aan deze wallet toe. Het registreert geen nieuwe identiteit en draagt geen eigendom over.',
  'helpCenter.article.link-identity.body':
    'Open [[verusid|VerusID]] en kies VerusID koppelen om een bestaande identiteit op het gekozen netwerk te vinden. De wallet controleert de identiteit en wat deze wallet ermee mag doen. Een identiteit of saldo kunnen zien betekent niet automatisch dat je deze beheert.\n\nOntkoppelen verwijdert de lokale koppeling. De identiteit, het tegoed en het openbare profiel blijven on-chain bestaan. De identiteit zelf wijzigen vereist een bevoegde on-chain-update.',
  'helpCenter.article.link-identity.keywords':
    'koppelen ontkoppelen zoeken opzoeken registreren registratie eigenaar alleen lezen',
  'helpCenter.article.public-profile.title': 'Wat wordt openbaar in mijn profiel?',
  'helpCenter.article.public-profile.summary':
    'Publiceren slaat je gekozen profielafbeeldingen en beschrijving op de openbare Verus-blockchain op.',
  'helpCenter.article.public-profile.body':
    'Iedereen kan gepubliceerde profielgegevens lezen. Apps die dit profiel ondersteunen kunnen het via je VerusID tonen; de informatie is dus niet aan deze installatie gebonden. Elke app bepaalt welke velden zij ondersteunt.\n\nLatere wijzigingen of verwijderingen passen het huidige profiel aan; eerdere versies blijven in de blockchaingeschiedenis. Publiceer alleen informatie die je openbaar wilt maken. Privénotities bij contacten en herstelgeheimen worden niet in een profielupdate opgenomen.',
  'helpCenter.article.public-profile.keywords':
    'on chain on-chain openbaar permanent privacy foto avatar header beschrijving verwijderen geschiedenis',
  'helpCenter.article.publish-profile.title': 'Een profiel bewerken en publiceren',
  'helpCenter.article.publish-profile.summary':
    'Wijzigingen beginnen als lokaal concept. Publiceren vereist controle, netwerkkosten en een bevoegde handtekening.',
  'helpCenter.article.publish-profile.body':
    'Open je gekoppelde [[verusid|VerusID]] en kies Profiel bewerken als dit beschikbaar is. Wijzig de avatar, header of beschrijving en controleer de wijzigingen en kosten. Deze versie publiceert profielen alleen voor ondersteunde VRSCTEST-identiteiten. De identiteit moet actief zijn en een ondersteunde beheervorm hebben.\n\nAfhankelijk van de afbeeldingen zijn één of twee updates mogelijk. Bij twee updates moet de eerste bevestigd zijn voordat je de tweede controleert en goedkeurt. Gewone ongepubliceerde concepten blijven tijdens de huidige ontgrendelingssessie beschikbaar. Een opgeslagen vervolg na de eerste update heeft een aparte herstelroute.',
  'helpCenter.article.publish-profile.keywords':
    'bewerken publiceren profiel afbeeldingen avatar header kosten concept testnet vrsctest twee updates',
  'helpCenter.article.profile-pending.title': 'Waarom mijn profiel nog de oude versie toont',
  'helpCenter.article.profile-pending.summary':
    'De wallet toont het laatst bevestigde profiel zolang een update op bevestiging wacht.',
  'helpCenter.article.profile-pending.body':
    'Bekijk de ingediende wijzigingen en transactiegegevens om te zien wat is verstuurd. Een ingediende update is nog geen nieuw bevestigd profiel. Mislukt het controleren van de bevestiging, probeer die controle dan opnieuw voordat je nogmaals publiceert.\n\nBij publicatie in twee updates kunnen de avatar en beschrijving eerder bevestigd zijn dan de header. Controleer en keur de resterende headerupdate goed wanneer dit wordt aangeboden. Andere apps vernieuwen hun profielgegevens op eigen momenten en kunnen achterlopen op de chain.',
  'helpCenter.article.profile-pending.keywords':
    'profiel pending oud afbeelding ongewijzigd wachten bevestigen header tweede update later opgeslagen',
  'helpCenter.article.profile-unavailable.title':
    'Waarom een profiel niet getoond of bewerkt kan worden',
  'helpCenter.article.profile-unavailable.summary':
    'Een ontbrekend profiel, een onleesbaar profiel en een profiel zonder bewerkrechten zijn verschillende toestanden.',
  'helpCenter.article.profile-unavailable.body':
    'Een identiteit heeft mogelijk nog geen openbaar profiel. Als profielgegevens niet geladen of gecontroleerd kunnen worden, kan de wallet nog wel de basisgegevens tonen. Probeer opnieuw wanneer je verbinding beschikbaar is.\n\nBewerken vereist dat deze wallet een ondersteunde actieve identiteit beheert. Publiceren op mainnet, tokenbeheerde identiteiten en niet-ondersteunde ondertekeningsvormen zijn niet beschikbaar in deze editor. Lees de getoonde reden. Opnieuw koppelen geeft geen ondertekeningsrecht.',
  'helpCenter.article.profile-unavailable.keywords':
    'profiel niet beschikbaar alleen lezen kan niet bewerken ontbreekt rechten mainnet',
  'helpCenter.article.recovery.title': 'Wat moet ik als back-up bewaren?',
  'helpCenter.article.recovery.summary':
    'Bewaar het herstelgeheim voor elk tegoed dat je gebruikt, inclusief een apart Private Verus-geheim als je dat hebt aangemaakt of geïmporteerd.',
  'helpCenter.article.recovery.body':
    'Open Instellingen en vervolgens [[profile-security|Profiel en beveiliging]] om herstelgegevens te bekijken. Herstelgegevens voor Private Verus zijn ook via de eigen instellingen bereikbaar. Volg de aanwijzingen en bewaar een nauwkeurige offline kopie. Een apart geïmporteerde sleutel kan alleen de eigen adressen beschermen.\n\nIedereen met een herstelzin of bestedingssleutel kan mogelijk het bijbehorende tegoed gebruiken. Deel deze niet in chat, screenshots of een hulpformulier. Je lokale wachtwoord en herstelgeheim hebben verschillende doelen; alleen het wachtwoord herstelt een verloren installatie niet.',
  'helpCenter.article.recovery.keywords':
    'backup back-up seed herstelzin geheim privé bestedingssleutel 24 woorden veiligheid apparaat kwijt',
  'helpCenter.article.forgot-password.title': 'Ik kan mijn wallet niet ontgrendelen',
  'helpCenter.article.forgot-password.summary':
    'Een vergeten lokaal wachtwoord kan niet door de wallet of community worden teruggehaald.',
  'helpCenter.article.forgot-password.body':
    'Heb je de oorspronkelijke herstelzin of een ander ondersteund herstelgeheim, importeer dat dan als wallet en stel een nieuw lokaal wachtwoord in. Kies de importmethode die bij je back-up past en gebruik het juiste netwerk. Mogelijk moet je ook een apart Private Verus-geheim herstellen.\n\nBewaar de oude installatie en bestanden totdat je de herstelde adressen en saldi hebt gecontroleerd. Sleutels importeren herstelt geen lokale contacten of notities. VerusID-herstel is een apart mechanisme en werkt alleen als de vereiste bevoegdheid herstel nog kan ondertekenen.',
  'helpCenter.article.forgot-password.keywords':
    'wachtwoord vergeten toegang kwijt vergrendeld ontgrendelen reset herstellen importeren apparaat',
  'helpCenter.article.guard.title': 'Wat VerusID Guard kan herstellen',
  'helpCenter.article.guard.summary':
    'VerusID Guard helpt een bevoegde intrekkings- of herstelautoriteit een VerusID te beheren.',
  'helpCenter.article.guard.body':
    'Intrekken blokkeert normaal gebruik zodra de intrekking on-chain van kracht is. Herstel kan een ingetrokken identiteit nieuw primair beheer geven. Beide vereisen de juiste bevoegdheid en een on-chain-transactie. Een autoriteit instellen helpt niet als je ook de toegang tot diens sleutels kwijtraakt.\n\nGuard is beschikbaar op de welkom- en ontgrendelschermen. Het reset geen lokaal wachtwoord en herstelt geen willekeurig Bitcoin-, Ethereum- of gewoon adrestegoed. Bij gelekte sleutels betreft identiteitsherstel alleen wat die identiteit beheert. Controleer identiteit, bevoegdheden, netwerk en wijzigingen voordat je goedkeurt.',
  'helpCenter.article.guard.keywords':
    'intrekken herstellen guard gestolen gelekt bevriezen autoriteit identiteit sleutels kwijt',
  'helpCenter.article.private-verus.title': 'Private Verus instellen',
  'helpCenter.article.private-verus.summary':
    'Private Verus gebruikt een afgeschermd adres en vereist eigen herstelgegevens en synchronisatie.',
  'helpCenter.article.private-verus.body':
    'Open Instellingen en vervolgens [[private-verus|Private Verus]]. Afhankelijk van je wallet kun je de primaire herstelzin hergebruiken, een nieuw privacyherstelgeheim aanmaken of een bestaand geheim importeren. Bewaar een apart aangemaakt of geïmporteerd geheim ook apart. De primaire walletback-up alleen herstelt dat privétegoed mogelijk niet.\n\nLaat de privésynchronisatie afronden voordat je verstuurt. Die zoekt op de chain naar tegoed dat de privésleutels kunnen gebruiken. Een nieuwe installatie of hersteld geheim kan tijd nodig hebben om het volledige saldo te vinden. Volg de getoonde configuratie- of herstartaanwijzingen.',
  'helpCenter.article.private-verus.keywords':
    'privacy afgeschermd zs sapling dlight sync synchroniseren privé instellen seed',
  'helpCenter.article.private-send.title': 'Privéadressen en transactiememos',
  'helpCenter.article.private-send.summary':
    'Een afgeschermd adres beschermt andere informatie dan een openbaar ontvangstadres. Controleer beide kanten van de transactie.',
  'helpCenter.article.private-send.body':
    'Private Verus kan naar ondersteunde afgeschermde of transparante Verus-bestemmingen versturen. Bij een openbaar adres is de ontvangende kant on-chain zichtbaar. Een privébron maakt een openbare bestemming niet privé.\n\nPrivémemos worden ondersteund voor afgeschermde bestemmingen. De wallet moet privésynchronisatie afronden en het transactiebewijs voorbereiden voordat zij kan indienen. Het maken van dat bewijs kan tijd kosten. Controleer bij een onzekere indiening de geschiedenis voordat je opnieuw verstuurt.',
  'helpCenter.article.private-send.keywords':
    'memo bericht afgeschermd transparant zs privé bewijs langzaam privacy',
  'helpCenter.article.requests.title': 'Een verzoek van een app controleren',
  'helpCenter.article.requests.summary':
    'Een verzoek vraagt de wallet bepaalde handelingen uit te voeren. Controleer de aanvrager en elke handeling voordat je goedkeurt.',
  'helpCenter.article.requests.body':
    'Gebruik Open request of open een ondersteunde walletlink. De wallet controleert het verzoek en toont de handelingen die zij ondersteunt. Dit kunnen authenticatie en identiteitswijzigingen zijn; sommige vereisen ook financiering en kosten.\n\nEen geldige handtekening identificeert de ondertekenaar, maar maakt het verzoek niet automatisch wenselijk. Controleer zelf de identiteit, bevoegdheden, bestemming en wijzigingen. Wijs onverwachte verzoeken af. Apps is momenteel geen appcatalogus en Activiteit toont in deze versie geen transactiegeschiedenis; bekijk daarvoor een munt.',
  'helpCenter.article.requests.keywords':
    'app apps verzoek deeplink link ondertekenen handtekening authenticatie inloggen rechten activiteit',
  'helpCenter.article.contacts.title': 'Contacten en openbare profielen',
  'helpCenter.article.contacts.summary':
    '[[contacts|Contacten]] bewaren ontvangers en privénotities in deze wallet. Een gekoppeld openbaar profiel komt van de VerusID.',
  'helpCenter.article.contacts.body':
    'Sla een contact met een ondersteund adres op of koppel een VerusID. Een openbaar profiel kan helpen de identiteit te herkennen; je eigen notitie blijft lokaal. Controleer bij elke betaling het gekozen ontvangstadres en netwerk.\n\nEen contact verwijderen verwijdert de lokale vermelding. Het verwijdert geen VerusID, openbaar profiel of blockchaintransacties. Contacten worden in deze installatie versleuteld bewaard en komen niet terug door alleen de wallet-herstelzin op een ander apparaat te importeren.',
  'helpCenter.article.contacts.keywords':
    'contact adresboek ontvanger privénotitie opslaan verwijderen profiel',
  'helpCenter.article.watchlist.title': 'Wat de volglijst toont',
  'helpCenter.article.watchlist.summary':
    'De [[watchlist|volglijst]] volgt openbaar tegoed van ondersteunde Verus-adressen en identiteiten.',
  'helpCenter.article.watchlist.body':
    'Een adres of identiteit toevoegen laat je de beschikbare openbare saldi bekijken. Het importeert geen sleutel en geeft geen bestedingsrecht. Gevolgd tegoed staat los van tegoed dat deze wallet beheert.\n\nResultaten hangen af van de ondersteunde netwerken en bereikbare gegevens. Een onbeschikbaar of gedeeltelijk geladen resultaat bewijst geen nulsaldo. Privésaldi kunnen niet via een openbare adreszoekopdracht worden gevonden. Een gevolgde vermelding verwijderen wijzigt alleen je lokale lijst.',
  'helpCenter.article.watchlist.keywords':
    'volgen volglijst alleen lezen openbaar saldo adres identiteit tegoed',
  'helpCenter.article.local-data.title': 'Wat blijft op dit apparaat?',
  'helpCenter.article.local-data.summary':
    'Sleutels en lokale walletgegevens hebben een ander doel dan openbare blockchainrecords.',
  'helpCenter.article.local-data.body':
    'Contacten, privénotities, volglijstkeuzes en weergave-instellingen zijn lokale walletgegevens. Een VerusID-profiel publiceren publiceert deze niet. Een herstelgeheim importeren herstelt de bijbehorende sleutels, geen kopie van elke lokale instelling of elk contact.\n\nOpenbare transacties en gepubliceerde profielgeschiedenis blijven op hun netwerken. De wallet vergrendelen beschermt lokale toegang; het verbergt geen informatie die al openbaar is. Bewaar de oorspronkelijke walletbestanden totdat je een verhuizing naar een nieuwe installatie hebt gecontroleerd.',
  'helpCenter.article.local-data.keywords':
    'lokaal gegevens apparaat privacy herstellen backup back-up sync contacten notities opslag openbaar',
  'helpCenter.article.settings.title': 'Weergave, taal en walletvergrendeling',
  'helpCenter.article.settings.summary':
    'Instellingen beheert weergave, fiatvaluta, taal, automatisch vergrendelen en toegang tot herstelgegevens.',
  'helpCenter.article.settings.body':
    'Kies [[display-language|Weergave en taal]] voor een thema, taal of fiatvaluta. Een andere weergavevaluta wijzigt schattingen in de interface; het converteert je tegoed niet.\n\n[[profile-security|Profiel en beveiliging]] bevat de vergrendeling bij inactiviteit en toegang tot herstelgegevens. Vergrendelen beëindigt de ontgrendelde sessie; rond je werk eerst af of verlaat het veilig. [[private-verus|Private Verus]] heeft een eigen configuratiepagina. [[about-support|Over en ondersteuning]] toont de appversie, die helpt bij het melden van problemen.',
  'helpCenter.article.settings.keywords':
    'instellingen thema donker licht taal fiat euro valuta automatisch vergrendelen versie',
  'helpCenter.article.support.title': 'Hulp vragen aan de community',
  'helpCenter.article.support.summary':
    'Beschrijf wat je probeerde, wat je verwachtte en welke melding de wallet toonde.',
  'helpCenter.article.support.body':
    'Vermeld de appversie, het netwerk en de munt. Een openbare transactie-ID kan helpen bij een overboekingsprobleem, maar kan ook transactiegegevens onthullen. Controleer screenshots voordat je ze deelt en verwijder privégegevens.\n\nKies Vraag het de community om Verus Discord te openen. Stuur nooit een herstelzin, privésleutel, bestedingssleutel of wachtwoord naar iemand die hulp aanbiedt. De Help-artikelen werken offline; communitylinks, actuele saldi en netwerkzoekopdrachten vereisen een verbinding.',
  'helpCenter.article.support.keywords':
    'ondersteuning community discord fout probleem melden bug offline hulp',
} satisfies Record<keyof typeof helpEn, string>;
