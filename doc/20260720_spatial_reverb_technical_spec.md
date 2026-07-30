# Spécification Technique & Guide de la Réverbération Spatiale (`SpatialReverb`)

## 1. Vue d'Ensemble & Objectifs Acoustiques

Dans un spectacle pyrotechnique en extérieur, les explosions et détonations génèrent des impulsions sonores à forte dynamique. En espace ouvert, ces sons ne résonnent pas comme dans une pièce fermée : l'air absorbe très rapidement les hautes fréquences (les aigus), et les réflexions lointaines créent une traînée d'écho mat et profond.

La **Réverbération Spatiale Algorithmique sur Bus Unique (`SpatialReverb`)** a pour objectif d'apporter cette immersion acoustique naturelle au moteur audio de *Fireworks Simulator* tout en garantissant un coût calculatoire strictement constant O(1) sur le thread audio CPAL.

---

## 2. Architecture DSP & Traitement Signal

L'effet s'appuie sur une architecture hybride de type **Schroeder / FDN (Feedback Delay Network)**.

```text
  [Signal Stéréo Sec self.acc]
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│ 4 Filtres de Peigne en Parallèle (CombFilters)             │
│   - Normalisation : comb_scale = 0.25                       │
│   - Gain feedback  : 0.68                                   │
│   - Amortissement  : damp = 0.50 (Absorption HF par l'air)  │
│   - Délais Gauche  : [1553, 2129, 2801, 3547] samples       │
│   - Délais Droite  : [1600, 2176, 2848, 3594] samples       │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2 Filtres Tout-Passe en Série (AllPassFilters)              │
│   - Diffusion sans coloration fréquentielle                 │
│   - Feedback       : 0.35                                   │
│   - Délais Gauche  : [641, 317] samples                     │
│   - Délais Droite  : [688, 364] samples                     │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ Mixage Final (Wet / Dry)                                    │
│   Frame_out[n] = Dry[n] + (Wet[n] * wet_gain)                │
└─────────────────────────────────────────────────────────────┘
```

### 2.1. Filtres de Peigne en Parallèle (`CombFilter`)
Pour chaque canal (Gauche et Droite), 4 filtres de peigne en parallèle sont appliqués sur le signal sec :

```text
state[n]    = output[n] * (1 - damp) + state[n-1] * damp
buffer[pos] = input[n] + state[n] * feedback
```

- **Longueurs de délai** : `[1553, 2129, 2801, 3547]` échantillons @ 48 kHz (≈ 32 ms à 74 ms).
- **Amortissement HF par l'air (`damp = 0.50`)** : Filtre passe-bas du 1er ordre dans la boucle de feedback. Les aigus s'éteignent rapidement, évitant tout son métallique.
- **Normalisation des peignes (`comb_scale = 0.25`)** : Somme pondérée par 1 / N pour maintenir le gain unitaire et éliminer la saturation.

### 2.2. Filtres Tout-Passe en Série (`AllPassFilter`)
La sortie des filtres de peigne passe à travers 2 filtres tout-passe en série :

```text
output[n]   = -input[n] + buffer[pos]
buffer[pos] = input[n] + buffer[pos] * feedback
```

- **Diffusion sans coloration** : Augmente la densité de la traînée sans altérer la réponse en fréquence.
- **Feedback** : `0.35`.

### 2.3. Décorrélation Stéréo (Stereo Uncorrelation)
Pour éviter un écho plat centré en mono, le canal droit applique un décalage de **+47 échantillons (≈ 1 ms)** sur tous les délais :
- Gauche : `[1553, 2129, 2801, 3547]`
- Droite : `[1600, 2176, 2848, 3594]`

### 2.4. Performance & Complexité O(1)
Plutôt que d'instancier un effet de réverbération par fusée (incalculable à 128 voix), l'effet s'exécute **une seule fois par bloc CPAL** dans [dsp_processor.rs](../src/audio_engine/dsp_processor.rs#L66) directement sur le buffer accumulé `self.acc`.

---

## 3. Réglage & Influence du Paramètre `wet_gain`

Le paramètre `wet_gain` contrôle le ratio entre le signal sec (Dry) et le signal réverbéré (Wet) :

```text
Frame_out[n] = Dry[n] + (Wet[n] * wet_gain)
```

### Table des Niveaux de Gain Wet

| Valeur (`wet_gain`) | Mixage Wet (%) | Rendu Acoustique & Perception |
| :--- | :--- | :--- |
| `0.00` | 0 % | **Signal Sec Pur (Dry)** : Aucune réverbération. Le son direct des détonations est 100% mat. |
| **`0.08`** *(Défaut)* | **8 %** | **Écho d'Espace Extérieur Subtil** : Le son sec conserve 100% de sa clarté, suivi d'une traînée d'écho mat très naturelle (0.5 s à 1.0 s). |
| `0.20` | 20 % | **Réverbération Moyenne** : Simule un spectacle pyrotechnique dans une vallée, un canyon ou un espace entouré de structures réfléchissantes. |
| `0.50` | 50 % | **Écho Fort / Cathédrale** : Traînée très longue et enveloppante, adaptée aux grands volumes clos. |
| `1.00` | 100 % | **Noyé (100% Wet)** : Le son direct s'efface au profit de l'écho diffus. |

---

## 4. Commandes Console & Contrôle Dynamique

Toutes les modifications sont transmises au thread audio de manière **lock-free** via des atomiques (`Arc<AtomicU32>`), garantissant l'absence de clics ou de blocages du thread CPAL.

### 4.1. Activer / Désactiver la Réverbération
- **Basculer l'effet** :
  ```text
  audio.fx spatial_reverb <on|off>
  ```
- **Consulter l'état de tous les effets DSP** :
  ```text
  audio.fx_status
  ```

### 4.2. Ajuster le Niveau `wet_gain`
- **Afficher le niveau courant & l'aide** :
  ```text
  audio.reverb_wet
  ```
- **Modifier le gain à chaud** :
  ```text
  audio.reverb_wet 0.12
  ```

---

## 5. Fichiers et Symboles du Code

- **Module Réverbération** : [SpatialReverb](../src/audio_engine/spatial_reverb.rs)
- **Traitement par Bloc** : [DspProcessor::process_block](../src/audio_engine/dsp_processor.rs#L66)
- **Drapeau DSP** : [AudioEffect::SpatialReverb](../src/audio_engine/effect_flags.rs#L35)
- **Commande Console** : [simulator.rs](../src/simulator.rs#L1075)
- **Manuel des Commandes** : [console_commands.md](console_commands.md#L17)andes** : [console_commands.md](console_commands.md#L17)
