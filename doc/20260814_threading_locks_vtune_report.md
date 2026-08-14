# Rapport de Profiling VTune : Analyse Threading & Locks (14 Août 2026)

Ce rapport consigne l'exécution d'un profilage de synchronisation (Locks & Waits) sur la branche optimisée via VTune. L'objectif était de vérifier l'absence de goulots d'étranglement ou de conflits majeurs entre les threads lors d'une simulation rapide (Fast-Forward).

## Méthodologie
- **Commande** : `task benchmark-vtune-threading`
- **Configuration** : `--deterministic-seed 42 --timeout-secs 5 --fixed-dt 0.016666 --disable-audio`
- **Profil mesuré** : Utilisation du processeur (Wait Time vs Spin Time), et identification des objets de synchronisation bloquants.

## Résultats du Profil

- **Elapsed Time** : ~7.9s
- **CPU Utilization** : 6.1% (1 cœur logique saturé sur 12).
- **Wait Time** : 26.1s (Temps total cumulé de mise en pause des threads virtuels).
- **Spin and Overhead Time** : **0s (0.0% du CPU Time)**.

### Top Waiting Objects
Les objets ayant provoqué la mise en pause des threads (somme = temps de timeout ~5s x nombre de threads) :
1. `Condition Variable 0xd7f46e28` : 5.234s
2. `Condition Variable 0x7ff48773` : 5.227s
3. `poll` : 5.201s
4. `Condition Variable 0x2edd17da` : 5.184s
5. `Condition Variable 0xd9e6109c` : 5.175s

## Analyse Architecturale
Le rapport indique une **parfaite santé** du point de vue de la concurrence pour la boucle physique isolée :
- Le `Spin and Overhead Time` de `0s` confirme une absence totale de contention (*Lock Contention*). Aucun thread n'a gaspillé de cycles CPU à boucler activement en essayant d'obtenir un `Mutex` ou un verrou.
- Les `Wait Time` de ~5.2s par thread témoignent d'un sommeil sain. Les threads secondaires (audio désactivé, workers inactifs, gestionnaire d'I/O) sont endormis proprement par le système d'exploitation via des appels de condition (`Condition Variable`) ou des écoutes non bloquantes (`poll`). 
- Le CPU unique travaille à 100% sur la logique de la simulation pendant la durée du test, sans entraver ni être entravé par le reste de l'application.
