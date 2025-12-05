# Formes d'Explosion Personnalisées

Cette fonctionnalité permet de modifier dynamiquement la forme des explosions de feux d'artifice en utilisant des images comme modèles.

## Principe de Fonctionnement

Le moteur physique échantillonne les pixels blancs d'une image en niveaux de gris pour déterminer les positions cibles des particules. Lors de l'explosion :

1.  Les particules sont générées à la position de la fusée.
2.  Leur vitesse initiale est calculée pour qu'elles atteignent les positions cibles définies par l'image.
3.  La forme est orientée selon la direction de la fusée au moment de l'explosion.
4.  La conservation du mouvement est appliquée : l'explosion continue de suivre la trajectoire balistique de la fusée.

## Commandes Console

### 1. Chargement de Presets

Utilisez la commande `physic.explosion.preset` pour charger des formes préconfigurées.

```bash
physic.explosion.preset <nom>
```

**Presets disponibles :**

| Nom      | Description | Paramètres (Scale / Time) |
| :---     | :---        | :--- |
| `star`   | ⭐ Étoile classique | 180.0 / 1.5s |
| `heart`  | ❤️ Cœur | 150.0 / 1.5s |
| `smiley` | ☺ Visage souriant | 200.0 / 2.0s |
| `note`   | ♪ Note de musique | 160.0 / 1.5s |
| `ring`   | ⭕ Anneau / Planète | 190.0 / 1.8s |

*(Note : Tous les paramètres sont optimisés pour chaque forme)*

### 2. Chargement d'Images Personnalisées

Vous pouvez charger n'importe quelle image PNG/JPEG (noir et blanc recommandé).

```bash
physic.explosion.image <chemin_fichier> [scale] [flight_time]
```

*   **chemin_fichier** : Chemin relatif vers l'image (ex: `assets/textures/explosion_shapes/custom.png`)
*   **scale** (optionnel, défaut 150) : Taille de l'explosion en pixels.
*   **flight_time** (optionnel, défaut 1.5) : Durée de déploiement de l'explosion en secondes.

### 3. Ajustement Fin

Une fois une forme chargée, vous pouvez ajuster ses paramètres en temps réel :

*   `physic.explosion.scale <valeur>` : Modifie la taille.
*   `physic.explosion.flight_time <valeur>` : Modifie la vitesse de déploiement.

### 4. Réinitialisation

Pour revenir au comportement par défaut (explosion sphérique aléatoire) :

```bash
physic.explosion.shape spherical
```

## Création de Formes Personnalisées

Pour créer vos propres formes :
1.  Créez une image carrée (ex: 512x512).
2.  Fond noir, dessin en blanc.
3.  Sauvegardez en PNG dans `assets/textures/explosion_shapes/`.
4.  Chargez-la via la console.

La forme sera automatiquement centrée sur son barycentre et orientée selon la trajectoire de la fusée.
