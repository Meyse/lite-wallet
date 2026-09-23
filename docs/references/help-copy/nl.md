---
owner: lite-wallet-team
last_reviewed: 2026-09-22
---

# Wallet Help teksten in het Nederlands

Hieronder staan alle 29 Help-artikelen, ingedeeld zoals in de app. Je kunt de
titels en alinea’s direct aanpassen. Laat de referentiecomments staan, zodat
elke wijziging aan het juiste artikel gekoppeld kan worden.

Sla het bestand op en zeg tegen Codex: “Ik heb de Help-teksten aangepast. Wil je
mijn wijzigingen in de app verwerken?” De app verandert pas wanneer je de
wijzigingen laat toepassen.

[Engelse teksten](./en.md)

<!-- help-category: basics -->

## Aan de slag

<!-- help-article: wallet -->

### Hoe deze wallet werkt

Je wallet bewaart de sleutels waarmee je transacties ondertekent. Je saldi staan
op de netwerken die je gebruikt.

Bij het aanmaken krijg je een herstelgeheim en stel je een lokaal wachtwoord in.
Het wachtwoord ontgrendelt deze installatie; het herstelgeheim kan de toegang
tot de sleutels herstellen. Bewaar het op een plek die je zonder dit apparaat
kunt bereiken.

Een wallet kan meerdere munten en netwerken tonen. Elk heeft eigen adressen,
kosten en bevestigingsregels. Een VerusID is optioneel voor gewone betalingen.

<!-- help-article: assets -->

### Een munt toevoegen of verbergen

Assets beheren bepaalt welke munten je wallet toont. Een munt verbergen
verplaatst geen tegoed.

Kies Assets in Wallet om Assets beheren te openen. Voeg een ondersteunde munt
toe of kies Aangepaste asset toevoegen als de munt ontbreekt. Controleer bij
aangepaste munten het netwerk en de valuta-ID of het tokencontract via een
betrouwbare bron. Alleen de naam of ticker is niet voldoende.

Toevoegen maakt de munt zichtbaar; je koopt er niets mee en krijgt geen saldo.
Ontbreekt er tegoed, controleer dan eerst het ontvangstadres en netwerk.

<!-- help-article: networks -->

### Het juiste netwerk kiezen

De munt, het ontvangstadres en het ontvangende netwerk moeten bij elkaar passen.

Mainnet en testnet zijn aparte netwerken. Testmunten zijn bedoeld om te testen
en kunnen niet als mainnetmunten worden verstuurd. Een adres kan geldig lijken
maar bij het verkeerde netwerk of een niet-ondersteunde dienst horen.

Gebruik voor een betaling het netwerk dat de ontvanger opgeeft. Controleer bij
een cross-chain-transactie het bestemmingsnetwerk in het overzicht. Een bekende
ticker of hetzelfde adres garandeert niet dat de ontvanger de munt op dat
netwerk ondersteunt.

<!-- help-category: payments -->

## Versturen en ontvangen

<!-- help-article: receive -->

### Een betaling ontvangen

Deel het ontvangstadres voor de munt en het netwerk waarop je wilt ontvangen.

Open Ontvangen, kies de munt en ontvangstmethode en kopieer het adres of toon de
QR-code. Controleer het netwerk met de verzender, vooral bij tokens die op
meerdere chains bestaan. Een QR-code deelt betaalgegevens; het bevestigt geen
betaling.

Een openbaar ontvangstadres mag je delen. Houd herstelzinnen, privésleutels en
bestedingssleutels geheim. Open de munt in Wallet en bekijk de
transactiegeschiedenis om te controleren of de betaling is aangekomen.

<!-- help-article: send -->

### Een betaling versturen

Controleer de ontvanger, munt, het netwerk, bedrag en de kosten voordat je
goedkeurt.

Kies Versturen vanuit Wallet of een munt. Kies de bron, vul het bedrag en de
ontvanger in en open Controleren. Bij ondersteunde Verus-routes kun je een
VerusID gebruiken; controleer de identiteit en bestemming die de wallet toont.

Het overzicht kan het verstuurbare bedrag aanpassen om ruimte voor kosten te
houden. Lees de uiteindelijke waarden voordat je verstuurt. Na bevestiging kan
de wallet de transactie niet terugdraaien. Bij terugkeer uit Help blijft je
formulier behouden, maar kan opnieuw controleren nodig zijn.

<!-- help-article: pending -->

### Mijn overboeking is niet aangekomen

Indienen en bevestigen zijn aparte stappen. Cross-chain-transacties moeten ook
op het bestemmingsnetwerk worden verwerkt.

Open de munt in Wallet en bekijk de transactiegeschiedenis. Zijn een
transactie-ID en explorerlink beschikbaar, controleer de transactie dan op het
juiste netwerk. Controleer het ontvangstadres en of de ontvangende dienst meer
bevestigingen vereist.

De wachttijd hangt af van het netwerk, de kosten en de omstandigheden. Een
bevestigde brontransactie bewijst op zichzelf niet dat een cross-chain-betaling
is aangekomen. Is het indienen onzeker, controleer dan de bestaande transactie
voordat je opnieuw betaalt.

<!-- help-article: fees -->

### Waarom een transactie kosten heeft

Kosten betalen voor verwerking door het netwerk. De munt waarin je betaalt hangt
af van de route.

Voor het versturen van een Ethereum-token heb je naast het tokensaldo ook ETH op
dat Ethereum-netwerk nodig voor gas. Conversies en cross-chain-transacties
kunnen conversie- en bridgekosten hebben naast de netwerkkosten.

Controleer het totaal afgeschreven bedrag en de munt voor elke kostenpost in het
overzicht. Een schatting kan voor het indienen veranderen. Een maximum is een
bovengrens voor die kostenpost, geen belofte dat het volledig wordt besteed.
Kostenkeuzes beïnvloeden de prioriteit, maar garanderen geen bevestigingstijd.

<!-- help-article: uncertain -->

### Indienen kon niet worden bevestigd

De wallet kan het antwoord na het versturen zijn kwijtgeraakt. Dat bewijst niet
dat de transactie is mislukt.

Controleer de transactiegeschiedenis van de munt en een eventueel getoonde
transactie-ID. Begin geen dubbele betaling zolang de uitkomst onbekend is.

Gebruik bij Ethereum het herstelscherm voor de opgeslagen overboeking als dit
verschijnt. Controleer de gegevens en kies de aangeboden optie om door te gaan
of de ingediende transactie te bekijken. Dit vervolgt de bestaande handeling.
Blijft de uitkomst onduidelijk, vraag dan hulp met het netwerk, de foutmelding
en de openbare transactie-ID.

<!-- help-category: conversions -->

## Conversies en cross-chain

<!-- help-article: convert -->

### Hoe conversies werken

Een conversie wisselt een munt om voor een andere via een ondersteunde
Verus-valutamand.

Een valutamand bevat reservemunten. Het protocol berekent conversieprijzen uit
de reserves en verwerkt conversies gezamenlijk via consensus. Wat je ontvangt
hangt af van de route, het bedrag, de reservesaldi en de kosten.

Kies Omwisselen, selecteer wat je betaalt en ontvangt en controleer de
bestemming en schatting. Een conversie kan ook naar een ander ondersteund
netwerk leveren. Alleen beschikbare routes worden getoond; een token toevoegen
maakt het niet automatisch converteerbaar.

<!-- help-article: conversion-estimate -->

### Waarom het conversiebedrag kan veranderen

Het te ontvangen bedrag is een schatting totdat het netwerk de conversie
verwerkt.

Andere conversies kunnen de reserves wijzigen tussen je prijsopgave en de
afwikkeling. Ook de grootte van je conversie beïnvloedt de prijs. Deze
prijsbeweging heet vaak slippage. De wallet toont de route en geschatte uitkomst
voordat je goedkeurt.

Lees het meest recente overzicht, inclusief conversie- en netwerkkosten.
Vernieuw een verlopen overzicht. De fiatwaarde in je portfolio is een aparte
marktschatting en niet de koers waartegen je conversie wordt afgewikkeld.

<!-- help-article: cross-chain -->

### Hoe cross-chain-transacties werken

Een cross-chain-transactie verplaatst waarde van het ene netwerk naar het andere
via een ondersteunde route.

Het bronnetwerk legt de transactie eerst vast. Bewijs daarvan moet vervolgens op
het bestemmingsnetwerk worden geaccepteerd en verwerkt. Dit kan langer duren dan
een betaling binnen één chain.

Controleer het bestemmingsnetwerk en de te ontvangen munt in het overzicht. Een
brontransactie-ID of bronbevestiging bewijst niet dat het bedrag is aangekomen.
Gebruik de beschikbare explorergegevens en houd rekening met extra verificatie
en de eisen van de ontvangende dienst. Beschikbaarheid hangt af van de route en
het netwerk; dit artikel toont geen actuele bridgestatus.

<!-- help-article: ethereum -->

### Ethereum-bridge en tokengoedkeuringen

Een bridgeoverboeking van een Ethereum-token kan eerst een tokengoedkeuring
vereisen.

Een goedkeuring geeft het bridgecontract toestemming om het tokenbedrag te
gebruiken. Dit is een aparte on-chain-handeling waarvoor gas nodig kan zijn.
Alleen de goedkeuring betekent niet dat de overboeking is ingediend of
ontvangen. Houd voldoende ETH op het bronnetwerk voor de benodigde stappen.

Volg het overzicht en eventuele herstelstappen voor de opgeslagen transactie.
Slaagt de goedkeuring maar mislukt de volgende stap, vervolg dan de opgeslagen
handeling wanneer dat wordt aangeboden. Bridgeroutes kunnen uitgeschakeld of
onbeschikbaar zijn; niet elk Ethereum-token kan worden overgezet.

<!-- help-category: identity -->

## VerusID en profielen

<!-- help-article: verusid -->

### Wat is een VerusID?

Een VerusID is een on-chain-identiteit met een naam, identiteitsadres en regels
voor wie deze kan beheren.

Je kunt een VerusID gebruiken als betaalbestemming op ondersteunde routes en bij
apps die VerusID ondersteunen. De primaire adressen en ondertekeningsregels
bepalen wie handelingen mag goedkeuren. Intrekkings- en herstelbevoegdheden
bieden aparte controles.

Een openbaar profiel kan afbeeldingen en een beschrijving toevoegen. Apps
bepalen zelf welke identiteits- en profielonderdelen ze tonen. Een naam of
profielfoto bewijst op zichzelf niet dat iemand betrouwbaar is.

<!-- help-article: link-identity -->

### Een VerusID koppelen of ontkoppelen

Koppelen voegt een bestaande identiteit aan deze wallet toe. Het registreert
geen nieuwe identiteit en draagt geen eigendom over.

Open VerusID en kies VerusID koppelen om een bestaande identiteit op het gekozen
netwerk te vinden. De wallet controleert de identiteit en wat deze wallet ermee
mag doen. Een identiteit of saldo kunnen zien betekent niet automatisch dat je
deze beheert.

Ontkoppelen verwijdert de lokale koppeling. De identiteit, het tegoed en het
openbare profiel blijven on-chain bestaan. De identiteit zelf wijzigen vereist
een bevoegde on-chain-update.

<!-- help-article: public-profile -->

### Wat wordt openbaar in mijn profiel?

Publiceren slaat je gekozen profielafbeeldingen en beschrijving op de openbare
Verus-blockchain op.

Iedereen kan gepubliceerde profielgegevens lezen. Apps die dit profiel
ondersteunen kunnen het via je VerusID tonen; de informatie is dus niet aan deze
installatie gebonden. Elke app bepaalt welke velden zij ondersteunt.

Latere wijzigingen of verwijderingen passen het huidige profiel aan; eerdere
versies blijven in de blockchaingeschiedenis. Publiceer alleen informatie die je
openbaar wilt maken. Privénotities bij contacten en herstelgeheimen worden niet
in een profielupdate opgenomen.

<!-- help-article: publish-profile -->

### Een profiel bewerken en publiceren

Wijzigingen beginnen als lokaal concept. Publiceren vereist controle,
netwerkkosten en een bevoegde handtekening.

Open je gekoppelde VerusID en kies Profiel bewerken als dit beschikbaar is.
Wijzig de avatar, header of beschrijving en controleer de wijzigingen en kosten.
Deze versie publiceert profielen alleen voor ondersteunde VRSCTEST-identiteiten.
De identiteit moet actief zijn en een ondersteunde beheervorm hebben.

Afhankelijk van de afbeeldingen zijn één of twee updates mogelijk. Bij twee
updates moet de eerste bevestigd zijn voordat je de tweede controleert en
goedkeurt. Gewone ongepubliceerde concepten blijven tijdens de huidige
ontgrendelingssessie beschikbaar. Een opgeslagen vervolg na de eerste update
heeft een aparte herstelroute.

<!-- help-article: profile-pending -->

### Waarom mijn profiel nog de oude versie toont

De wallet toont het laatst bevestigde profiel zolang een update op bevestiging
wacht.

Bekijk de ingediende wijzigingen en transactiegegevens om te zien wat is
verstuurd. Een ingediende update is nog geen nieuw bevestigd profiel. Mislukt
het controleren van de bevestiging, probeer die controle dan opnieuw voordat je
nogmaals publiceert.

Bij publicatie in twee updates kunnen de avatar en beschrijving eerder bevestigd
zijn dan de header. Controleer en keur de resterende headerupdate goed wanneer
dit wordt aangeboden. Andere apps vernieuwen hun profielgegevens op eigen
momenten en kunnen achterlopen op de chain.

<!-- help-article: profile-unavailable -->

### Waarom een profiel niet getoond of bewerkt kan worden

Een ontbrekend profiel, een onleesbaar profiel en een profiel zonder
bewerkrechten zijn verschillende toestanden.

Een identiteit heeft mogelijk nog geen openbaar profiel. Als profielgegevens
niet geladen of gecontroleerd kunnen worden, kan de wallet nog wel de
basisgegevens tonen. Probeer opnieuw wanneer je verbinding beschikbaar is.

Bewerken vereist dat deze wallet een ondersteunde actieve identiteit beheert.
Publiceren op mainnet, tokenbeheerde identiteiten en niet-ondersteunde
ondertekeningsvormen zijn niet beschikbaar in deze editor. Lees de getoonde
reden. Opnieuw koppelen geeft geen ondertekeningsrecht.

<!-- help-category: security -->

## Beveiliging en herstel

<!-- help-article: recovery -->

### Wat moet ik als back-up bewaren?

Bewaar het herstelgeheim voor elk tegoed dat je gebruikt, inclusief een apart
Private Verus-geheim als je dat hebt aangemaakt of geïmporteerd.

Open Instellingen en vervolgens Profiel en beveiliging om herstelgegevens te
bekijken. Herstelgegevens voor Private Verus zijn ook via de eigen instellingen
bereikbaar. Volg de aanwijzingen en bewaar een nauwkeurige offline kopie. Een
apart geïmporteerde sleutel kan alleen de eigen adressen beschermen.

Iedereen met een herstelzin of bestedingssleutel kan mogelijk het bijbehorende
tegoed gebruiken. Deel deze niet in chat, screenshots of een hulpformulier. Je
lokale wachtwoord en herstelgeheim hebben verschillende doelen; alleen het
wachtwoord herstelt een verloren installatie niet.

<!-- help-article: forgot-password -->

### Ik kan mijn wallet niet ontgrendelen

Een vergeten lokaal wachtwoord kan niet door de wallet of community worden
teruggehaald.

Heb je de oorspronkelijke herstelzin of een ander ondersteund herstelgeheim,
importeer dat dan als wallet en stel een nieuw lokaal wachtwoord in. Kies de
importmethode die bij je back-up past en gebruik het juiste netwerk. Mogelijk
moet je ook een apart Private Verus-geheim herstellen.

Bewaar de oude installatie en bestanden totdat je de herstelde adressen en saldi
hebt gecontroleerd. Sleutels importeren herstelt geen lokale contacten of
notities. VerusID-herstel is een apart mechanisme en werkt alleen als de
vereiste bevoegdheid herstel nog kan ondertekenen.

<!-- help-article: guard -->

### Wat VerusID Guard kan herstellen

VerusID Guard helpt een bevoegde intrekkings- of herstelautoriteit een VerusID
te beheren.

Intrekken blokkeert normaal gebruik zodra de intrekking on-chain van kracht is.
Herstel kan een ingetrokken identiteit nieuw primair beheer geven. Beide
vereisen de juiste bevoegdheid en een on-chain-transactie. Een autoriteit
instellen helpt niet als je ook de toegang tot diens sleutels kwijtraakt.

Guard is beschikbaar op de welkom- en ontgrendelschermen. Het reset geen lokaal
wachtwoord en herstelt geen willekeurig Bitcoin-, Ethereum- of gewoon
adrestegoed. Bij gelekte sleutels betreft identiteitsherstel alleen wat die
identiteit beheert. Controleer identiteit, bevoegdheden, netwerk en wijzigingen
voordat je goedkeurt.

<!-- help-article: private-verus -->

### Private Verus instellen

Private Verus gebruikt een afgeschermd adres en vereist eigen herstelgegevens en
synchronisatie.

Open Instellingen en vervolgens Private Verus. Afhankelijk van je wallet kun je
de primaire herstelzin hergebruiken, een nieuw privacyherstelgeheim aanmaken of
een bestaand geheim importeren. Bewaar een apart aangemaakt of geïmporteerd
geheim ook apart. De primaire walletback-up alleen herstelt dat privétegoed
mogelijk niet.

Laat de privésynchronisatie afronden voordat je verstuurt. Die zoekt op de chain
naar tegoed dat de privésleutels kunnen gebruiken. Een nieuwe installatie of
hersteld geheim kan tijd nodig hebben om het volledige saldo te vinden. Volg de
getoonde configuratie- of herstartaanwijzingen.

<!-- help-article: private-send -->

### Privéadressen en transactiememos

Een afgeschermd adres beschermt andere informatie dan een openbaar
ontvangstadres. Controleer beide kanten van de transactie.

Private Verus kan naar ondersteunde afgeschermde of transparante
Verus-bestemmingen versturen. Bij een openbaar adres is de ontvangende kant
on-chain zichtbaar. Een privébron maakt een openbare bestemming niet privé.

Privémemos worden ondersteund voor afgeschermde bestemmingen. De wallet moet
privésynchronisatie afronden en het transactiebewijs voorbereiden voordat zij
kan indienen. Het maken van dat bewijs kan tijd kosten. Controleer bij een
onzekere indiening de geschiedenis voordat je opnieuw verstuurt.

<!-- help-article: requests -->

### Een verzoek van een app controleren

Een verzoek vraagt de wallet bepaalde handelingen uit te voeren. Controleer de
aanvrager en elke handeling voordat je goedkeurt.

Gebruik Open request of open een ondersteunde walletlink. De wallet controleert
het verzoek en toont de handelingen die zij ondersteunt. Dit kunnen
authenticatie en identiteitswijzigingen zijn; sommige vereisen ook financiering
en kosten.

Een geldige handtekening identificeert de ondertekenaar, maar maakt het verzoek
niet automatisch wenselijk. Controleer zelf de identiteit, bevoegdheden,
bestemming en wijzigingen. Wijs onverwachte verzoeken af. Apps is momenteel geen
appcatalogus en Activiteit toont in deze versie geen transactiegeschiedenis;
bekijk daarvoor een munt.

<!-- help-category: data -->

## Walletgegevens en instellingen

<!-- help-article: contacts -->

### Contacten en openbare profielen

Contacten bewaren ontvangers en privénotities in deze wallet. Een gekoppeld
openbaar profiel komt van de VerusID.

Sla een contact met een ondersteund adres op of koppel een VerusID. Een openbaar
profiel kan helpen de identiteit te herkennen; je eigen notitie blijft lokaal.
Controleer bij elke betaling het gekozen ontvangstadres en netwerk.

Een contact verwijderen verwijdert de lokale vermelding. Het verwijdert geen
VerusID, openbaar profiel of blockchaintransacties. Contacten worden in deze
installatie versleuteld bewaard en komen niet terug door alleen de
wallet-herstelzin op een ander apparaat te importeren.

<!-- help-article: watchlist -->

### Wat de volglijst toont

De volglijst volgt openbaar tegoed van ondersteunde Verus-adressen en
identiteiten.

Een adres of identiteit toevoegen laat je de beschikbare openbare saldi
bekijken. Het importeert geen sleutel en geeft geen bestedingsrecht. Gevolgd
tegoed staat los van tegoed dat deze wallet beheert.

Resultaten hangen af van de ondersteunde netwerken en bereikbare gegevens. Een
onbeschikbaar of gedeeltelijk geladen resultaat bewijst geen nulsaldo.
Privésaldi kunnen niet via een openbare adreszoekopdracht worden gevonden. Een
gevolgde vermelding verwijderen wijzigt alleen je lokale lijst.

<!-- help-article: local-data -->

### Wat blijft op dit apparaat?

Sleutels en lokale walletgegevens hebben een ander doel dan openbare
blockchainrecords.

Contacten, privénotities, volglijstkeuzes en weergave-instellingen zijn lokale
walletgegevens. Een VerusID-profiel publiceren publiceert deze niet. Een
herstelgeheim importeren herstelt de bijbehorende sleutels, geen kopie van elke
lokale instelling of elk contact.

Openbare transacties en gepubliceerde profielgeschiedenis blijven op hun
netwerken. De wallet vergrendelen beschermt lokale toegang; het verbergt geen
informatie die al openbaar is. Bewaar de oorspronkelijke walletbestanden totdat
je een verhuizing naar een nieuwe installatie hebt gecontroleerd.

<!-- help-article: settings -->

### Weergave, taal en walletvergrendeling

Instellingen beheert weergave, fiatvaluta, taal, automatisch vergrendelen en
toegang tot herstelgegevens.

Kies Weergave en taal voor een thema, taal of fiatvaluta. Een andere
weergavevaluta wijzigt schattingen in de interface; het converteert je tegoed
niet.

Profiel en beveiliging bevat de vergrendeling bij inactiviteit en toegang tot
herstelgegevens. Vergrendelen beëindigt de ontgrendelde sessie; rond je werk
eerst af of verlaat het veilig. Private Verus heeft een eigen
configuratiepagina. Over en ondersteuning toont de appversie, die helpt bij het
melden van problemen.

<!-- help-article: support -->

### Hulp vragen aan de community

Beschrijf wat je probeerde, wat je verwachtte en welke melding de wallet toonde.

Vermeld de appversie, het netwerk en de munt. Een openbare transactie-ID kan
helpen bij een overboekingsprobleem, maar kan ook transactiegegevens onthullen.
Controleer screenshots voordat je ze deelt en verwijder privégegevens.

Kies Vraag het de community om Verus Discord te openen. Stuur nooit een
herstelzin, privésleutel, bestedingssleutel of wachtwoord naar iemand die hulp
aanbiedt. De Help-artikelen werken offline; communitylinks, actuele saldi en
netwerkzoekopdrachten vereisen een verbinding.

<!-- help-controls -->

## Help labels en meldingen

Je kunt ook de kolom Tekst hieronder aanpassen. Laat de kolom Referentie en de
plaatsaanduiding `{count}` ongewijzigd.

| Referentie                      | Tekst                                                   |
| ------------------------------- | ------------------------------------------------------- |
| `helpCenter.title`              | Help                                                    |
| `helpCenter.search`             | Zoeken in help                                          |
| `helpCenter.searchPlaceholder`  | Zoek een vraag of onderwerp                             |
| `helpCenter.home`               | Alle onderwerpen                                        |
| `helpCenter.suggested`          | Begin hier                                              |
| `helpCenter.topics`             | Helponderwerpen                                         |
| `helpCenter.backWallet`         | Terug naar wallet                                       |
| `helpCenter.backWelcome`        | Terug naar welkom                                       |
| `helpCenter.backUnlock`         | Terug naar inloggen                                     |
| `helpCenter.backResults`        | Terug naar resultaten                                   |
| `helpCenter.backArticle`        | Terug naar artikel                                      |
| `helpCenter.backTopics`         | Terug naar onderwerpen                                  |
| `helpCenter.results`            | Zoekresultaten                                          |
| `helpCenter.resultOne`          | 1 artikel                                               |
| `helpCenter.resultCount`        | {count} artikelen                                       |
| `helpCenter.noResults`          | Geen artikelen gevonden                                 |
| `helpCenter.noResultsHint`      | Probeer een kortere zoekopdracht of kies een onderwerp. |
| `helpCenter.related`            | Gerelateerde artikelen                                  |
| `helpCenter.community`          | Vraag het de community                                  |
| `helpCenter.communityHint`      | Opent Verus Discord                                     |
| `helpCenter.source.identity`    | Meer over VerusID                                       |
| `helpCenter.source.conversions` | Meer over Verus-conversies                              |
| `helpCenter.source.bridge`      | Meer over de Verus-Ethereum Bridge                      |
| `help.link.needHelp`            | Hulp krijgen                                            |
| `common.clearSearch`            | Zoekopdracht wissen                                     |
