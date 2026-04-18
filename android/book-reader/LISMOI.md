# book-reader

Application Android pour lire des livres physiques a voix haute. Pointez la camera de votre telephone vers une page, capturez une image, tapez sur les blocs de texte que vous voulez entendre, et l'application les lit a voix haute grace a la synthese vocale. Supporte le francais et l'anglais avec un changement de langue en un seul tap.

Tout le traitement se fait sur l'appareil -- aucune connexion internet requise. L'OCR utilise le modele latin integre de ML Kit, et la synthese vocale utilise le moteur TTS natif d'Android.

## Plateforme

Android 8.0+ (API 26+), compile sur Linux ou macOS

## Prerequis

Pour compiler depuis les sources :

- **Android Studio** (Hedgehog 2023.1 ou plus recent) ou les outils en ligne de commande du Android SDK
- **JDK 17+**

Pour installer l'APK pre-compile :

- Un telephone Android sous Android 8.0 ou plus recent

## APK pre-compile

Un APK pret a installer est inclus dans le depot :

```
book-reader/app-debug.apk
```

Voir la section [Installation](#installation) ci-dessous pour le transferer sur votre telephone.

## Compilation

### Avec Android Studio

1. **File > Open** et selectionnez le repertoire `book-reader/`
2. Attendez la fin de la synchronisation Gradle
3. Cliquez sur **Build > Build Bundle(s) / APK(s) > Build APK(s)**

### En ligne de commande

```bash
# Ajustez ces chemins selon votre systeme
export ANDROID_HOME=$HOME/Android/Sdk
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk

cd book-reader
./gradlew assembleDebug
```

L'APK est genere dans :

```
app/build/outputs/apk/debug/app-debug.apk
```

## Installation

### Via USB (avec ADB)

1. Activez les Options pour les developpeurs sur votre telephone : **Parametres > A propos du telephone > tapez 7 fois sur "Numero de build"**
2. Activez le **Debogage USB** dans **Parametres > Options pour les developpeurs**
3. Connectez le telephone par USB et executez :

```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

### Transfert direct de l'APK

1. Copiez `app-debug.apk` sur votre telephone (transfert USB, email, stockage cloud, Bluetooth, etc.)
2. Sur le telephone, ouvrez le fichier et autorisez l'installation depuis des sources inconnues lorsque demande
3. Tapez **Installer**
4. Lancez **BookReader** depuis le tiroir d'applications

## Utilisation

### Fonctionnement

1. Accordez la permission camera au premier lancement
2. Choisissez la langue avec le bouton **FR/EN** en haut a droite
3. Pointez la camera vers une page de livre
4. Tapez **Capturer** pour figer l'image
5. Les zones de texte detectees apparaissent surlignees en bleu
6. Tapez sur les zones que vous voulez lire -- elles passent en orange quand elles sont selectionnees
7. Tapez **Lire la selection** -- le texte est lu a voix haute
8. Tapez **Effacer** pour revenir a la camera en direct

### Reglages

Tapez l'icone **engrenage** en haut a droite (a gauche du bouton FR/EN) pour ouvrir l'ecran des reglages.

- **Vitesse de lecture** -- curseur horizontal de 0.5x a 2.0x (pas de 0.1, defaut 1.0x). Le changement s'applique a la prochaine phrase lue ; la lecture en cours n'est pas interrompue.
- **Modeles vocaux** -- emplacement reserve pour de futurs telechargements de voix (actuellement desactive avec "Bientot disponible").

La vitesse est persistee via Jetpack DataStore (Preferences) sous la cle `speech_rate`. Une nouvelle installation reinitialise a 1.0x.

### Changement de langue

Le bouton **FR/EN** en haut a droite bascule entre le francais et l'anglais pour la reconnaissance OCR et la voix de synthese vocale. La langue par defaut est le francais.

### Installation des voix TTS

L'application utilise le moteur de synthese vocale integre a Android. Si la voix pour la langue selectionnee n'est pas installee, un bandeau apparait avec un bouton **Installer** qui ouvre les parametres TTS de votre appareil.

Pour installer les voix manuellement :

#### Telephones Samsung (Galaxy S, A, etc.)

1. **Parametres > Accessibilite > Synthese vocale** (ou Parametres > Gestion generale > Synthese vocale)
2. Verifiez le moteur selectionne (Samsung TTS ou Google TTS)
3. Tapez sur l'icone **engrenage** a cote du moteur
4. **Langue** > telechargez **Francais (France)** et/ou **English (United States)**

> **Astuce Samsung** : si la qualite est mauvaise avec Samsung TTS, installez **Google TTS** depuis le Play Store et selectionnez-le comme moteur par defaut.

#### Telephones Google Pixel

1. **Parametres > Systeme > Langues et saisie > Synthese vocale**
2. Tapez sur l'icone **engrenage** a cote de Google Speech Services
3. **Installer les donnees vocales** > telechargez **Francais** et **English**

#### Telephones Xiaomi / Redmi / POCO

1. **Parametres > Parametres supplementaires > Accessibilite > Synthese vocale**
2. Tapez sur l'icone **engrenage** a cote du moteur (souvent Google TTS)
3. **Installer les donnees vocales** > telechargez **Francais** et **English**

> Si Google TTS n'apparait pas, installez-le depuis le Play Store : recherchez "Google Synthese vocale".

#### Methode universelle (tous telephones)

1. Ouvrez le **Play Store**
2. Recherchez **"Google Synthese vocale"** (Google Text-to-Speech)
3. Installez ou mettez a jour
4. Retournez dans **Parametres** et cherchez **"Synthese vocale"** dans la barre de recherche
5. Selectionnez **Google** comme moteur > **engrenage** > telechargez les voix

#### Verifier que la voix fonctionne

Dans les parametres de synthese vocale, il y a un bouton **"Ecouter un exemple"** (ou "Play"). Tapez dessus pour verifier que la voix fonctionne correctement.

## Architecture

```
app/src/main/java/com/example/bookreader/
  MainActivity.kt           Activity : configuration camera, boutons, observation de l'etat
  SettingsActivity.kt       Activity : curseur vitesse + emplacement modeles vocaux
  CameraViewModel.kt        ViewModel : machine a etats + sync vitesse de lecture
  OcrProcessor.kt           OCR sur l'appareil via ML Kit Text Recognition (modele latin integre)
  TtsManager.kt             Wrapper TTS avec StateFlow, locale et vitesse de lecture
  HighlightOverlayView.kt   Vue personnalisee : overlay des bounding boxes, tap pour selectionner
  data/
    SettingsDataStore.kt    Jetpack DataStore (Preferences) : cle speech_rate (Float)
  model/
    CameraState.kt          Interface scellee : LivePreview | Processing | Frozen
    TtsState.kt             Interface scellee : Initializing | Ready | Speaking | MissingVoice | Error
    TextRegion.kt           Data class : bloc de texte detecte avec bounding box et etat de selection

app/src/main/res/
  layout/activity_main.xml      ConstraintLayout : camera, image figee, overlay, boutons, engrenage
  layout/activity_settings.xml  Ecran reglages : toolbar, curseur, carte modeles vocaux
  values/strings.xml            Chaines de l'interface (en francais par defaut)
```

### Pipeline

```
Camera (CameraX) --> Capture image --> Rotation a l'endroit (Matrix.postRotate)
  --> ML Kit Text Recognition (sur l'appareil, modele integre)
  --> List<TextRegion> triee en ordre de lecture (haut en bas, gauche a droite)
  --> Affichage de l'image figee + overlay des bounding boxes
  --> L'utilisateur tape pour selectionner les regions
  --> Concatenation du texte selectionne --> Moteur TTS Android --> Sortie audio
```

### Dependances principales

| Bibliotheque | Role |
|-------------|------|
| CameraX 1.3 | Apercu camera et capture d'image |
| ML Kit Text Recognition 16.0 | OCR sur l'appareil (modele latin integre, pas de reseau necessaire) |
| Android TextToSpeech | Synthese vocale native |
| Material 3 | Composants d'interface et theme |
| Jetpack DataStore Preferences 1.1 | Persistance de la vitesse de lecture |

## Limitations connues

- **Orientation portrait uniquement.** L'application est verrouillee en mode portrait pour simplifier la correspondance des coordonnees entre l'image camera et l'overlay.
- **Alphabet latin uniquement.** Le modele latin integre de ML Kit gere le francais, l'anglais et les autres langues a alphabet latin. Il ne supporte pas le CJK, l'arabe, le devanagari, etc.
- **L'eclairage est important.** La precision de l'OCR diminue avec un mauvais eclairage, des reflets ou des pages inclinees. Tenez le telephone parallele a la page pour de meilleurs resultats.
- **Pas de scan continu.** Chaque capture est une image unique -- il n'y a pas de mode de detection automatique ou de lecture continue.
- **La qualite TTS depend de l'appareil.** Certains fabricants livrent des moteurs TTS de moindre qualite. Installer Google TTS depuis le Play Store donne generalement les meilleurs resultats.
- **Android 8.0 minimum** (API 26) requis.

## Depannage

| Probleme | Solution |
|----------|----------|
| "Sources inconnues" introuvable | Cherchez "installer applis inconnues" dans les parametres |
| L'app ne s'installe pas | Verifiez qu'Android 8.0+ est installe (Parametres > A propos) |
| L'OCR ne detecte rien | Eclairez bien la page, rapprochez la camera, evitez les reflets |
| La voix parle dans la mauvaise langue | Verifiez le bouton FR/EN en haut a droite et la voix TTS installee |
| Pas de son | Verifiez le volume media (pas le volume sonnerie) |
| "Voix non disponible" | Suivez la section "Installation des voix TTS" ci-dessus |
| Crash au lancement | Desinstallez puis reinstallez l'APK |

## Licence

MIT -- voir [LICENSE](../../LICENSE)
