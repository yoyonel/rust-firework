# Ubiquitous Language

Ce document formalise la terminologie et le langage omniprésent (*Ubiquitous Language*) du projet `rust-firework`. Il définit les termes métier partagés par les développeurs et les experts du domaine de la simulation physique, du rendu graphique OpenGL, et du moteur audio temps réel.

## Physique et Simulation

| Terme | Définition | Synonymes / Alias à éviter |
| :--- | :--- | :--- |
| **Fusée (Rocket)** | Un projectile propulsé vers le ciel qui déclenche une explosion lorsqu'il atteint son apogée. | Projectile, Missile |
| **Particule (Particle)** | Un élément unitaire issu d'une explosion, doté d'une vitesse, position, durée de vie et couleur. | Point, Étincelle, Dot |
| **Explosion (Explosion)** | L'événement de détonation d'une fusée qui génère un ensemble de particules. | Boom, Blast, Détonation |
| **Forme d'Explosion (Explosion Shape)** | Le motif géométrique (sphère, cœur, anneau, etc.) utilisé pour définir les directions d'éjection des particules. | Pattern, Layout |
| **Moteur Physique (Physic Engine)** | Le système qui calcule et met à jour la position et l'état des fusées et des particules à chaque pas de temps. | Simulation, Boucle physique |

## Rendu Graphique (OpenGL)

| Terme | Définition | Synonymes / Alias à éviter |
| :--- | :--- | :--- |
| **Buffer Persistant (Persistent Buffer)** | Une zone mémoire GPU partagée et mappée en continu sur laquelle le CPU écrit directement sans appel bloquant. | Mapped buffer, Shared buffer |
| **Appel de Dessin (Draw Call)** | Une commande OpenGL unique (`glDraw*`) ordonnant à la carte graphique de dessiner un ensemble de primitives. | Commande de rendu, Paint call |
| **Barrière de Synchronisation (Sync Fence)** | Un verrou OpenGL (`GLsync`) coordonnant les accès du CPU et du GPU sur les buffers persistants pour éviter les conflits de lecture/écriture. | Lock, Barrier |
| **Bloom** | Un effet post-processus de flou appliqué aux zones lumineuses pour générer un halo lumineux. | Glow, Flou, Éclat |
| **Quad / Billboard** | Une forme rectangulaire à 4 sommets orientée face à la caméra pour dessiner les particules texturées. | Sprite, Point |

## Moteur Audio et Spatialisation

| Terme | Définition | Synonymes / Alias à éviter |
| :--- | :--- | :--- |
| **Voix (Voice)** | Une instance active de lecture audio (comme le son d'une fusée ou d'une explosion) gérée par le moteur audio. | Son, Piste, Track |
| **Vol de Voix Prioritaire (Voice Stealing)** | Algorithme réattribuant une voix active faible à un nouveau son plus prioritaire lorsque les 128 voix sont saturées. | Drop sonore, Interruption voix |
| **Atténuation Inverse-Distance (Inverse-Distance Roll-off)** | Modèle d'atténuation logarithmique de la puissance sonore selon la distance $1/d$ entre la source et l'auditeur. | Décroissance linéaire, Fade audio |
| **Positionnement Atomique (`AtomicVec2`)** | Transmission thread-safe sans verrou (lock-free) des coordonnées $f32$ du Listener du thread principal au thread audio CPAL. | Thread lock, Mutex listener |
| **Moniteur de Diagnostic Audio (Audio Diagnostic Monitor)** | Interface ImGui et télémétrie temps réel affichant les latences (transit / render-to-start) et l'état des événements sonores. | Profiler audio, Log window |
| **Effet Doppler (Doppler Effect)** | Le décalage de fréquence d'un son causé par le déplacement relatif d'une source audio par rapport à l'auditeur. | Pitch shift, Glissement de fréquence |
| **Événement Doppler (Doppler Event)** | Un message asynchrone transmettant la position et la vitesse d'une source sonore au moteur audio. | Message Doppler, Audio update |
| **Binauralisation (Spatialisation 3D)** | Le traitement audio simulant la provenance 3D d'un son dans un casque audio à l'aide de fonctions de transfert (HRTF). | Son 3D, Stéréo 3D |

## Relations clés

* Une **Fusée** génère exactement une **Explosion** à la fin de sa durée de vie.
* Une **Explosion** produit un ensemble de **Particules** selon une **Forme d'Explosion**.
* Le **Moteur Physique** met à jour la position des **Fusées** et des **Particules**.
* Le déplacement d'une **Fusée** génère périodiquement des **Événements Doppler** envoyés au moteur audio.
* Le moteur audio applique un **Effet Doppler** sur la **Voix** associée à la source sonore en fonction des **Événements Doppler** reçus.
* Pour le rendu, le CPU écrit les positions des **Particules** dans le **Buffer Persistant** et place une **Barrière de Synchronisation** pour que le GPU puisse effectuer son **Appel de Dessin** sans conflit.

## Dialogue d'Exemple

> **Dev :** "Quand une **Fusée** explose, est-ce que les **Particules** sont générées immédiatement dans le **Moteur Physique** ?"
>
> **Expert Métier :** "Oui. L'**Explosion** génère un ensemble de **Particules** dont les directions de départ dépendent de la **Forme d'Explosion** sélectionnée (sphère, cœur, etc.)."
>
> **Dev :** "Et pour le son, est-ce que le déplacement de la **Fusée** génère des **Événements Doppler** à chaque frame ?"
>
> **Expert Métier :** "Exactement. Ces **Événements Doppler** permettent de recalculer en temps réel l'**Effet Doppler** appliqué à la **Voix** correspondante dans le moteur audio, afin d'ajuster sa fréquence de lecture."
>
> **Dev :** "Comment s'assure-t-on que le CPU n'écrase pas les positions des **Particules** dans le **Buffer Persistant** pendant que le GPU dessine ?"
>
> **Expert Métier :** "On utilise des **Barrières de Synchronisation** (Sync Fences) pour coordonner les accès. Le CPU attend que le GPU ait fini son **Appel de Dessin** précédent sur cette section du buffer avant d'y réécrire."

## Ambiguïtés levées

* **"Son" vs "Voix" :** Le terme "Son" était utilisé de manière ambiguë pour désigner un fichier audio ou une instance de lecture. Nous utilisons désormais **Voix** pour l'instance active en cours de lecture et de spatialisation sur la carte son.
* **"Point" vs "Quad" :** Historiquement, les particules standard étaient dessinées sous forme de points OpenGL, tandis que les fusées utilisaises des quads texturés. Nous distinguons désormais explicitement les **Particules** (rendu de points simples) et les **Quads / Billboards** (rendu instancié texturé).
