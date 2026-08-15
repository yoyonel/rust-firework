# Système de Benchmark des Ratios de Profilage Tracy (Zero-Timestamp)

**Date :** 05 août 2026  
**Auteur :** Antigravity AI Agent  
**Branche :** [`feat/tracy-ratio-benchmarking`](file:///home/latty/Prog/__PERSO__/rust-firework)

---

## 1. Contexte & Problématique

Les mesures absolues de profilage temporel (ex: millisecondes, nanosecondes par frame) présentent une forte instabilité causée par la fréquence dynamique des processeurs (Nvidia Boost, CPU Scaling), le Thermal Throttling de la machine hôte et la charge réseau/système en arrière-plan.

Afin d'établir un indicateur de performance (KPI) **invariant du matériel et de la charge hôte**, ce projet introduit un système de benchmarking basé sur les **proportions relatives (%) et ratios inter-zones** issus des captures [Tracy Profiler](file:///home/latty/Prog/__PERSO__/suckless-ogl/deps/tracy).

---

## 2. Découpage Canonique et Invariants Mathématiques

Le parser POSIX [`scripts/analyze_tracy_ratios.sh`](file:///home/latty/Prog/__PERSO__/rust-firework/scripts/analyze_tracy_ratios.sh) sépare le **Thread Principal** (Physics, Renderer, UI) du **Thread Audio Asynchrone** pour garantir une observabilité déterministe.

### 2.1 Ratios du Thread Principal (Main Thread Core)
Le temps de frame cœur est défini par :
$$T_{frame\_main} = T_{physics} + T_{renderer} + T_{ui}$$

1. **Part Moteur Physique (%)** : $\frac{T_{physics}}{T_{frame\_main}} \times 100$
2. **Part Moteur Rendu (%)** : $\frac{T_{renderer}}{T_{frame\_main}} \times 100$
3. **Part Interface ImGui (%)** : $\frac{T_{ui}}{T_{frame\_main}} \times 100$

### 2.2 Sous-Passes du Rendu Graphique (% de Renderer)
1. **Pass HDR Scene (%)** : $\frac{T_{hdr}}{T_{renderer}} \times 100$
2. **Pass Bloom & Composite (%)** : $\frac{T_{bloom}}{T_{renderer}} \times 100$
3. **Draw All Particles (%)** : $\frac{T_{particles}}{T_{renderer}} \times 100$

### 2.3 Ratios Inter-Zones
* **Ratio Physique / Rendu** : $R_{phys\_rend} = \frac{T_{physics}}{T_{renderer}}$
* **Ratio Bloom / HDR** : $R_{bloom\_hdr} = \frac{T_{bloom}}{T_{hdr}}$
* **Part Doppler Audio / Audio Bus** : $R_{doppler\_audio} = \frac{T_{audio\_doppler}}{T_{audio}}$

---

## 3. Seuils de Tolérance (Règle Zero-Trust)

Le système applique un double verrou d'évaluation :
* **Dérive Absolue en Points de % ($\Delta \%$)** : Tolérance maximale de $\pm 6.0\%$ par rapport à la baseline `develop`.
* **Facteur de Augmentation Relatif Inter-Zone ($R_{diff}$)** : Tolérance maximale de $1.20\times$ (+20% de dérive relative de proportion).

En cas de dépassement sur une quelconque métrique, le script retourne un exit code `1` et la tâche Taskfile échoue.

---

## 4. Runbook de Reproductibilité & CLI

### Génération de la Baseline de Référence (`develop`)
```bash
task tracy:generate-baseline
```
Enregistre la baseline dans [`benches/baselines/tracy_ratios_develop.csv`](file:///home/latty/Prog/__PERSO__/rust-firework/benches/baselines/tracy_ratios_develop.csv).

### Audit Comparatif des Ratios
```bash
task bench:tracy-ratios
```
Exécute une capture headless Tracy de 5s et produit le rapport comparatif ANSI :

```text
================================================================
📊 AUDIT COMPARATIF DES RATIOS TRACY PROFILER (ZERO-TIMESTAMP)
================================================================
METRIQUE             | BASELINE     | ACTUEL       | DIFF (PTS/%/RATIO) | STATUT  
----------------------------------------------------------------
perc_physics         | 21.29       % | 21.97       % |             +0.68% | OK      
perc_renderer        | 78.52       % | 77.87       % |             -0.65% | OK      
perc_ui              | 0.18        % | 0.15        % |             -0.03% | OK      
perc_hdr             | 78.91       % | 79.78       % |             +0.87% | OK      
perc_bloom           | 20.06       % | 19.20       % |             -0.86% | OK      
perc_particles       | 76.77       % | 77.61       % |             +0.84% | OK      
perc_audio_doppler   | 1.61        % | 1.93        % |             +0.32% | OK      
r_phys_rend          | 0.2712       | 0.2822       |              1.04x | OK      
r_bloom_hdr          | 0.2542       | 0.2407       |              0.95x | OK      
r_doppler_audio      | 0.0161       | 0.0193       |              1.20x | OK      
================================================================
🟢 SUCCÈS : Les proportions relatives du pipeline sont stables et conformes à la baseline.
```

---

## 5. Intégration Taskfile & CI/CD GitHub Actions

### 5.1 Tâches Taskfile
Trois nouvelles tâches sont configurées dans [`Taskfile.yml`](file:///home/latty/Prog/__PERSO__/rust-firework/Taskfile.yml) :
* `tracy:capture-headless` : Lancement du simulateur et capture de 5 secondes avec `tracy-capture`.
* `bench:tracy-ratios` : Audit comparatif des proportions relatives contre la baseline `develop` et génération du rapport `/tmp/tracy_pr_comment.md`.
* `tracy:generate-baseline` : Génération et écriture du fichier CSV de référence `benches/baselines/tracy_ratios_develop.csv`.

### 5.2 Workflow GitHub Actions ([`.github/workflows/ci.yml`](file:///home/latty/Prog/__PERSO__/rust-firework/.github/workflows/ci.yml))
À chaque ouverture ou modification de Pull Request :
1. **Compilation des outils CLI Tracy** : Les binarisés `tracy-capture` et `tracy-csvexport` sont compilés depuis les sources (version v0.11.1) et mis en cache via `actions/cache@v4`.
2. **Exécution du Benchmark Headless** : La commande `xvfb-run -a task bench:tracy-ratios` est exécutée sous Xvfb.
3. **Publication du Rapport sur la PR** : L'action `mshick/add-pr-comment@v3` publie ou met à jour dynamiquement un commentaire Markdown structuré (`message-id: tracy-ratio-benchmark`) sans spamer le fil de discussion.

