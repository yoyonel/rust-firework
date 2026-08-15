# Gestion des Compétences IA (Agent Skills) : Architecture & Workflow

**Date :** 18 Juillet 2026  
**Auteur :** Équipe d'Ingénierie Rust Firework  
**Mots-clés :** IA, Antigravity, agy, Repomix, Taskfile, LLM, Workflow, Architecture  
**Statut :** Spécification active  

---

## 1. Contexte & Problématique

Le projet **Rust Firework** associe des contraintes d'exécution très strictes :
- Rendu graphique bas niveau hautes performances (**OpenGL AZDO** : *Approaching Zero Driver Overhead*).
- Traitement audio temps réel sans allocation (**CPal**, DSP, audio binaural, Doppler).
- Simulation physique intensive (**Generational Arena**, *Data-Oriented Design*, *Static AoS*).
- Interface utilisateur réactive en mode immédiat (**ImGui**, consoles de diagnostic).

L'analyse complète du dépôt représente plus de **250 000 tokens**. Soumettre l'intégralité de cette base à un agent IA dans une seule conversation dilue son attention et provoque des hallucinations critiques (ex: proposition de `Mutex` bloquants dans le thread audio ou d'appels OpenGL synchronisants).

Pour garantir un code IA 100% conforme à nos règles d'ingénierie, nous utilisons une architecture modulaire de **Compétences Métiers (Agent Skills)** standardisées (format `SKILL.md`) et chargées dynamiquement par notre client CLI.

---

## 2. Le Pattern Architectural : "Build & Overlay"

La génération des skills repose sur une séparation stricte entre le **contexte de code** (dynamique et changeant) et les **invariants d'ingénierie** (statiques et conçus par l'humain). Nous appliquons le pattern *Build & Overlay* :
```
+-------------------------------------------------------------+
| 1. BUILD : Repomix (Analyse AST & Extraction du Code source)|
+-------------------------------------------------------------+
|
v
[~/.agents/skills/firework-/references/summary|files.md]
|
+-------------------------------------------------------------+
| 2. OVERLAY : Remplacement par le SKILL.md Expert (Git)      |
+-------------------------------------------------------------+
|
v
[doc/ai_skills/firework-.md (Invariants)]
|
v
+-------------------------------------------------------------+
| 3. CONSUME : Routage Précis par Antigravity CLI (agy)       |
+-------------------------------------------------------------+
```

1. **Phase Build (`Repomix`) :** L'outil scanne les modules et sous-dossiers spécifiques du projet pour produire les références de code (`references/files.md`, `summary.md`). Cette partie pèse entre 40k et 50k tokens par domaine.
2. **Phase Overlay (`cp -f`) :** Le fichier de métadonnées générique (`SKILL.md`) produit par Repomix est écrasé et remplacé par un modèle expert stocké et versionné dans notre dépôt sous `doc/ai_skills/`. Ce fichier contient le *frontmatter* YAML indispensable au routage et rappelle nos "Règles d'Or" absolues (zéro-allocation, alignement `std140`, lock-free, etc.).

---

## 3. Segmentation des 4 Skills Métiers

L'arborescence est divisée en quatre compétences ultra-spécialisées, stockées localement dans `~/.agents/skills/` après compilation :

| Nom du Skill | Dossiers & Fichiers Cibles | Contraintes architecturales strictes (Invariants) |
|---|---|---|
| **`firework-audio`** | `src/audio_engine/**`<br>`doc/*audio*`, `doc/*doppler*` | Zéro allocation sur le tas dans le callback, structures lock-free (ring buffers), autovectorisation LLVM / SIMD. |
| **`firework-renderer`** | `src/renderer_engine/**`<br>`assets/shaders/**`<br>`doc/*azdo*`, `doc/*opengl*` | Buffers mappés persistants (Write-Combining), zéro appel GL synchronisant, alignement mémoire strict `#[repr(C)]` / `std140`. |
| **`firework-physic`** | `src/physic_engine/**`<br>`src/simulator.rs`<br>`doc/*physic*` | *Data-Oriented Design* (Static AoS/SoA), zéro fragmentation par pools pré-alloués et *Generational Arena*, isolation du step physique. |
| **`firework-imgui`** | `src/window_engine/**`<br>`src/utils/command_console/`<br>`src/profiler.rs`<br>`doc/*profiling*` | Mode immédiat pur (pas de duplication d'état), isolation des handles GL bruts, réutilisation des buffers de texte pour le profilage. |

---

## 4. Outils & Commandes (Taskfile)

La régénération et l'application des overlays sont entièrement automatisées via le task runner **Go-Task** (`Taskfile.yml`).

### Exécuter la mise à jour complète
À lancer après chaque modification majeure de l'architecture, ajout de module, ou mise à jour de spécification technique dans `doc/` :

```bash
task ai:update-skills
```

### Comportement interne du Taskfile.yml
La tâche s'exécute en séquence pour chaque module via une tâche interne factorisée (`_ai:generate-overlay-skill`) :

1. Appel de npx repomix@latest avec le filtre --include approprié.
2. Export du résultat dans ~/.agents/skills/firework-<module>.
3. Vérification conditionnelle de l'existence de l'overlay dans doc/ai_skills/firework-<module>.md.
4. Écrasement silencieux du SKILL.md générique par notre overlay versionné via cp -f.

## 5. Guide d'Invocation dans Antigravity (agy)
Le client CLI Antigravity charge dynamiquement les compétences en lisant leurs en-têtes YAML. Il existe deux modes d'utilisation :

### Routage Implicite (Recommandé pour le code quotidien)
L'agent analyse le langage naturel du prompt et le fait correspondre au champ description: des fichiers SKILL.md.
```sh
# Déclenche automatiquement [firework-audio] via les mots-clés "DSP", "CPal", "sans allocation"
agy run "Ajoute un filtre passe-bande dans la chaîne DSP de CPal sans allouer de mémoire dans le callback."

# Déclenche automatiquement [firework-imgui]
agy run "Ajoute un widget ImGui dans command_console/ pour afficher la consommation mémoire du GPU."
```

### Routage Explicite (Forçage de contexte pour l'architecture)
Pour forcer l'attention du modèle sur un ou plusieurs modules spécifiques et éliminer tout risque d'hallucination, nommez explicitement les compétences entre crochets [] ou backticks dans votre prompt :
```sh
# Forçage strict d'un module unique
agy run "En te basant obligatoirement sur le skill [firework-renderer], écris un nouveau shader d'étincelles en respectant l'alignement std140 de nos UBO."

# Croisement de compétences (Cross-domain)
agy run "Utilise les skills [firework-physic] et [firework-imgui].
```
