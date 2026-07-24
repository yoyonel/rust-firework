# Pipeline de Linting de la Documentation (Vale CLI)

Ce guide décrit l'intégration de **Vale CLI** dans notre infrastructure afin de valider automatiquement la syntaxe LaTeX/MathJax de notre documentation et de prévenir les régressions de rendu au sein du livre compilé par `mdBook`.

---

## 🎯 1. Pourquoi ce Linter ?

`mdBook` s'appuie sur le moteur client MathJax pour le rendu d'équations. Cependant, le compilateur Markdown (côté serveur) traite le document avant MathJax. Deux conflits majeurs surviennent fréquemment :
1. **Les délimiteurs simples (`$`) :** Le symbole dollar simple est souvent confondu avec le texte normal ou n'est pas reconnu par défaut sous mdBook, provoquant l'affichage de la formule brute. Les délimiteurs explicites `\\( ... \\)` (inline) et `\\[ ... \\]` ou `$$ ... $$` (block) doivent être préférés.
2. **Les underscores d'indice (`_`) :** Si une formule contient un caractère de soulignement non échappé (par exemple : `T_{visuel}`), le parseur Markdown le convertit en balise italique HTML `T_{<em>visuel</em>}`. La formule reçue par MathJax est alors brisée. Tout underscore dans une équation doit être échappé sous la forme `\\_`.

---

## ⚙️ 2. Configuration & Structure du Projet

L'outil est configuré pour s'intégrer de manière transparente et sans dépendances système globales préalables.

### Fichier `.vale.ini`
Situé à la racine du projet, il associe l'extension `.md` au format Markdown et active notre style personnalisé :
```ini
StylesPath = doc/styles
MinAlertLevel = error

[formats]
md = markdown

[*]
BasedOnStyles = Firework
```

### Règles personnalisées (`doc/styles/Firework/`)
Quatre règles de détection basées sur des expressions régulières (Regex) analysent le contenu brut (hors blocs de code) des fichiers :
* **`SingleDollarMath.yml` :** Recherche toute occurrence de simple dollar `$expression$` pour imposer l'utilisation de la syntaxe parenthésée `\\( expression \\)`.
* **`UnescapedUnderscoreBlockMath.yml` :** Valide les blocs `$$ ... $$` pour s'assurer qu'aucun soulignement non échappé `_` ne s'y trouve.
* **`UnescapedUnderscoreInlineMath.yml` :** Valide les expressions inline `\\( ... \\)` de la même façon.
* **`UnescapedUnderscoreBracketMath.yml` :** Valide la syntaxe crochet `\\[ ... \\]`.

---

## 🚀 3. Utilisation & Automatisation

Le linter s'exécute de deux façons :

### A. Tâche Taskfile (Vérification Globale)
* **Installation à la volée :** Si `vale` n'est pas installé sur l'OS, la tâche `task doc-setup-vale` télécharge automatiquement le binaire officiel Linux 64-bit et l'installe localement dans `./bin/vale`.
* **Lancement du Linter :**
  ```bash
  task doc-lint
  ```
* **Chaînage de Qualité :** La commande globale `task lint` intègre automatiquement la vérification des documents en plus de `cargo fmt` et `cargo clippy`.

### B. Crochet Git de validation (Git Pre-Commit Hook)
Un script de crochet a été déployé sous `.git/hooks/pre-commit`.
* À chaque commande `git commit`, le hook identifie uniquement les fichiers Markdown `.md` présents dans l'index de validation (staged changes).
* Il lance Vale sur ces fichiers ciblés.
* Si Vale remonte une erreur, le commit est avorté avec un rapport listant la ligne et l'erreur exacte, garantissant qu'aucune formule mathématique brisée ne soit enregistrée dans l'historique de version.
