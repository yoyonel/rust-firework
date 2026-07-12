## 8. PHASE 2 : IMPLÉMENTATION DU PROFILING GPU MATÉRIEL (AZDO)

Pour obtenir les temps d'exécution réels du silicium sans impacter le framerate, un moteur de profiling OpenGL asynchrone a été développé en Rust.

* **Architecture Double-Bufferisée (Ring-Buffer) :** Création d'un pool `GpuProfiler` alternant entre deux `GpuQueryBuffer`. Pendant que la Frame $N$ enregistre ses timestamps (`glQueryCounter`), le CPU récolte les résultats de la Frame $N-1$ (`glGetQueryObjectui64v`).
* **Zéro Blocage (AZDO) :** Avant toute lecture, le driver est interrogé via `gl::QUERY_RESULT_AVAILABLE`. Si le GPU n'a pas terminé, le CPU abandonne la récolte pour cette frame au lieu de staller le pipeline.
* **Intégration RAII & Thread-Safe :** Le profiler est encapsulé dans un `Arc<Mutex<>>`. La macro unifiée `gpu_profile_zone!` déploie un garde RAII qui s'occupe simultanément du `Push/PopDebugGroup` (RenderDoc) et du `start/end_stage` (Timestamps GPU).
* **Anti-Flood Temporel :** Échantillonnage des logs consoles (intervalle de 2s) via `std::time::Instant` pour maintenir un terminal lisible à très haut framerate.
