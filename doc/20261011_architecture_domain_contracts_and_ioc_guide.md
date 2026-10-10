# Guide Architectural : Inversion de Contrôle (IoC) & Contrats de Domaine (domain_contracts.rs)

Ce document détaille les fondements architecturaux, le modèle de conception et le guide d'utilisation du module `src/domain_contracts.rs`, pivot de l'architecture découplée du simulateur de feux d'artifice.

---

## 1. Contexte & Problématique Legacy

Avant le grand chantier de refactoring architectural (Phases 0 à 3), le code souffrait d'un couplage fort et bidirectionnel :
1. **Couplage Circulaire & Spaghetti d'Imports** : L'interface utilisateur ImGui (`gui_settings`), la console interactive (`command_console`) et l'orchestrateur (`simulator`) importaient directement les modules internes concrets des moteurs (`PhysicEngine`, `AudioEngine`, `Renderer`). Modifier une passe OpenGL ou un calcul physique déclenchait la recompilation de l'UI, de la console et de toute l'arborescence.
2. **Mutations Directes Hors-Thread** : L'interface graphique mutait directement l'état interne des moteurs (ex: `audio_engine.volume = 0.5;`), en violation des contraintes de sûreté concurrente du thread audio CPAL temps réel.
3. **Complexité de Test (Testabilité Réduite)** : Impossible d'écrire des tests unitaires rapides pour l'UI ou la console sans instancier l'intégralité du contexte GLFW/OpenGL et le serveur son hôte.

---

## 2. Le Paradigme : Inversion de Contrôle (IoC) & 0 Import Amont

Pour assainir durablement l'architecture, le module `src/domain_contracts.rs` a été positionné **à la base stricte de l'arbre de dépendances**.

### La Règle d'Or des Zéro Imports Amont

`domain_contracts.rs` ne dépend que de types de base standards (`glam::Vec2` et `serde`). **Il a l'interdiction formelle et absolue d'importer les modules moteurs amont** (`crate::audio_engine`, `crate::physic_engine`, `crate::renderer_engine`, `crate::simulator`).

Ce sont les moteurs qui importent `domain_contracts.rs` et implémentent ses contrats :

```text
       ┌──────────────┐     ┌──────────────┐     ┌─────────────────┐
       │ AudioEngine  │     │ PhysicEngine │     │ RendererEngine  │
       └──────┬───────┘     └──────┬───────┘     └────────┬────────┘
              │                    │                      │
              │ implémente         │ implémente           │ implémente
              ▼                    ▼                      ▼
    ┌──────────────────────────────────────────────────────────────┐
    │                     domain_contracts.rs                      │
    │  - Traits d'abstraction : AudioEngine, PhysicEngine          │
    │  - DTOs & Enums purs : AudioEffect, BlurMethod, ToneMapping  │
    │  - Commandes CQRS : AudioCommand, PhysicCommand, RendererCmd │
    │  - Snapshots de configuration : PhysicConfigSnapshot, etc.   │
    └──────────────────────────────────────────────────────────────┘
              ▲                    ▲                      ▲
              │ émet / lit         │ émet / lit           │ émet / lit
       ┌──────┴───────┐     ┌──────┴───────┐              │
       │   ImGui UI   │     │ CLI Console  │              │
       └──────────────┘     └──────────────┘              │
              ▲                                           │
              └────────────── Simulator (Orchestration) ──┘
```

---

## 3. Modèle CQRS & Flux de Données

Le système applique rigoureusement le pattern **Command Query Responsibility Segregation (CQRS)** :

### Flux d'Écriture (Commandes)
L'UI et la console ne disposent d'aucun accès mutable direct aux moteurs. Toute modification passe par l'émission d'une commande dans une file thread-safe `cmd_queue` :

```rust
// Dans l'UI ou la console : zéro pointeur concret, zéro allocation
cmd_queue.push(EngineCommand::Renderer(
    RendererCommand::SetVolumetricLightingEnabled(true),
));
```

Au début de chaque frame, le `Simulator` dépile la file et distribue les commandes :

```rust
while let Some(cmd) = self.cmd_queue.pop() {
    match cmd {
        EngineCommand::Audio(cmd) => self.audio_engine.apply_command(cmd),
        EngineCommand::Physic(cmd) => self.physic_engine.apply_command(cmd),
        EngineCommand::Renderer(cmd) => self.renderer.apply_command(cmd),
        EngineCommand::Smoke(cmd) => self.physic_engine.apply_smoke_command(cmd),
        EngineCommand::Gui(cmd) => self.apply_gui_command(cmd),
    }
}
```

### Flux de Lecture (Queries & Snapshots)
Pour afficher l'état courant dans les widgets ImGui :
- **Snapshots Immutables** : `PhysicConfigSnapshot`, `RendererConfigSnapshot`, `SmokeConfigSnapshot`.
- **Traits de Lecture Pure** : Trait `AudioStateReader` permettant à l'UI de consulter les paramètres audio (volume, mute, status DSP) sans dépendre de l'implémentation concrète du moteur CPAL.

---

## 4. Anatomie du Module `domain_contracts.rs`

Le fichier regroupe 4 catégories de structures pures :

1. **Enums & DTOs Transverses** :
   - `AudioEffect` : Masque de bits `u32` représentant les 11 effets DSP spatiaux.
   - `ToneMappingMode` : Opérateurs de Tone Mapping (`ACES`, `AgX`, `KhronosPBR`, `Reinhard`, `Uncharted2`).
   - `BlurMethod` : Méthodes de flou du Bloom (`Gaussian`, `Kawase`).
   - `SmokeColorMode` : Mode de coloration de la fumée (`RocketColor`, `Custom`).
   - `AudioSoundType` & `AudioDebugEvent` : Télémétrie d'événements audio.
2. **Traits d'Abstraction Moteurs** :
   - `AudioEngine` : Interface publique du moteur sonore (déclenchement de sons, gestion du listener, paramétrage DSP).
   - `PhysicEngine` : Interface publique de simulation de particules.
   - `AudioStateReader` : Contrat de lecture seule pour découpler l'UI.
3. **Commandes CQRS (`Copy`, Zéro Allocation)** :
   - `AudioCommand`, `PhysicCommand`, `RendererCommand`, `SmokeCommand`, `GuiCommand`.
   - Regroupées dans l'enum racine `EngineCommand`.
4. **Snapshots de Configuration** :
   - `PhysicConfigSnapshot`, `RendererConfigSnapshot`, `SmokeConfigSnapshot` sérialisables via Serde sans état d'exécution.

---

## 5. Politique Zéro-Allocation & Performance Temps Réel

Le design de `domain_contracts.rs` respecte scrupuleusement les contraintes de performance du simulateur :

- **Commandes `Copy` Stack-Allocated** : Tous les variants des enums `*Command` contiennent uniquement des types scalaires primitifs (`f32`, `u32`, `bool`, `[f32; 2]`). Aucun `String`, `Box` ou `Vec` n'est alloué lors de l'émission ou du dispatch d'une commande.
- **Taille Mémoire Prévisible** : `EngineCommand` est une énumération compacte tenant sur quelques octets de pile, garantissant une utilisation optimale du cache L1 lors du parcours de la file de commandes.
- **Isolation du Thread Audio** : Le thread audio CPAL lit des atomiques lock-free (`AudioRealtimeStats`) et reçoit ses instructions de manière asynchrone, éliminant tout risque d'underrun causé par un blocage de l'UI.

---

## 6. Guide Pratique : Comment Ajouter un Nouveau Réglage Moteur

Pour ajouter un nouveau réglage (ex: un paramètre d'intensité lumineuse `light_intensity: f32` sur le Renderer), suivre la séquence standard en 4 étapes :

### Étape 1 : Ajouter la commande dans `domain_contracts.rs`
```rust
// src/domain_contracts.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererCommand {
    // ...
    SetLightIntensity(f32),
}
```

### Étape 2 : Ajouter le champ dans le Snapshot de configuration
```rust
// src/domain_contracts.rs
pub struct RendererConfigSnapshot {
    // ...
    pub light_intensity: f32,
}
```

### Étape 3 : Implémenter l'application dans le moteur concerné
```rust
// src/renderer_engine/mod.rs (ou renderer_graphics.rs)
impl Renderer {
    pub fn apply_command(&mut self, cmd: RendererCommand) {
        match cmd {
            // ...
            RendererCommand::SetLightIntensity(val) => {
                self.config.light_intensity = val;
            }
        }
    }
}
```

### Étape 4 : Émettre la commande depuis l'UI ou la console
```rust
// src/simulator/gui_settings/renderer.rs
if ui.slider("Light Intensity", 0.0, 10.0, &mut local_val) {
    cmd_queue.push(EngineCommand::Renderer(
        RendererCommand::SetLightIntensity(local_val),
    ));
}
```

---

## 7. Matrice des Bénéfices & Compromis (Trade-Offs)

| Dimension | Bénéfice (Gain Ingénierie) | Coût / Compromis |
|---|---|---|
| **Compilation** | **Gain majeur** : Recompilation incrémentale ultra-rapide des modules indépendants. | Nécessite de centraliser les types partagés dans un fichier commun. |
| **Sûreté Concurrente** | **Absolue** : Aucune data-race possible entre UI ImGui et threads moteurs. | Légère latence d'une frame (les commandes sont appliquées au tick suivant). |
| **Testabilité** | **100% Mockable** : Écriture de tests unitaires purs pour l'UI et la CLI sans GLFW ni OpenGL. | Nécessite d'implémenter les traits abstraits sur des mocks de test. |
| **Maintenance** | **Lisibilité CQRS** : Séparation nette entre logique métier, rendu et interface. | Verbosité accrue : chaque action nécessite un variant de commande et son handler. |

---

## 8. Contrôle Automatisé en CI (Pre-Commit Gate)

Pour garantir qu'aucun import amont ne vienne briser l'inversion de contrôle lors d'un refactoring futur, la vérification suivante est exécutée :

```bash
# Vérifie l'absence absolue de dépendance montante (doit retourner 0 résultat)
git grep "crate::audio_engine\|crate::physic_engine\|crate::renderer_engine" src/domain_contracts.rs
```

Tout commit introduisant une référence directe vers un moteur amont au sein de `src/domain_contracts.rs` doit être rejeté.
