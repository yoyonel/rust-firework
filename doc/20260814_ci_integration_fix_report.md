# Rapport de Fix: CI/CD Intégration - Restauration Multi-stage et ISO

## 1. Rationnel
L'agent "DevSecOps" avait écrasé le `Dockerfile.ci` en remplaçant la logique `multi-stage` par un simple `CMD ["/bin/bash"]`, transformant le conteneur en run passif.
Conséquence : la CI passait silencieusement car la vidéo était vide, masquant l'absence d'exécution du simulateur.

Les modifications apportées :
1. Rétablissement de `Dockerfile.ci` en multi-stage avec un environnement `ubuntu:22.04` pour builder le binaire release avec mise en cache Cargo et exécution par défaut de `.github/scripts/test_integration.sh`.
2. Restauration de `.github/workflows/integration.yml` pour réutiliser la structure DinD (Docker in Docker) correcte, avec setup approprié pour `uv` et `ffmpeg`.
3. Correction de `Taskfile.yml` pour cibler correctement le chemin `.github/docker/Dockerfile.ci`.

## 2. Preuves comparatives A/B
* **Avant le fix** : Le conteneur CI démarrait et quittait instantanément via `/bin/bash`. 0 frame rendue, mouvement absent.
* **Après le fix** : `task ci:analyze-fireworks` passe avec succès :
  * Optical-flow analysis sur 50 frames
  * Mean speed (px/frame) : ~0.2585
  * Tous les graphs générés avec succès.

## 3. Runbook de Reproductibilité
```bash
# Lancement de la construction de l'image (optimisée via cache inline)
task ci:build-docker-integration

# Exécution isolée du test de rendu headless
task ci:run-integration

# Validation du flux optique et des métriques
task ci:analyze-fireworks
```
