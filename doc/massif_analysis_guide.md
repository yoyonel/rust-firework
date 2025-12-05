# Guide d'Analyse Massif - Profiling Mémoire

## Analyse Rapide avec `ms_print`

### Commande de base
```bash
ms_print massif.out.12345 | less
```

### Sortie typique et interprétation

```
--------------------------------------------------------------------------------
  n        time(i)         total(B)   useful-heap(B) extra-heap(B)    stacks(B)
--------------------------------------------------------------------------------
  0              0                0                0             0            0
  1        234,567        1,048,576          983,040        65,536            0
  2        456,789        2,097,152        1,966,080       131,072            0
...
 50     10,000,000       10,485,760        9,830,400       655,360            0  <- Peak
```

**Colonnes importantes** :
- `time(i)` : Instructions exécutées (pas du temps réel)
- `total(B)` : Mémoire totale allouée (heap + extra + stacks)
- `useful-heap(B)` : Mémoire réellement utilisée par votre code
- `extra-heap(B)` : Overhead de l'allocateur (fragmentation, metadata)

---

## Identifier le Peak Memory

```bash
# Trouver le pic de mémoire
ms_print massif.out.12345 | grep -A 20 "peak"
```

**Exemple de sortie** :
```
99.99% (10,485,760B) (heap allocation functions)
->50.00% (5,242,880B) 0x4A2B3C: Vec::with_capacity
| ->50.00% (5,242,880B) 0x4A2C4D: ParticlesPool::new
|   ->50.00% (5,242,880B) 0x4A3E5F: PhysicEngineFireworks::new
```

**Interprétation** :
- **50%** de la mémoire vient de `ParticlesPool::new` → Normal (pré-allocation)
- **25%** de `Arena::with_capacity` → Normal aussi
- **24.99%** de `Vec::push` dans une boucle → **⚠️ Potentiel problème !**

---

## Visualiser avec Massif-Visualizer (GUI)

```bash
sudo apt install massif-visualizer
massif-visualizer massif.out.12345
```

---

## Script d'Analyse Automatique

```bash
#!/bin/bash
# analyze_massif.sh

MASSIF_FILE=$1

echo "=== Massif Analysis ==="
echo ""

# Peak memory
echo "📊 Peak Memory:"
ms_print "$MASSIF_FILE" | grep -A 1 "peak" | head -2
echo ""

# Top 5 allocations
echo "🔝 Top 5 Allocations:"
ms_print "$MASSIF_FILE" | grep -E "^\->[0-9]+\.[0-9]+%" | head -5
echo ""

# Memory trend
echo "📈 Memory Trend (last 10 snapshots):"
ms_print "$MASSIF_FILE" | grep -E "^[[:space:]]*[0-9]+" | tail -10
```

---

## Commandes Essentielles

```bash
# 1. Profiler
valgrind --tool=massif ./target/release/fireworks_sim

# 2. Analyser
ms_print massif.out.* | less

# 3. Trouver le pic
ms_print massif.out.* | grep -A 20 "peak"

# 4. Visualiser
massif-visualizer massif.out.*
```
