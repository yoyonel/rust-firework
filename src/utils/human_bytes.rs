pub trait HumanBytes {
    fn human_bytes(&self) -> String;
}

macro_rules! impl_human_bytes {
    ($($t:ty),*) => {
        $(
            impl HumanBytes for $t {
                fn human_bytes(&self) -> String {
                    const KB: f64 = 1024.0;
                    const MB: f64 = KB * 1024.0;
                    const GB: f64 = MB * 1024.0;

                    let size = *self as f64;
                    match size.abs() {
                        s if s >= GB => format!("{:.2} GB", size / GB),
                        s if s >= MB => format!("{:.2} MB", size / MB),
                        s if s >= KB => format!("{:.2} KB", size / KB),
                        _ => format!("{} B", size),
                    }
                }
            }
        )*
    };
}

// Implémentation pour les types usuels
impl_human_bytes!(usize, isize, u64, i64, u32, i32);

#[cfg(test)]
mod tests {
    use super::HumanBytes;

    #[test]
    fn test_bytes_to_human_readable() {
        // Cas de base
        assert_eq!(0usize.human_bytes(), "0 B");
        assert_eq!(1usize.human_bytes(), "1 B");
        assert_eq!(999usize.human_bytes(), "999 B");

        // Juste en dessous de 1 KB
        assert_eq!(1023usize.human_bytes(), "1023 B");

        // 1 KB pile
        assert_eq!(1024usize.human_bytes(), "1.00 KB");

        // Un peu au-dessus
        assert_eq!(1536usize.human_bytes(), "1.50 KB");

        // 1 MB
        assert_eq!(1024usize.pow(2).human_bytes(), "1.00 MB");

        // 2.5 MB
        assert_eq!(((2.5 * 1024.0 * 1024.0) as isize).human_bytes(), "2.50 MB");

        // 1 GB
        assert_eq!((1024usize.pow(3) as isize).human_bytes(), "1.00 GB");

        // 3.25 GB
        assert_eq!(
            ((3.25 * 1024.0 * 1024.0 * 1024.0) as isize).human_bytes(),
            "3.25 GB"
        );
    }

    #[test]
    fn test_negative_values_display_correctly() {
        use super::HumanBytes;
        // Vérifie que les types signés fonctionnent et ne paniquent pas
        // Cas de base
        assert_eq!((-0).human_bytes(), "0 B");
        assert_eq!((-1).human_bytes(), "-1 B");
        assert_eq!((-999).human_bytes(), "-999 B");

        // Juste en dessous de 1 KB
        assert_eq!((-1023).human_bytes(), "-1023 B");

        // 1 KB pile
        assert_eq!((-1024).human_bytes(), "-1.00 KB");

        // Un peu au-dessus
        assert_eq!((-1536).human_bytes(), "-1.50 KB");

        // 1 MB
        assert_eq!((-(1024isize).pow(2)).human_bytes(), "-1.00 MB");

        // 2.5 MB
        assert_eq!(
            ((-2.5 * 1024.0 * 1024.0) as isize).human_bytes(),
            "-2.50 MB"
        );

        // 1 GB
        assert_eq!((-(1024i64).pow(3) as isize).human_bytes(), "-1.00 GB");

        // 3.25 GB
        assert_eq!(
            ((-3.25 * 1024.0 * 1024.0 * 1024.0) as isize).human_bytes(),
            "-3.25 GB"
        );
    }

    #[test]
    fn test_large_values_display_correctly() {
        let big_value = 10u64 * 1024 * 1024 * 1024; // 10 GB
        assert_eq!(big_value.human_bytes(), "10.00 GB");
    }

    #[test]
    fn test_precision_rounding() {
        // Vérifie que le formatage est bien à deux décimales
        let val = (1.23456 * 1024.0 * 1024.0) as isize; // ~1.23 MB
        let s = val.human_bytes();
        assert!(s.starts_with("1.23"), "format incorrect: {}", s);
    }

    #[test]
    fn test_consistency_across_types() {
        // Vérifie que différents types donnent le même résultat
        let a: usize = 1024;
        let b: u64 = 1024;
        let c: u32 = 1024;
        let d: isize = 1024;
        let e: i64 = 1024;
        let f: i32 = 1024;

        assert_eq!(a.human_bytes(), b.human_bytes());
        assert_eq!(b.human_bytes(), c.human_bytes());
        assert_eq!(c.human_bytes(), d.human_bytes());
        assert_eq!(d.human_bytes(), e.human_bytes());
        assert_eq!(e.human_bytes(), f.human_bytes());
    }
}
