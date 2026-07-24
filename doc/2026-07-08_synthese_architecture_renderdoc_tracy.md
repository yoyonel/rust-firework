# 2026-07-08 - SYNTHÈSE D'ARCHITECTURE : INSTRUMENTATION RENDERDOC & TRACY (RUST / OPENGL)

Ce document synthétise les refactorings architecturaux et les ajouts d'outillage menés sur le moteur de rendu Rust pour intégrer le profilage CPU (Tracy) et le débogage GPU (RenderDoc). L'objectif était de transformer un pipeline graphique "boîte noire", invisible pour les débogueurs, en une architecture totalement transparente, hiérarchisée et nommée (façon moteur AAA).

---

## 1. DIAGNOSTIC INITIAL : L'AVEUGLEMENT DU DÉBOGUEUR GPU

### Le Problème du RenderDoc "Vide"
Initialement, la capture RenderDoc ne montrait aucune ressource, aucun Draw Call, et remontait des erreurs de type `GL_INVALID_OPERATION in glGetVertexAttribfv(index==0)`.
* **La Cause :** Le contexte OpenGL était initialisé sans le flag de débogage, et les callbacks de debug étaient désactivés manuellement. Face à un pipeline légèrement incomplet ou dépourvu de `Sync Object` lors des écritures de buffers persistants, le driver OpenGL rejetait silencieusement les requêtes. RenderDoc refusait de capturer un pipeline invalide.
* **La Solution :** Activation du contexte de debug via GLFW (`OpenGlDebugContext(true)`) et stabilisation explicite de la liaison des VAOs avant le rendu.

### Le Problème du Rendu "À Plat" et Anonyme
Une fois les Draw Calls débloqués, l'Event Browser affichait une liste plate (sans dossiers) de `glDrawArrays` impossibles à distinguer, manipulant des "Texture 45" et "Framebuffer 3". Impossible de profiler l'impact d'une passe de post-traitement par rapport au rendu des instanciations.

---

## 2. RÉSOLUTION ARCHITECTURALE : L'OUTILLAGE SAFE & GLOBAL

Pour pallier le manque de sémantique, nous avons implémenté l'extension OpenGL `KHR_debug` de manière sécurisée (Zero-Crash).

### Module `instrumentation.rs` et Macros Globales (`#[macro_export]`)
* **Le Problème :** Appeler des fonctions d'extension OpenGL directement expose à des crashs si le driver du client ne les supporte pas. De plus, placer le code de debug au milieu de la logique métier violait la séparation des préoccupations (SoC).
* **La Solution :** Création d'un module utilitaire exposant trois macros injectées à la racine du crate (`#[macro_export]`) :
    1. `push_debug_group!(id, "Nom")`
    2. `pop_debug_group!()`
    3. `label_gl_object!(type, id, "Nom")`
* **Le Gain :** Les macros vérifient en \\( O(1) \\) si la fonction (`is_loaded()`) est supportée par le driver. Le code de rendu reste purement sémantique sans être pollué par des vérifications de sécurité OpenGL.

---

## 3. HIÉRARCHISATION DES PIPELINES GPU (EVENT BROWSER)

Nous avons encerclé les logiques de rendu critiques par des balises `PushDebugGroup` et `PopDebugGroup` pour créer une arborescence lisible.

* **Architecture Injectée :**
    * 📂 **`Pass: HDR Scene`** (Gérée par le `Renderer` principal)
        * 📂 **`Draw All Particles`**
            * `Draw Points` (Rendu via VBO persistant)
            * `Draw Instanced Quads` (Rendu des roquettes)
    * 📂 **`Pass: Forward (No Bloom)`** (Chemin alternatif si PostFX désactivé)
    * 📂 **`PostFX: Bloom Blur Chain`** (Dans `BloomPass`)
        * Contient toutes les itérations Ping-Pong (ex: Gaussian ou Kawase).
    * 📂 **`PostFX: ToneMapping & Composition`** (Rendu final sur écran)

* **Le Gain :** La Timeline RenderDoc affiche désormais des segments de couleurs par passe graphique. Le coût temporel de chaque bloc (en \\( \mu \\)s) est calculé par RenderDoc instantanément.

---

## 4. NOMMAGE SÉMANTIQUE DES RESSOURCES (TEXTURE VIEWER)

Au lieu de laisser OpenGL générer des ID numériques anonymes, chaque ressource est désormais labellisée à sa création via `gl::ObjectLabel`.

* **Cibles d'Instrumentation :**
    * **Shaders :** `Shader_PointRendering`, `Shader_InstancedQuad`, `Shader_Bloom_Gaussian`...
    * **Buffers & VAO :** `VAO_Points`, `VBO_Instanced_Data`, `VBO_Static_Quad`...
    * **Textures & FBOs :** `Tex_HDR_Brightness_Mask`, `Tex_Blur_Ping`, `FBO_HDR_Main`...
* **Le Gain :** Inspection visuelle immédiate des entrées/sorties (Inputs/Outputs) dans le "Pipeline State" et le "Texture Viewer" de RenderDoc. Facilite le débogage visuel des passes intermédiaires (comme le *bright pass* du Bloom).

---

## 5. PROFILING CPU : INTÉGRATION DE TRACY

Pour mesurer l'impact du moteur CPU indépendamment du GPU, nous avons ajouté un système de balises CPU.

* **La Solution :** Macro `tracy_zone!("Nom", couleur)`.
* **Optimisation de Build :** Enveloppée dans `#[cfg(feature = "tracy")]`, cette macro se compile dans le néant total (zéro instruction) lors d'un build standard, garantissant que l'instrumentation CPU ne coûte rien en mode *release* normal.

---

## 6. TABLEAU SYNTHÉTIQUE DES GAINS OBSERVÉS

| Refactoring / Instrumentation | Motif Technique & Architecture | Gain Observé (Empirique) |
| :--- | :--- | :--- |
| **Macros de Profiling (`#[macro_export]`)** | Isoler la logique de debug (`KHR_debug`) du code métier en évitant les crashs sur des drivers anciens. | **Zéro crash / Zéro boilerplate**. Instrumentation disponible globalement avec gestion automatique du fallback. |
| **Object Labels (`gl::ObjectLabel`)** | Les ressources GPU (Texture 42, Buffer 12) rendaient le "Texture Viewer" indéchiffrable. | Identifiants sémantiques clairs (`Tex_Blur_Ping`, `VBO_Instanced_Data`). Permet de valider visuellement les passes MRT instantanément. |
| **Debug Groups (Push/Pop)** | Event Browser de RenderDoc affichant des centaines de `glDrawArrays` à plat. | Création d'une **Timeline hiérarchisée et colorée**. Isolation immédiate des goulots d'étranglement GPU. |
| **Découverte de la Charge Bloom** | Profiter de l'instrumentation pour autopsier le pipeline de post-traitement. | **Constat critique mesuré :** La `Bloom Blur Chain` (Gaussienne ping-pong à haute résolution) consomme **1.28 ms** (~20% du budget frame pour du 144Hz). |
| **Pistes d'optimisations validées** | Résoudre le goulot d'étranglement mesuré ci-dessus. | Identification immédiate des leviers de performance : augmentation du `downsample_factor`, passage au flou `Kawase`, ou usage du filtrage bilinéaire hardware. |

---

## 7. CONTEXTE POUR LA PROCHAINE CONVERSATION

L'architecture GPU de base est dorénavant transparente. La prochaine étape logique, basée sur les découvertes de cette instrumentation, sera de s'attaquer au goulot d'étranglement du **Fill-rate / Bande Passante Mémoire** identifié dans le `Bloom Blur Chain` (réduction drastique du temps d'exécution ciblant les \\( < 300\mu s \\) via le *Kawase blur* et l'optimisation des FBO ping-pong).
