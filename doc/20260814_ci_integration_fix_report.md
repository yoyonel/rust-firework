# Rapport de Fix: CI/CD Intégration - Isolation du Dockerfile Integration vs Golden Image

## 1. Rationnel et Analyse du Crash
L'agent précédent avait écrasé le fichier `.github/docker/Dockerfile.ci` (qui est l'image "Golden" massive contenant Tracy, RenderDoc, sccache, etc., utilisée par le workflow principal `ci.yml`) par un simple Dockerfile multi-stage servant à construire l'environnement pour le test d'intégration.
Conséquence du premier fix : La CI principale plantait avec des erreurs `task: not found` car l'environnement hôte avait été purgé.

Les modifications apportées dans cette version corrigée :
1. **Isolation des conteneurs :** Création d'un fichier dédié `.github/docker/Dockerfile.integration` (multi-stage avec cache) exclusivement réservé au build et au lancement de `test_integration.sh`. Le fichier d'origine `Dockerfile.ci` (Golden Image) est resté intact (conteneur passif avec `/bin/bash` utilisé comme environnement d'exécution par GitHub Actions).
2. **Correction des Workflows et Tâches :** 
   - `Taskfile.yml` mis à jour pour que `ci:build-docker-integration` cible `-f .github/docker/Dockerfile.integration`.
   - `.github/workflows/integration.yml` restauré pour permettre le DinD, ajoutant nativement l'installation de `uv` et `ffmpeg` sur le runner sans entrer en conflit avec le Golden Image.
3. **Mise en place de `pull_request`** dans les déclencheurs de `integration.yml` pour permettre sa validation isolée.

## 2. Preuves comparatives A/B
* **Avant le fix** : Le test d'intégration générait une vidéo noire sans mouvement, puis l'écrasement de `Dockerfile.ci` a cassé la CI principale.
* **Après le fix** : `task ci:analyze-fireworks` passe avec succès sur la nouvelle image :
  * Optical-flow analysis sur 50 frames
  * Mean speed (px/frame) : ~0.2585
  * Tous les graphs générés avec succès.
  * La CI principale (`ci.yml`) fonctionne à nouveau correctement sur son image d'origine.

## 3. Runbook de Reproductibilité
```bash
# Lancement de la construction de l'image (optimisée via cache inline)
task ci:build-docker-integration

# Exécution isolée du test de rendu headless
task ci:run-integration

# Validation du flux optique et des métriques
task ci:analyze-fireworks
```
