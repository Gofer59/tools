# gamebook-digitize

Convertissez la vidéo d'un feuilletage de livre-jeu physique en :

1. **Source markdown** — modifiable, avec des sections numérotées `## § N` et des références d'images
2. **Lecteur HTML autonome** — thème sombre, barre latérale avec fiche de personnage, navigation entre sections, renvois cliquables

Conçu pour les "Livres dont vous êtes le héros" français et les livres Fighting Fantasy / Choose Your Own Adventure anglais.

## Installation

```bash
cd tools/gamebook-digitize
chmod +x install.sh && ./install.sh
```

Cette commande crée un environnement virtuel Python à `~/.local/share/gamebook-digitize/venv/` et un lanceur à `~/.local/bin/gamebook-digitize`.

**Surya OCR** (moteur par défaut) télécharge ~1-2 Go de poids de modèles au premier lancement, mis en cache dans `~/.cache/huggingface/`.

**Tesseract** (repli optionnel) nécessite une installation séparée :
```bash
# Debian/Ubuntu
sudo apt-get install -y tesseract-ocr tesseract-ocr-fra tesseract-ocr-eng

# Arch/SteamOS
sudo pacman -S tesseract tesseract-data-fra tesseract-data-eng
```

## Utilisation

### Pipeline complète — vidéo vers livre-jeu

```bash
# Livre-jeu français, 3 pages de référence (couverture, fiche personnage, équipement)
gamebook-digitize input.mp4 --lang fr --ref-pages 3

# Livre anglais, répertoire de sortie personnalisé
gamebook-digitize input.mp4 --lang en --ref-pages 5 --output ./mon-livre/

# Ignorer le nettoyage LLM (plus rapide, pas besoin de la CLI Claude)
gamebook-digitize input.mp4 --lang fr --ref-pages 3 --no-llm

# Utiliser Tesseract au lieu de Surya
gamebook-digitize input.mp4 --lang fr --ref-pages 3 --ocr-engine tesseract

# Paramètres de réglage
gamebook-digitize input.mp4 --lang fr --ref-pages 3 \
  --frame-interval 0.3 \
  --sharpness-threshold 80 \
  --hash-threshold 6

# Debug : conserver les images extraites, sortie détaillée
gamebook-digitize input.mp4 --lang fr --ref-pages 3 --keep-frames --verbose
```

### Régénérer le HTML depuis un markdown modifié

Après avoir modifié `sections.md` pour corriger des erreurs OCR ou ajuster le texte des sections :

```bash
gamebook-digitize --from-markdown mon-livre/sections.md
```

Cette commande régénère `player.html` sans retraiter la vidéo.

## Sortie

```
mon-livre/
├── sections.md    # Source markdown modifiable
├── player.html    # Lecteur HTML de livre-jeu autonome
├── images/        # Images extraites
│   ├── ref_001.jpg        # Pages de référence (barre latérale)
│   ├── sec001_fig1.jpg    # Illustrations de sections (en ligne)
│   └── ...
└── frames/        # (uniquement avec --keep-frames) Images de pages sélectionnées
```

## Étapes de la pipeline

| Étape | Description |
|-------|-------------|
| 1 | **Extraction d'images** — OpenCV extrait des images à intervalle configurable |
| 2 | **Sélection de la meilleure image** — netteté Laplacienne + différenciation d'images sélectionne une image par page stable |
| 3 | **Déduplication des pages** — hachage perceptuel supprime les doublons (consécutifs + global) |
| 4 | **OCR** — Surya (par défaut) ou Tesseract extrait le texte avec les boîtes englobantes |
| 5 | **Découpage en sections** — détecte les grands numéros de sections (§ N) par taille de police, divise le texte en sections |
| 6 | **Extraction d'images** — recadre les illustrations des pages, les associe à la section la plus proche |
| 7 | **Nettoyage LLM** — la CLI Claude corrige les artéfacts OCR, restaure les accents et ligatures français |
| 8 | **Assemblage markdown** — écrit un markdown structuré avec frontmatter, références et sections |
| 9 | **Génération HTML** — produit un lecteur de livre-jeu à thème sombre autonome |

## Options CLI

| Option | Par défaut | Description |
|--------|-----------|-------------|
| `input` | requis | Chemin vers le fichier vidéo |
| `--from-markdown CHEMIN` | — | Ignorer la pipeline vidéo, générer le HTML depuis un markdown existant |
| `-l, --lang` | `fr` | Langue du livre : `fr` ou `en` |
| `--ref-pages N` | `0` | Nombre de pages initiales comme matériel de référence (barre latérale) |
| `-o, --output DIR` | `./<nom-entrée>/` | Répertoire de sortie |
| `--ocr-engine` | `surya` | Moteur OCR : `surya` ou `tesseract` |
| `--no-llm` | désactivé | Ignorer la passe de nettoyage de la CLI Claude |
| `--frame-interval` | `0.5` | Secondes entre les images extraites |
| `--sharpness-threshold` | `50.0` | Variance Laplacienne en dessous de laquelle = flou |
| `--hash-threshold` | `8` | Distance de Hamming maximale pour la déduplication "même page" |
| `--keep-frames` | désactivé | Sauvegarder les images de pages sélectionnées dans `sortie/frames/` |
| `--title TEXTE` | auto | Titre du livre pour le frontmatter markdown et le HTML |
| `-v, --verbose` | désactivé | Progression détaillée sur stderr |

## Fonctionnalités du lecteur HTML

- **Barre latérale gauche** (repliable) : images de pages de référence (fiche personnage, tables d'équipement)
- **Barre de navigation supérieure** (fixe, défilable) : tous les numéros de §, section courante surlignée en or
- **Zone de lecture principale** : police serif, thème sombre, illustrations en ligne
- **Renvois** : "rendez-vous au 147" / "go to 32" deviennent des liens cliquables
- **Autonome** : fichier HTML unique avec toutes les images en base64, sans dépendances externes
- **Tout le texte est sélectionnable** : fonctionne avec les outils TTS (voice-speak, etc.)

## Format Markdown

Le fichier `sections.md` généré utilise ce format :

```markdown
---
title: Le Sorcier de la Montagne de Feu
lang: fr
ref_pages: 3
---

<!-- REF: Page 1 -->
![Page de référence 1](images/ref_001.jpg)

---

## § 1

Vous vous trouvez à l'entrée d'un sombre donjon...

Si vous voulez entrer, rendez-vous au 45.
Si vous préférez fuir, allez au 278.

---

## § 2

Le couloir mène à une salle immense...
```

## Nettoyage LLM

L'étape 7 appelle la CLI `claude` pour corriger les artéfacts OCR. Pour les livres en français, elle restaure spécifiquement :
- Accents : é, è, ê, ë, à, â, ç, ù, û, î, ï, ô
- Ligatures : oeuvre → œuvre, coeur → cœur, soeur → sœur

La passe LLM est optionnelle (`--no-llm` pour l'ignorer) et se dégrade gracieusement si `claude` n'est pas installé.

## Dépendances

- Python 3.10+
- OpenCV (extraction d'images, traitement d'images)
- Pillow + ImageHash (hachage perceptuel)
- NumPy
- Surya OCR (moteur OCR par défaut)
- pytesseract (OCR de repli optionnel)
- CLI `claude` (optionnel, pour le nettoyage LLM)
