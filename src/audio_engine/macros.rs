/// Macro pour créer une zone Tracy sans conditionner l'exécution du code.
/// Utilisation: `crate::tracy_zone!("nom_zone", 0xRRGGBB);`
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! tracy_zone {
    ($name:expr, $color:expr) => {
        let _span = tracy_client::span!($name);
        _span.emit_color($color);
    };
}

#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! tracy_zone {
    ($name:expr, $color:expr) => {};
}

/// Macro pour tracer des graphiques dans Tracy sans conditionner le code.
/// Utilisation: `crate::tracy_plot!("Nom", valeur);`
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! tracy_plot {
    ($name:expr, $val:expr) => {
        tracy_client::plot!($name, $val);
    };
}

#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! tracy_plot {
    ($name:expr, $val:expr) => {
        let _ = $val;
    };
}
