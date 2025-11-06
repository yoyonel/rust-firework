use log::info;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Valeur de métrique typée (soit f32 soit usize)
#[derive(Debug)]
pub enum MetricValue {
    Usize(usize),
    F32(f32),
    Duration(Duration),
}

impl From<usize> for MetricValue {
    fn from(v: usize) -> Self {
        MetricValue::Usize(v)
    }
}
impl From<f32> for MetricValue {
    fn from(v: f32) -> Self {
        MetricValue::F32(v)
    }
}
impl From<Duration> for MetricValue {
    fn from(v: Duration) -> Self {
        MetricValue::Duration(v)
    }
}

impl fmt::Display for MetricValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricValue::Usize(u) => write!(f, "{}", u),
            MetricValue::F32(v) => write!(f, "{:.2}", v),
            MetricValue::Duration(d) => write!(f, "{:.2?}", d),
        }
    }
}

/// Données internes du profiler
pub struct ProfilerInner {
    pub samples: HashMap<String, Vec<f32>>, // Durées RAII / profile_block
    pub metrics: HashMap<String, Vec<MetricValue>>, // Valeurs scalaires typées
    pub max_samples: usize,
    pub total_frame_times: Vec<f32>,
}

/// Profiler partagé et thread-safe
#[derive(Clone)]
pub struct Profiler {
    inner: Arc<RwLock<ProfilerInner>>,
}

impl Profiler {
    pub fn new(max_samples: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ProfilerInner {
                samples: HashMap::new(),
                metrics: HashMap::new(),
                max_samples,
                total_frame_times: Vec::with_capacity(max_samples),
            })),
        }
    }

    /// Mesure globale d'une frame (RAII)
    pub fn frame(&self) -> FrameGuard {
        FrameGuard {
            profiler: self.clone(),
            start: Instant::now(),
        }
    }

    /// Mesure d'un bloc labelisé (RAII)
    pub fn measure<'a>(&'a self, label: impl Into<String>) -> MeasureGuard<'a> {
        MeasureGuard {
            profiler: self,
            label: label.into(),
            start: Instant::now(),
        }
    }

    /// Enregistre une métrique scalaire typée
    pub fn record_metric<T: Into<MetricValue>>(&self, label: impl Into<String>, value: T) {
        let label = label.into();
        let mut inner = self.inner.write().unwrap();
        let max_samples = inner.max_samples;
        let buffer = inner.metrics.entry(label).or_default();
        if buffer.len() >= max_samples {
            buffer.remove(0);
        }
        buffer.push(value.into());
    }

    /// Retourne le FPS moyen
    pub fn fps(&self) -> f32 {
        let inner = self.inner.read().unwrap();
        if inner.total_frame_times.is_empty() {
            return 0.0;
        }
        let avg =
            inner.total_frame_times.iter().sum::<f32>() / inner.total_frame_times.len() as f32;
        1000.0 / avg
    }

    /// Nombre total de frames enregistrées
    pub fn total_frames(&self) -> f32 {
        let inner = self.inner.read().unwrap();
        inner.total_frame_times.len() as f32
    }

    /// Résumé des temps mesurés (moyenne, min, max)
    pub fn summary(&self) -> HashMap<String, (f32, f32, f32)> {
        let inner = self.inner.read().unwrap();
        summarize_map(&inner.samples)
    }

    /// Résumé des métriques scalaires (moyenne, min, max)
    pub fn metrics_summary(&self) -> HashMap<String, (MetricValue, MetricValue, MetricValue)> {
        let inner = self.inner.read().unwrap();
        inner
            .metrics
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.clone(), summarize_metric(v)))
            .collect()
    }

    /// Résumé pour une métrique spécifique
    pub fn metric_summary(&self, label: &str) -> Option<(MetricValue, MetricValue, MetricValue)> {
        let inner = self.inner.read().unwrap();
        inner.metrics.get(label).map(|v| summarize_metric(v))
    }

    /// Profile un bloc de code et retourne sa valeur de retour
    pub fn profile_block<T, F>(&self, label: impl Into<String>, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let label = label.into();
        let start = Instant::now();
        let result = f();
        let dt = start.elapsed().as_secs_f32() * 1000.0;

        let mut inner = self.inner.write().unwrap();
        let max_samples = inner.max_samples;
        let samples = inner.samples.entry(label).or_default();
        if samples.len() >= max_samples {
            samples.remove(0);
        }
        samples.push(dt);

        result
    }
}

/// Résumé pour les valeurs f32
fn summarize_series(series: &[f32]) -> (f32, f32, f32) {
    let avg = series.iter().sum::<f32>() / series.len() as f32;
    let min = series.iter().cloned().fold(f32::MAX, f32::min);
    let max = series.iter().cloned().fold(f32::MIN, f32::max);
    (avg, min, max)
}

/// Fonction générique pour `usize`/Usize et `f32`
fn summarize_numeric<T, F, G>(
    series: &[MetricValue],
    extract: F,
    wrap: G,
) -> (MetricValue, MetricValue, MetricValue)
where
    T: Copy + PartialOrd + std::ops::Add<Output = T> + From<u8> + std::ops::Div<Output = T>,
    F: Fn(&MetricValue) -> T,
    G: Fn(T) -> MetricValue,
{
    if series.is_empty() {
        return (wrap(T::from(0)), wrap(T::from(0)), wrap(T::from(0)));
    }

    let mut sum = T::from(0);
    let mut min = extract(&series[0]);
    let mut max = extract(&series[0]);

    for v in series {
        let x = extract(v);
        sum = sum + x;
        if x < min {
            min = x;
        }
        if x > max {
            max = x;
        }
    }

    let avg = sum / T::from(series.len() as u8);

    (wrap(avg), wrap(min), wrap(max))
}

/// Résumé pour une série de MetricValue
pub fn summarize_metric(series: &[MetricValue]) -> (MetricValue, MetricValue, MetricValue) {
    use MetricValue::*;

    match &series[0] {
        Usize(_) => summarize_numeric(
            series,
            |v| match v {
                Usize(u) => *u,
                _ => 0,
            },
            MetricValue::Usize,
        ),
        F32(_) => summarize_numeric(
            series,
            |v| match v {
                F32(f) => *f,
                _ => 0.0,
            },
            F32,
        ),
        Duration(_) => {
            let mut min = std::time::Duration::new(u64::MAX, 0);
            let mut max = std::time::Duration::new(0, 0);

            for v in series {
                if let Duration(d) = v {
                    min = min.min(*d);
                    max = max.max(*d);
                }
            }
            let avg = (min + max) / 2;
            (Duration(avg), Duration(min), Duration(max))
        }
    }
}

/// Résumé pour une map f32
fn summarize_map(map: &HashMap<String, Vec<f32>>) -> HashMap<String, (f32, f32, f32)> {
    map.iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (k.clone(), summarize_series(v)))
        .collect()
}

/// Mesure globale d'une frame
pub struct FrameGuard {
    profiler: Profiler,
    start: Instant,
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        let dt = self.start.elapsed().as_secs_f32() * 1000.0;
        let mut inner = self.profiler.inner.write().unwrap();
        if inner.total_frame_times.len() >= inner.max_samples {
            inner.total_frame_times.remove(0);
        }
        inner.total_frame_times.push(dt);
    }
}

/// Mesure d’un bloc labelisé (RAII)
pub struct MeasureGuard<'a> {
    profiler: &'a Profiler,
    label: String,
    start: Instant,
}

impl<'a> Drop for MeasureGuard<'a> {
    fn drop(&mut self) {
        let dt = self.start.elapsed().as_secs_f32() * 1000.0;
        let mut inner = self.profiler.inner.write().unwrap();
        let max_samples = inner.max_samples;
        let samples = inner.samples.entry(self.label.clone()).or_default();
        if samples.len() >= max_samples {
            samples.remove(0);
        }
        samples.push(dt);
    }
}

impl Profiler {
    /// Log toutes les métriques scalaires vers l’info log avec un target spécifique
    pub fn log_metrics_for_target(&self, target: &str, show_fps: bool) {
        if show_fps {
            info!(target: target, "{:.2} FPS", self.fps());
        }
        // Lecture des métriques de temps
        for (label, (avg, min, max)) in self.summary() {
            info!(
                target: target,
                "{}: avg = {:.3} ms | min = {:.3} ms | max = {:.3} ms",
                label, avg, min, max
            );
        }
        // Lecture des métriques scalaires
        let metrics = self.metrics_summary();
        for (label, (avg, min, max)) in metrics {
            info!(target: target, "{label}: avg={avg:}, min={min:}, max={max:}");
        }
    }
}

/// Macro helper : déduit automatiquement le target via le module appelant
// macro module_path!() qui est évaluée au moment de la compilation pour obtenir le module courant.
#[macro_export]
macro_rules! log_metrics {
    ($profiler:expr) => {
        $profiler.log_metrics_for_target(module_path!(), false);
    };
}

#[macro_export]
macro_rules! log_metrics_and_fps {
    ($profiler:expr) => {
        $profiler.log_metrics_for_target(module_path!(), true);
    };
}
