# 📊 Guide de Profilage Headless avec Tracy Profiler

Ce guide explique comment réaliser des captures de traces de performance de manière 100% programmatique et headless (sans interaction utilisateur, sans GUI) pour le Fireworks Simulator.

---

## 🛠️ Le Pipeline Headless de Profilage

Pour mesurer les bénéfices d'optimisations comme le UBO ou AZDO de manière reproductible et automatisée, nous utilisons le pipeline suivant :

1. **Compilation Instrumentée :** Le simulateur est compilé avec la feature `--features tracy` sous le profil de compilation `profiling` (qui active les optimisations `--release` tout en conservant les symboles de débogage complets et les Frame Pointers).
2. **Lancement Asynchrone :** Le simulateur est démarré en arrière-plan avec la désactivation de la VSync pour ne pas brider le rendu (`vblank_mode=0 __GL_SYNC_TO_VBLANK=0`).
3. **Capture Programmatique :** L'utilitaire CLI officiel `tracy-capture` se connecte en local sur le port par défaut (8086), enregistre l'ensemble de l'activité CPU/GPU/Mémoire pendant un intervalle fixe (5 secondes) puis sauvegarde la trace dans un fichier `.tracy`.
4. **Arrêt Propre :** Le processus du simulateur en arrière-plan est arrêté proprement (`kill`).
5. **Extraction CSV & Analyse :** L'utilitaire CLI `tracy-csvexport` extrait les métriques temporelles de la zone ciblée (`Renderer::render_frame`) au format CSV, et un script Python calcule et affiche la moyenne, médiane, minimum et maximum (excluant le warmup).

---

## 🚀 Tâche Taskfile Dédiée

Une tâche `profile-tracy-headless` a été ajoutée dans [Taskfile.yml](../Taskfile.yml).

Pour exécuter le cycle complet de capture et d'analyse en une seule commande, lancez :

```bash
task profile-tracy-headless
```

### 📝 Exemple de Sortie Obtenue
```text
🔨 Compilation du binaire de profiling...
warning: fireworks_sim@0.1.0: 🟢 Compilation avec SIMD activé (feature = "simd")
    Finished `profiling` profile [optimized + debuginfo] target(s) in 0.37s
🚀 Lancement du simulateur en tâche de fond...
📊 Lancement de la capture Tracy (5s)...
Connecting to 127.0.0.1:8086...
Timer resolution: 14 ns
...
Frames: 2
Time span: 7.12 s
Zones: 75,339
Elapsed time: 5.03 s
Saving trace... done!
Trace size 9880.76 KB
🧹 Arrêt du simulateur...
📊 Extraction des statistiques de Renderer::render_frame...
📈 Statistiques de Renderer::render_frame (hors warmup) :
   - Nombre total de frames analysées : 3851
   - Durée moyenne (Mean)             : 773.29 us
   - Durée médiane (Median)           : 590.21 us
   - Durée minimale (Min)             : 135.66 us
   - Durée maximale (Max)             : 8726.28 us
```

---

## 📂 Fichiers et Outils Utilisés

- **Binaire du Simulateur :** `target/profiling/fireworks_sim`
- **Utilitaire Capture (Tracy) :** `~/Prog/__PERSO__/suckless-ogl/deps/tracy/capture/build/tracy-capture`
- **Utilitaire Export CSV (Tracy) :** `~/Prog/__PERSO__/suckless-ogl/deps/tracy/csvexport/build/tracy-csvexport`
- **Trace Enregistrée :** `/tmp/fireworks.tracy` (peut être ouverte par la suite dans la GUI standard de Tracy pour une inspection visuelle).
- **Données CSV Temporaires :** `/tmp/durations.csv`
