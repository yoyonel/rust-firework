# Guide d'Analyse VTune : Threading & Synchronisation (Locks & Waits)

La commande `task benchmark-vtune-threading` exécute un profilage matériel centré sur la manière dont les threads se partagent l'exécution et interagissent. L'objectif est d'identifier les goulets d'étranglement de synchronisation (Lock Contention) et l'inefficacité multi-core.

## 1. Glossaire et Terminologie

### Objets de Synchronisation
- **Mutex (Mutual Exclusion)** : Verrou exclusif utilisé pour protéger une zone mémoire (ex: l'accès à ImGui). Un seul thread peut le posséder.
- **RwLock (Read-Write Lock)** : Verrou optimisé autorisant plusieurs lecteurs simultanés, mais un seul écrivain exclusif (pratique pour des données souvent lues, rarement mutées).
- **Canaux Crossbeam (Channels)** : Structures (MPSC/MPMC) permettant d'envoyer des messages (ex: des `PhysicCommand`) d'un thread à l'autre de manière "Lock-Free" ou optimisée.
- **Condition Variable (Condvar)** : Primitives du système d'exploitation permettant de mettre un thread en "sommeil profond" jusqu'à ce qu'un autre thread le réveille (ex: un worker attendant une tâche, ou l'attente du VSync).
- **poll() / select() / epoll()** : Appels systèmes (souvent vus dans GLFW, Audio, ou le Réseau) où un thread attend qu'un événement matériel ou logiciel se produise, en étant mis en veille par le Kernel.

### Métriques d'Exécution
- **Wait Time** : Temps cumulé total pendant lequel un thread était inactif, en train d'attendre qu'une ressource se libère (un Lock) ou qu'un événement se produise (Condition Variable). Avoir un "Wait Time" élevé n'est pas forcément mauvais si le thread n'avait volontairement rien à faire.
- **Spin Time (Lock Spinning)** : C'est le fléau de la performance. Lorsqu'un thread cherche à prendre un Lock qui est déjà occupé, il peut décider de "Spin" (boucler à l'infini dans une boucle `while` très rapide) en gaspillant des cycles CPU purs, dans l'espoir que le Lock se libère en une fraction de milliseconde. Un *Spin Time* élevé indique un Mutex violemment contesté (goulot d'étranglement).
- **Overhead Time** : Temps CPU perdu par le scheduler du système d'exploitation (Kernel) pour effectuer les changements de contexte (Context Switches) entre les threads.

## 2. Que cherche-t-on à diagnostiquer ? (Bottlenecks)

### Bottleneck A : Lock Contention Massive (Spin Time Élevé)
Si le rapport VTune montre un `Spin and Overhead Time` élevé (ex: `> 5%` du CPU Time), cela signifie que vos threads se battent pour la même donnée. 
* *Exemple :* Le thread Physique tente d'écrire dans un `Arc<Mutex<Data>>` alors que le thread de Rendu passe son temps à le lire. 
* *Solution :* Utiliser un `RwLock`, double-buffering, ou migrer vers une file de messages (Event-Driven / CQRS) pour découpler les producteurs des consommateurs.

### Bottleneck B : Threads Sous-Utilisés (Load Imbalance)
Si l'`Effective CPU Utilization` est très bas et que le `Wait Time` n'est imputé qu'à des `Mutex` ou `Channels` bloquants de synchronisation asynchrone, cela signifie qu'un thread maître va trop lentement et laisse les autres mourir de faim (Starvation).
* *Exemple :* Le thread audio `CPal` attend que la physique produise un son, mais la physique est engluée.
* *Solution :* Diviser la charge physique via des frameworks comme `Rayon`.

## 3. Comportement Idéal (Architecture Saine)
Un moteur bien optimisé lors de l'exécution de `task benchmark-vtune-threading` affichera :
1. **Spin Time = 0s** (Aucun gaspillage d'énergie, pas de bagarre sur les locks).
2. **Top Waiting Objects = Condition Variable / poll** (Les threads inactifs sont sagement endormis par le Kernel et ne consomment pas de CPU).
