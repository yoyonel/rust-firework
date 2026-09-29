# Rapport Technique FIX-05 : Sample-Accurate Audio Scheduling

- **Date :** 29 Septembre 2026
- **Branche :** `feat/audio-sample-accurate-scheduling`
- **Statut :** Validé (Pre-commit / Pre-PR)
- **Domaine :** Moteur Audio DSP (`src/audio_engine/`), Synchronisation Simulation-Audio

---

## 1. Rationnel & Architecture

### 1.1 Problématique Initiale
Avant ce chantier, les requêtes audio émises depuis le simulateur ou le moteur physique (`play_rocket`, `play_explosion`) étaient consommées de manière asynchrone dès le début du bloc DSP suivant :
- **Jitter intra-bloc :** Un son déclenché au milieu d'un bloc DSP de 512 échantillons (~10.67 ms à 48 kHz) débutait invariablement à l'index 0 du bloc traité, introduisant un jitter temporel aléatoire allant jusqu'à la durée d'un bloc complet.
- **Désynchronisation physique/audio :** Le simulateur calculait un délai d'anticipation physique sub-frame mais ne pouvait pas transmettre d'échéance temporelle exacte au moteur audio.
- **Absence d'horloge absolue :** Aucun compteur d'échantillons absolu n'était partagé de manière lock-free entre le thread audio temps réel (CPAL) et le thread principal (simulateur).

### 1.2 Conception Mathématique & Architecture Implémentée

#### Horloge Échantillon & Pointeur Atomique
- **Compteur DSP interne :** `current_sample_clock: u64` incrémenté exactement de `frames` à chaque fin de bloc dans `DspProcessor::process_block`.
- **Miroir Atomique Lock-Free :** `sample_clock: Arc<AtomicU64>` mis à jour avec `Ordering::Relaxed` à la fin de chaque bloc. Le simulateur peut lire cette horloge sans verrou ni contention via `AudioEngine::current_sample_clock()`.

#### Calcul de l'Échantillon Cible ($N_{\text{target}}$)
Pour un son devant retentir avec un délai d'anticipation ou une projection physique de $\Delta t_{\text{ms}}$ :
$$N_{\text{target}} = N_{\text{current\_sample}} + \left\lfloor \frac{\Delta t_{\text{ms}} \times f_s}{1000} \right\rfloor$$
où $f_s$ est la fréquence d'échantillonnage audio (ex: 48 000 Hz).

#### Planification Intra-Bloc & Rendu Précis
Dans `DspProcessor::consume_requests` :
1. Les requêtes en attente sont triées par `target_sample` ascendant.
2. Pour chaque requête arrivant à échéance ($N_{\text{target}} \le N_{\text{block\_end}}$) :
   - Si $N_{\text{target}} < N_{\text{block\_start}}$ : la requête est échue (en retard).
     - Si le retard dépasse `MAX_SCHEDULED_SOUND_LATENESS_MS = 200.0` ms, elle est purgée gracieusement pour éviter les accumulations toxiques d'explosions post-freeze.
     - Sinon, elle est activée immédiatement avec `start_offset = 0`.
   - Si $N_{\text{block\_start}} \le N_{\text{target}} < N_{\text{block\_end}}$ :
     - `start_offset = (N_target - N_block_start) as usize`.
3. Le silence est préservé sur les tranches $[0 .. \text{start\_offset})$, et le mixage/rendu démarre exactement à l'index `start_offset`.
4. Après le premier bloc partiel, `voice.start_offset` est réinitialisé à 0 pour la suite de la lecture continue.

#### Ring Buffer Borné & Protection Surcharge
- Le canal de requêtes utilise une capacité bornée SSOT `PLAY_REQUEST_CHANNEL_CAPACITY = 512`.
- Si le producteur sature le buffer (ex: 1000 requêtes consécutives), l'appel `try_send` rejette immédiatement le surplus avec `AudioDebugEvent::Dropped` sans jamais bloquer le thread principal ni allouer sur le heap.

---

## 2. Preuves & Validation

### 2.1 Tests Unitaires DSP Ciblés (`src/audio_engine/dsp_processor/tests.rs`)
- `test_sample_accurate_audio_scheduling` :
  - Son planifié à $N_{\text{target}} = 300$ avec bloc de 256 samples.
  - Bloc 0 ($[0, 256)$) : 100% silencieux.
  - Bloc 1 ($[256, 512)$) : samples $0..44$ rigoureusement silencieux (`0.0`), samples $44..256$ actifs ($|s| > 0.1$).
- `test_overdue_scheduled_audio_discarded` :
  - Son en retard de plus de 200 ms (seuil anti-accumulation).
  - Rejet gracieux confirmé sans fuite mémoire ni bruit numérique.

### 2.2 Test d'Intégration & Saturation QA (`tests/audio_queue_saturation_test.rs`)
- `test_audio_queue_saturation_1000_requests_non_blocking` :
  - 1000 requêtes consécutives envoyées sans consommateur.
  - Temps total d'exécution : **0.02 ms** (exigence : $< 50$ ms).
  - 512 requêtes acceptées dans le ring buffer, 488 requêtes rejetées gracieusement.
  - Zéro blocage, zéro panic.
- `test_dsp_drain_saturated_queue_no_leak_and_no_nan` :
  - Saturation complète à 512 requêtes puis drain sur 20 blocs DSP consécutifs.
  - Horloge validée à $20 \times 512 = 10\,240$ samples.
  - Zéro NaN, zéro Inf, zéro fuite mémoire.

### 2.3 Validation Concurrence & Sanitizers (TSan)
```
RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test -Z build-std --test audio_queue_saturation_test --target x86_64-unknown-linux-gnu
test result: ok. 2 passed; 0 failed; 0 ignored; finished in 0.54s
```
Zéro data race, atomicité lock-free confirmée sous ThreadSanitizer.

### 2.4 Matrice de Validation Globale
| Suite de Tests | Commande | Résultat |
| :--- | :--- | :--- |
| Tests Unitaires & Intégration | `task test:all` | **92 unit tests + 44 integration tests PASS** |
| Validation OpenGL Mesa Headless | `task test:opengl-mesa` | **0 violation OpenGL** |
| Persistance UI ImGui | `task test:gui-persistence-check` | **11 inventory rows PASS** |
| Linting & Formatage | `task lint:all` | **Clippy strict 0 warnings, fmt PASS, vale PASS** |

---

## 3. Runbook de Reproductibilité Humaine

Commandes exactes pour reproduire l'audit et la validation du correctif :

```bash
# 1. Validation du test de saturation de queue audio (1000 requêtes)
cargo test --test audio_queue_saturation_test -- --nocapture

# 2. Validation unitaire du scheduling sub-bloc sample-accurate
task test:one -- test_sample_accurate

# 3. Validation de la sûreté concurrente sous ThreadSanitizer (Rust Nightly requis)
RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test -Z build-std --test audio_queue_saturation_test --target x86_64-unknown-linux-gnu

# 4. Exécution de la suite complète de non-régression
task test:all

# 5. Validation de la conformité OpenGL headless Mesa
task test:opengl-mesa

# 6. Vérification statique complète (Clippy strict, formatage, doc linting)
task lint:all
```
