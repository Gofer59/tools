# book-digitize

Outil en ligne de commande qui prend un enregistrement vidéo d'un livre que l'on feuillette et produit un fichier **markdown**, **texte brut** ou **PDF** propre et ordonné — prêt pour la lecture ou la synthèse vocale.

## Fonctionnement

```
input.mp4
    |
    v
+----------------------+
| Étape 1 : Extraire   |  OpenCV VideoCapture -> images JPEG toutes les N secondes
|   les images         |
+----------+-----------+
           v
+----------------------+
| Étape 1.5 : Préparer |  Détecter les pages, diviser les doubles pages,
|                      |  correction de perspective, amélioration
+----------+-----------+
           v
+----------------------+
| Étape 2 : Évaluer &  |  Variance Laplacienne (netteté) + diff image à image
|  Sélectionner les    |  -> grouper en segments stables, choisir la plus nette
|  meilleures images   |  -> écarter les images de transition/floues (tournages de pages)
+----------+-----------+
           v
+----------------------+
| Étape 3 : Dédupliquer|  Hash perceptuel (phash) -- dédup consécutive + globale
|  & Ordonner les pages|  -> une image par page unique
|                      |  -> OCR en-tête/pied de page pour les numéros
|                      |  -> valider l'ordre, signaler les pages manquantes
+----------+-----------+
           v
+----------------------+
| Étape 4.5 : Images   |  (optionnel) Détecter & extraire les images intégrées
|  (--extract-images)  |  -> détection de mise en page Surya ou contour OpenCV
|                      |  -> recadrer, sauvegarder, détecter les légendes
+----------+-----------+
           v
+----------------------------+
| Étape 4.75 : Claude Vision |  (optionnel, --claude-layout)
|                            |  -> envoyer chaque page à Claude Vision API
|                            |  -> mise en page structurée : titres, paragraphes, images
|                            |  -> remplace le texte OCR par l'extraction de Claude
+----------+-----------------+
           v
+----------------------+
| Étape 5 : OCR        |  Surya (par défaut, plus précis) ou Tesseract
|                      |  -> markdown : détecte les titres
|                      |  -> préserve la structure des paragraphes
|                      |  -> intercale les références d'images
+----------+-----------+
           v
+----------------------+
| Étape 6 : Assembler  |  -> book.md / book.txt / book.pdf
|   la sortie          |  -> journal de résumé (nombre de pages, lacunes, faible confiance)
+----------------------+
```

## Installation

```bash
cd /chemin/vers/book-digitize
chmod +x install.sh
./install.sh
```

L'installateur va :
1. Vérifier `python3` et `tesseract`
2. Installer les paquets de langue Tesseract (`eng`, `fra`) si manquants
3. Créer un environnement virtuel Python isolé à `~/.local/share/book-digitize/venv/`
4. Installer les dépendances Python (OpenCV, pytesseract, Pillow, ImageHash, numpy, surya-ocr, fpdf2, anthropic)
5. Installer la commande `book-digitize` dans `~/.local/bin/`

**Note :** Surya OCR télécharge ~1-2 Go de poids de modèles au premier lancement (mis en cache dans `~/.cache/huggingface/`).

### Configuration requise

- **Linux** (Debian/Ubuntu/Mint ou basé sur Arch)
- **Python 3.10+**
- **Tesseract 5.x** (`sudo apt install tesseract-ocr`)
- Pour la meilleure sortie PDF : une police serif TrueType (DejaVu Serif, Liberation Serif ou Noto Serif). La plupart des distributions en incluent une par défaut.

## Utilisation

```bash
# Basique -- texte français, sortie markdown (OCR Surya)
book-digitize input.mp4

# Livre anglais, chemin de sortie personnalisé
book-digitize input.mp4 --output book.md --lang en

# Sortie texte brut (sans détection de titres)
book-digitize input.mp4 --format txt --output book.txt

# Sortie PDF (active automatiquement l'extraction d'images)
book-digitize input.mp4 --format pdf --output book.pdf

# PDF avec analyse de mise en page Claude Vision (meilleure qualité, utilise la CLI claude)
book-digitize input.mp4 --format pdf --claude-layout

# PDF avec Claude Vision, limité à 10 appels
book-digitize input.mp4 --format pdf --claude-layout --max-claude-calls 10

# Extraire les images intégrées (photos, diagrammes)
book-digitize input.mp4 --extract-images

# Utiliser Tesseract au lieu de Surya
book-digitize input.mp4 --ocr-engine tesseract --lang fra

# Les deux langues (détection auto par page)
book-digitize input.mp4 --lang fr+en

# Debug : sauvegarder les images de pages sélectionnées
book-digitize input.mp4 --keep-frames --verbose
```

### Options

| Option | Par défaut | Description |
|--------|-----------|-------------|
| `input` (positionnel) | requis | Chemin vers le fichier vidéo (MP4, MKV, AVI) |
| `-o, --output` | `<entrée>.md` | Chemin du fichier de sortie |
| `-f, --format` | `md` | Format de sortie : `md` (markdown), `txt` (texte brut) ou `pdf` (livre numérique) |
| `-l, --lang` | `fr` | Langue : `fr`, `en`, `fr+en` (Surya) ou `fra`, `eng` (Tesseract) |
| `--ocr-engine` | `surya` | Moteur OCR : `surya` (plus précis, plus lent sur CPU) ou `tesseract` |
| `--no-preprocess` | désactivé | Ignorer la détection/division/amélioration des pages (utiliser les images brutes) |
| `--extract-images` | désactivé | Détecter et extraire les images intégrées dans le répertoire `images/`. Activé automatiquement pour `--format pdf`. |
| `--claude-layout` | désactivé | Utiliser Claude Vision pour l'analyse de mise en page (nécessite la CLI `claude` connectée) |
| `--pdf-margin` | `2.0` | Marge PDF en cm |
| `--pdf-font` | auto-détection | Chemin vers un fichier de police TTF pour le corps du PDF |
| `--max-claude-calls` | `0` (illimité) | Limite du budget d'appels API Claude |
| `--frame-interval` | `0.5` | Secondes entre les images extraites (accepte `0.5s`, `0.5sec` ou `0.5`) |
| `--sharpness-threshold` | `50.0` | Variance Laplacienne en dessous de laquelle = flou |
| `--diff-threshold` | `30.0` | Diff pixel entre images au-dessus duquel = transition de page |
| `--hash-threshold` | `8` | Distance de Hamming maximale pour considérer "même page" |
| `--page-crop-ratio` | `0.08` | Fraction de la hauteur de page pour chercher les numéros |
| `--keep-frames` | désactivé | Sauvegarder les images de pages sélectionnées dans `./frames/` |
| `--log` | désactivé | Écrire le journal de résumé dans un fichier (en plus de stderr) |
| `-v, --verbose` | désactivé | Progression détaillée sur stderr |

## Formats de sortie

### Markdown (`--format md`)

Détecte les titres en analysant les hauteurs des boîtes englobantes des mots. Les lignes significativement plus hautes que le corps du texte deviennent des titres `#` ou `##`. Les images intégrées apparaissent comme `![légende](images/nom.jpg)`.

```markdown
---

**Page 1**

# AVANT-PROPOS

La question d'une suite se pose depuis plus de 5 ans au moment
où j'écris ces lignes.

![Figure 1: Diagramme](images/page_1p1_fig1.jpg)

---

**Page 2**

Le succès du premier livre a été une surprise...
```

### Texte brut (`--format txt`)

```
--- Page 1 ---
AVANT-PROPOS

La question d'une suite se pose depuis plus de 5 ans...

--- Page 2 ---
Le succès du premier livre...
```

### PDF (`--format pdf`)

Produit un PDF multi-pages où chaque page du livre devient une page PDF :
- **Texte** : Police serif (DejaVu Serif par défaut), corps 11pt, H1 18pt, H2 14pt
- **Images** : Placées à la position verticale détectée, mises à l'échelle sur la largeur de page
- **Légendes** : Italique 9pt sous chaque image
- **Numéros de page** : En bas au centre de chaque page
- **Taille de page** : Correspond au ratio d'aspect de l'image capturée
- **Gestion du débordement** : La police se réduit automatiquement (minimum 6pt) si le contenu dépasse la hauteur de page

L'extraction d'images est activée automatiquement pour la sortie PDF.

### Mise en page Claude Vision (`--claude-layout`)

Quand cette option est activée, chaque image de page est envoyée à Claude Sonnet via l'API Anthropic pour une analyse structurée de la mise en page. Claude identifie les titres, paragraphes, images et légendes avec des positions verticales précises — produisant une sortie PDF de meilleure qualité qu'avec l'OCR seul.

Prérequis :
- La CLI `claude` doit être installée et connectée (utilise votre abonnement Claude Max)
- Les réponses sont mises en cache à côté du fichier de sortie (`.claude_cache.json`) pour que les relances ne répètent pas les appels
- Réessaie avec backoff exponentiel en cas d'échec
- Utilisez `--max-claude-calls` pour limiter l'utilisation

## Résumé de sortie

Un résumé est affiché sur stderr après chaque exécution :

```
[book-digitize] Summary
[book-digitize]   Total pages extracted: 32
[book-digitize]   Pages with detected numbers: 28/32
[book-digitize]   OCR engine: surya
[book-digitize]   Images extracted: 5
[book-digitize]   WARNING: Missing pages: 6, 7 (between 5 and 8)
[book-digitize]   Low-confidence pages: page 3 (45%), page 15 (52%)
[book-digitize]   Output written to: book.md
```

## Conseils de réglage

- **Images floues encore sélectionnées ?** Augmentez `--sharpness-threshold` (essayez 80-100).
- **Trop peu de pages détectées ?** Diminuez `--sharpness-threshold` (essayez 20-30) ou réduisez `--frame-interval` à 0,3.
- **Pages en double dans la sortie ?** Diminuez `--hash-threshold` (essayez 4-6).
- **Pages fusionnées ensemble ?** Augmentez `--hash-threshold` (essayez 10-12).
- **Feuilletage rapide ?** Réduisez `--frame-interval` à 0,2-0,3s.
- **Numéros de page non détectés ?** Augmentez `--page-crop-ratio` (essayez 0,12-0,15) si les numéros sont loin du bord.
- **Fausses transitions de page ?** Augmentez `--diff-threshold` (essayez 40-50) si l'éclairage vacille et cause de fausses divisions.
- **Voulez un journal persistant ?** Utilisez `--log run.log` pour sauvegarder le résumé à côté de la sortie stderr.
- **Pas besoin de titres ?** Utilisez `--format txt` pour une sortie en texte brut.
- **OCR trop lent sur CPU ?** Utilisez `--ocr-engine tesseract` pour des résultats plus rapides (mais moins précis).
- **Texte PDF trop petit ?** Réduisez le contenu par page ou augmentez légèrement `--pdf-margin`.
- **Accents manquants dans le PDF ?** Installez une police serif Unicode : `sudo apt install fonts-dejavu`.

## Dépendances

| Bibliothèque | Rôle |
|-------------|------|
| `opencv-python-headless` | Extraction d'images vidéo, évaluation de la netteté, différenciation d'images |
| `pytesseract` | Wrapper Python pour Tesseract OCR |
| `Pillow` | Chargement et recadrage d'images |
| `ImageHash` | Hachage perceptuel pour la déduplication de pages |
| `numpy` | Opérations numériques (utilisé par OpenCV et ImageHash) |
| `surya-ocr` | Moteur OCR avancé avec détection de mise en page (par défaut, plus précis que Tesseract) |
| `fpdf2` | Génération de PDF (Python pur, sans dépendances système) |
| CLI `claude` | CLI Claude Code pour l'analyse de mise en page par vision (pas un paquet pip — installez séparément) |

## Limitations connues

- **Doubles pages** : La division automatique des pages fonctionne mais peut ne pas être parfaite pour toutes les mises en page.
- **Pages courbées** : Le texte près de la reliure peut être déformé par la courbure de la page. Aucun redressement n'est appliqué.
- **Texte multi-colonnes** : Tesseract `--psm 6` suppose une seule colonne. Surya gère mieux les colonnes.
- **Détection gras/italique** : Les moteurs OCR ne détectent pas de façon fiable les styles de police. Seule la *taille* (titres) est détectée.
- **Annotations manuscrites** : L'OCR est optimisé pour le texte imprimé uniquement.
- **Numéros de page en chiffres romains** : Non détectés (chiffres arabes uniquement).
- **Grandes vidéos** : Une vidéo de 2 heures à intervalles de 0,5s produit ~14 400 images (~7 Go d'espace disque temporaire). Un avertissement d'espace disque est affiché si l'espace libre est faible.
- **Utilisation de Claude Vision** : Chaque page correspond à un appel de la CLI `claude`. Un livre de 300 pages = 300 appels. Utilisez `--max-claude-calls` pour limiter.
