# Pipeline de Linting de la Documentation (Vale CLI)

Ce guide décrit l'intégration de **Vale CLI** dans notre infrastructure afin de valider automatiquement l'absence de délimiteurs LaTeX/MathJax dans notre documentation Markdown et de prévenir les régressions de rendu au sein du livre compilé par `mdBook`.

---

## 🎯 1. Pourquoi ce Linter ?

Le serveur de documentation local (`mdBook`) ne gère pas nativement les équations LaTeX (telles que `$ ... $`, `$$ ... $$` ou `\( ... \)`). Conformément à la **Règle 7** du projet, les formules doivent être rédigées en texte clair et caractères Unicode lisibles (ex: `48 kHz` au lieu de `\(48\text{ kHz}\)`, `Δt` au lieu de `$$\Delta t$$`).

Le linter Vale valide automatiquement l'ensemble des fichiers Markdown pour s'assurer qu'aucun délimiteur LaTeX ne soit introduit par inadvertance.

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

### Règle personnalisée (`doc/styles/Firework/NoLatexMath.yml`)
Une règle de détection basée sur des expressions régulières (Regex) analyse le contenu des fichiers pour interdire l'utilisation des délimiteurs LaTeX (`$`, `$$`, `\(`, `\[`).

---

## 🚀 3. Utilisation & Automatisation

Le linter s'exécute de deux façons :

### A. Tâche Taskfile (Vérification Globale)
* **Installation à la volée :** Si `vale` n'est pas installé sur l'OS, la tâche `task doc:setup-vale` télécharge automatiquement le binaire officiel Linux 64-bit et l'installe localement dans `./bin/vale`.
* **Lancement du Linter :**
  ```bash
  task doc:lint
  ```
* **Chaînage de Qualité :** La commande globale `task lint:all` intègre automatiquement la vérification des documents en plus de `cargo fmt` et `cargo clippy`.

### B. Crochet Git de validation (Git Pre-Commit Hook)
Un script de crochet a été déployé sous `.git/hooks/pre-commit`.
* À chaque commande `git commit`, le hook identifie uniquement les fichiers Markdown `.md` présents dans l'index de validation (staged changes).
* Il lance Vale sur ces fichiers ciblés.
* Si Vale remonte une erreur, le commit est avorté avec un rapport listant la ligne et l'erreur exacte, garantissant qu'aucune formule mathématique brisée ne soit enregistrée dans l'historique de version.
