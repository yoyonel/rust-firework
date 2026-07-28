# Guide des Méthodologies d'Évaluation de Fonctionnalités & Choix d'Architecture

## Overview & Contexte

Dans le cadre du développement de **`rust-firework`**, le choix des optimisations graphiques (GPU AZDO, compute shaders), des fonctionnalités du moteur audio (Bus Spatial 2D, Reverb, HRTF), ou du moteur physique nécessite des critères d'évaluation stricts et objectifs.

Ce document présente **4 frameworks complémentaires d'ingénierie logicielle** utilisés pour analyser, chiffrer et prioriser les idées et choix d'architecture.

---

## 1. La Matrice 2D : Valeur vs Effort (Value vs Effort Matrix)

La Matrice 2D croise l'**Effort/Complexité d'implémentation** (axe X) avec la **Valeur/Immersion produite** (axe Y). 

```text
       ▲ VALEUR / IMMERSION PRODUITE
       │
  HAUTE│   [⚡ QUICK WINS]                [🎯 PROJETS MAJEURS]
       │   Rendement maximal immédiat.    Fort impact, effort important.
       │   À exécuter en priorité !       À planifier soigneusement.
       │
  BASSE│   [💡 RESSORTISSONS / FILL-INS]   [🔴 PIÈGES À TEMPS / MONEY PITS]
       │   Faible effort, gain marginal.   Complexe pour peu de gain.
       │   À faire entre deux jalons.      À écarter ou déprioriser.
       └─────────────────────────────────────────────────────────────► EFFORT / COMPLEXITÉ
           FAIBLE                                         ÉLEVÉ
```

### Grille de Décision :
- **Quick Wins** (Effort Faible, Valeur Haute) : Tâches prioritaires à fort ROI (ex: *Spatial Reverb Bus*, *Distance Audio Atlas*).
- **Projets Majeurs** (Effort Élevé, Valeur Haute) : Évolutions stratégiques structurantes (ex: *Convoluteur HRTF Binaural*).
- **Fill-Ins** (Effort Faible, Valeur Basse) : Petits ajustements UI/Ergonomie.
- **Pièges à Temps** (Effort Élevé, Valeur Basse) : Fausse bonne idée (ex: *Ambisonics Ordre 2 pour sortie stéréo simple*).

---

## 2. Le Score RICE (Intercom Framework)

Le framework **RICE** fournit un score numérique quantifiable pour départager objectivement plusieurs fonctionnalités concurrentes.

> **Formule du Score RICE :**  
> **Score RICE = ( Reach &times; Impact &times; Confidence ) / Effort**

### Décomposition des 4 Facteurs :

1. **Reach (Portée / Fréquence)** : Pourcentage d'utilisateurs ou de cas d'usage impactés (*ex: 100% des fusées tirées*).
2. **Impact (Gain perçu)** :
   - `3.0` = Impact Massif / Transformationnel
   - `2.0` = Fort Impact
   - `1.0` = Impact Modéré
   - `0.5` = Impact Faible
3. **Confidence (Confiance / Certitude)** : Indice de confiance dans les estimations (*100% = certitude architecturale, 80% = moyen, 50% = spéculatif*).
4. **Effort (Complexité en jours-homme)** : Temps de conception, code et test.

### Exemple d'Application au Moteur Audio :

| Fonctionnalité | Reach | Impact | Confidence | Effort (jours) | Score RICE | Classement |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Distance Audio Atlas** | 100% (1.0) | 1.0 (Modéré) | 100% (1.0) | 0.5 jour | **200** | 🥇 **N° 1** |
| **Spatial Reverb Bus** | 100% (1.0) | 3.0 (Massif) | 100% (1.0) | 2.0 jours | **150** | 🥈 **N° 2** |
| **Décodeur HRTF Bus** | 70% (0.7 - Casque) | 2.0 (Fort) | 80% (0.8) | 2.0 jours | **56** | 🥉 **N° 3** |
| **HOA 2D Ordre 2** | 100% (1.0) | 0.5 (Faible) | 100% (1.0) | 2.0 jours | **25** | ❌ **Rejeté** |

---

## 3. Méthode ATAM (Architectural Trade-off Analysis Method)

Issue des travaux du *Software Engineering Institute (SEI / Carnegie Mellon)*, la méthode **ATAM** analyse les compromis d'architecture selon 5 **Attributs de Qualité (Quality Attributes)** :

```
             ┌──────────────────────────────────────────┐
             │       Qualité Globale Architecture      │
             └────────────────────┬─────────────────────┘
                                  │
      ┌───────────────┬───────────┴───┬───────────────┬───────────────┐
      ▼               ▼               ▼               ▼               ▼
┌───────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐
│Performance│   │ Mémoire   │   │ Latence   │   │ Richesse  │   │  Code     │
│ Temps Réel│   │ Bandwidth │   │  Audio    │   │  Sonore   │   │ Simplicité│
└───────────┘   └───────────┘   └───────────┘   └───────────┘   └───────────┘
```

1. **Performance Temps Réel** : Temps d'exécution \\( \mu\text{s} \\) par bloc audio ou frame GPU vs budget disponible (ex: 5.33 ms audio, 16.6 ms vidéo).
2. **Bande Passante Mémoire** : Accès contigus SIMD vectorisables (`vmulps`/`vaddps`), zéro allocation sur la boucle *hot-path*.
3. **Latence & Réactivité** : Transit time entre événement déclencheur et rendu final.
4. **Qualité & Immersion** : Richesse acoustique ou visuelle (équivalence ISO, dynamique).
5. **Simplicité du Code & Maintenabilité** : Nombre de lignes de code, absence d'états cachés ou de verrous (lock-free).

---

## 4. Cadre de Priorisation MoSCoW

Le cadre **MoSCoW** permet d'organiser la Feuille de Route (*Roadmap*) des livraisons logicielles en 4 catégories claires :

- **Must Have (Indispensable)** : Fonctionnalités vitale sans lesquelles l'application est inutilisable ou en échec de performance (*ex: Bus Spatial 2D, Voice Stealing*).
- **Should Have (Fortement Recommandé)** : Améliorations majeures à réaliser dès le jalon suivant (*ex: Spatial Reverb Bus*).
- **Could Have (Optionnel)** : Évolutions secondaires si du temps/budget reste disponible (*ex: Convoluteur HRTF Binaural*).
- **Won't Have (Exclu / Rejeté)** :Idées écartées explicitement à ce stade car le rapport coût/bénéfice est défavorable (*ex: Ambisonics Ordre 2 en stéréo*).
