# Références Architecturales : Persistent Mapping, Cache Coherency & Write-Combining

Ce document regroupe les fondations théoriques, spécifications officielles, articles de recherche et blogs d'ingénierie système qui justifient l'utilisation du **Write-Combining** et du **Persistent Mapping (AZDO)** dans notre moteur de rendu OpenGL.

---

## 📚 Spécifications Officielles & Normes

### 1. Spécification OpenGL 4.6 (Khronos Group)
* **Description :** La spécification officielle décrivant le fonctionnement des buffers persistants (`BufferStorage`) et les garanties de visibilité mémoire entre le CPU et le GPU.
* **Lien de Référence :** [OpenGL 4.6 Core Profile Specification](https://registry.khronos.org/OpenGL/specs/gl/glspec46.core.pdf) (voir Section 6.3, *"Mapping and Unmapping Buffer Data"*).
* **Concept Clé :** Décrit pourquoi l'absence de `MAP_COHERENT_BIT` impose l'usage de barrières d'écriture (`glFlushMappedBufferRange`), permettant en contrepartie d'éviter la pénalité de synchronisation matérielle continue sur le bus PCIe (snooping).

### 2. Guide d'Architecture Intel (Intel SDM)
* **Description :** Le manuel de référence pour les développeurs système détaillant le comportement des caches processeur, de l'ordonnancement de la mémoire et des types de cache.
* **Lien de Référence :** [Intel® 64 and IA-32 Architectures Software Developer’s Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) (voir Volume 3A, Chapitre 11, *"Memory Cache Control"*).
* **Concept Clé :** Définit le type de mémoire **Write-Combining (WC)**. Les écritures CPU vers la VRAM sont accumulées dans des Write-Combining Buffers internes de 64 octets (Line Fill Buffers). Une fois pleins, ils sont envoyés d'un seul bloc (Burst Transaction) à travers le bus PCIe, multipliant la vitesse de transfert par rapport à l'écriture non-mise en cache (Uncached Coherent / UC).

---

## 🎤 Conférences & Littérature de l'Industrie

### 1. Approaching Zero Driver Overhead (AZDO) - GDC 2014
* **Auteurs :** Cass Everitt (NVIDIA), John McDonald (NVIDIA), Graham Sellers (AMD), Tim Foley (Intel).
* **Lien de Référence :** [AZDO Slides (Khronos Group)](https://www.khronos.org/assets/uploads/apis/2014-gdc-azdo-slides.pdf)
* **Concept Clé :** Cette présentation de l'industrie pose les bases du rendu moderne haute performance. Elle explique explicitement pourquoi `glBufferStorage` couplé à `glMapBufferRange(MAP_FLUSH_EXPLICIT_BIT)` est la méthode optimale pour transférer dynamiquement de la géométrie sans subir les latences de validation internes du driver ou les baisses de régime de la cohérence matérielle du bus PCIe.

### 2. OpenGL SuperBible (7th Edition)
* **Auteur :** Graham Sellers.
* **Concept Clé :** Le chapitre 12 (*"High-Performance OpenGL"*) est entièrement dédié aux techniques AZDO, à la triple-bufferisation et à la gestion de la BAR PCIe. Il détaille la désactivation de la cohérence automatique au profit de flushes restreints par plage (Explicit Flushing) pour maximiser le débit d'écriture du processeur.

---

## ✍️ Blogs d'Ingénierie Système de Référence

### 1. "Write combining is not your friend" — Fabian Giesen (ryg)
* **Auteur :** Fabian Giesen (ingénieur système/rendu chez RAD Game Tools / Epic Games, créateur de démos légendaires).
* **Lien de Référence :** [Write combining is not your friend (The ryg blog)](https://fgiesen.wordpress.com/2013/01/29/write-combining-is-not-your-friend/)
* **Concept Clé :** Explication physique très détaillée du fonctionnement des Write-Combining Buffers du CPU. L'article démontre pourquoi les écritures doivent être strictement séquentielles (ce que notre Piste 3 d'alignement de structure permet) et pourquoi toute lecture dans une zone mémoire mappée en WC détruit les performances en forçant le vidage prématuré et coûteux des tampons d'écriture.

### 2. "Cache coherency primer" — Fabian Giesen (ryg)
* **Auteur :** Fabian Giesen.
* **Lien de Référence :** [Cache coherency primer (The ryg blog)](https://fgiesen.wordpress.com/2014/07/07/cache-coherency-primer/)
* **Concept Clé :** Présentation des protocoles de cohérence de cache processeur (MESI et variantes). L'article permet de comprendre la complexité et le coût en bande passante des opérations de "snooping" de bus et d'invalidation de ligne de cache lorsque l'on tente de synchroniser automatiquement le CPU et le GPU externe à travers le bus PCIe.
