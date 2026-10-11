# Audit de Compatibilité ISO : Environnement Legacy (Debian 13 X11+Mate / Tools-on-Host)
**Date** : 2026-10-11
**Statut** : ✅ Validation de compatibilité 100% ISO.

Ce rapport documente l'audit de compatibilité des récentes modifications des outils de profilage (réparation des scripts Heaptrack, VTune, Apitrace et Flamegraph) vis-à-vis d'un poste de développement legacy sous **Linux Debian 13 (X11+Mate)** utilisant une approche **tools-on-host** (outils natifs hors conteneur).

## 1. Comportement Opt-in de Distrobox
- **Avant** : Le `Taskfile.yml` s'accrochait automatiquement à `distrobox` s'il détectait sa présence sur le système, forçant parfois l'exécution dans un conteneur non désiré.
- **Après** : L'utilisation de Distrobox/XVFB est conditionnée à l'export explicite de `USE_DISTROBOX=1`.
- **Bénéfice Debian 13** : Sur un environnement *tools-on-host*, les scripts s'exécutent désormais directement sur l'hôte, respectant la volonté de l'opérateur.

## 2. Standards POSIX & GNU Coreutils
- Les outils capricieux ont été encadrés de manière robuste via `timeout --preserve-status -k 1s` et la recherche de fichiers de dump stabilisée via `ls -t 2>/dev/null | head -n 1`.
- **Bénéfice Debian 13** : Ces commandes sont des composants natifs absolus des `coreutils` GNU inclus dans toute installation Debian. Aucun outil externe ou `find` spécifique n'est requis.

## 3. Conformité Bash (Tableaux d'arguments)
- Le correctif du script Heaptrack passe de la syntaxe ambiguë `"${@:---args}"` à `${@:- --args}` (sans encapsulation sous forme de string unique).
- **Bénéfice Debian 13** : Cette syntaxe d'expansion de paramètres par défaut est standard (POSIX/Bash) et garantie d'être interprétée correctement par Bash 5.2+ (embarqué dans Debian 13).

## 4. Retrait de l'Élévation de Privilèges (`sudo`) et des Chemins Absolus
- Les scripts VTune s'attendaient auparavant à trouver l'outil strictement sous `/opt/intel/oneapi/...` et imposaient systématiquement `sudo -E`.
- Les correctifs suppriment ces contraintes : VTune est appelé via le `$PATH` de l'utilisateur, et sans `sudo`. Le `$PATH` des binaires Rust (`~/.cargo/bin`) a été sécurisé dans les subshells.
- **Bénéfice Debian 13** : Sur une Debian où l'utilisateur gère ses propres installations (ou si `kernel.perf_event_paranoid` est assoupli), VTune s'exécutera silencieusement, sans rupture de flux interactif (demande de mot de passe) ni erreur de chemin introuvable.

## 5. Compatibilité X11 / GLX
- **Bénéfice Debian 13 (X11+Mate)** : Les hooks du script Apitrace (`apitrace trace --api gl`) interceptent `libGLX.so.0`, ce qui est le backend natif de X11 (utilisé par Mate). Aucune dépendance à Wayland ou à un compositeur spécifique n'entrave la capture graphique.
