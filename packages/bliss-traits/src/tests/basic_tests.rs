// Bliss-Traits Basic Tests

use crate::shell::ColorScheme;
use crate::shell::FileDialogFilter;
use crate::shell::Viewport;
use crate::shell::*;
use std::time::Instant;

#[cfg(test)]
mod basic_functionality_tests {
    #[test]
    fn test_shell_error_creation() {
        let err = ShellError::PermissionDenied("test".to_string());
        assert!(format!("{}", err).contains("Permission denied"));

        let err = ShellError::Unsupported;
        assert_eq!(format!("{}", err), "Operation not supported");

        let err = ShellError::Backend("test error".to_string());
        assert!(format!("{}", err).contains("Backend error"));
    }

    #[test]
    fn test_viewport_creation() {
        let viewport = Viewport::new(800, 600, 2.0, ColorScheme::Light);
        assert_eq!(viewport.window_size, (800, 600));
        assert_eq!(viewport.hidpi_scale, 2.0);
        assert_eq!(viewport.zoom(), 1.0);
        assert_eq!(viewport.scale(), 2.0);
        assert_eq!(viewport.color_scheme, ColorScheme::Light);
    }

    #[test]
    fn test_viewport_scaling() {
        let mut viewport = Viewport::default();
        viewport.set_zoom(2.0);
        assert_eq!(viewport.zoom(), 2.0);

        viewport.set_hidpi_scale(1.5);
        assert_eq!(viewport.hidpi_scale, 1.5);
        assert_eq!(viewport.scale(), 3.0); // 1.5 * 2.0

        viewport.zoom_by(-0.5);
        assert_eq!(viewport.zoom(), 1.5);
        assert_eq!(viewport.scale(), 2.25); // 1.5 * 1.5
    }

    #[test]
    fn test_file_dialog_filter() {
        let filter = FileDialogFilter {
            name: "Text Files".to_string(),
            extensions: vec!["txt".to_string(), "md".to_string()],
        };

        assert_eq!(filter.name, "Text Files");
        assert_eq!(filter.extensions.len(), 2);
        assert!(filter.extensions.contains(&"txt".to_string()));
        assert!(filter.extensions.contains(&"md".to_string()));
    }

    #[test]
    fn test_color_scheme() {
        assert_eq!(ColorScheme::default(), ColorScheme::Light);
        assert_ne!(ColorScheme::Light, ColorScheme::Dark);
        assert_eq!(ColorScheme::Light, ColorScheme::Light);
        assert_eq!(ColorScheme::Dark, ColorScheme::Dark);
    }
}

#[cfg(test)]
mod performance_tests {
    use std::time::Instant;

    #[test]
    fn test_error_formatting_performance() {
        let start = Instant::now();

        for i in 0..10000 {
            let err = ShellError::PermissionDenied(format!("permission denied {}", i));
            let _ = format!("{}", err);
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 200,
            "Error formatting too slow: {:?}ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_viewport_operations_performance() {
        let mut viewport = Viewport::default();
        let start = Instant::now();

        for i in 0..100000 {
            viewport.set_zoom(1.0 + (i as f32) / 10000.0);
            viewport.set_hidpi_scale(1.0 + (i as f32) / 10000.0);
            let _ = viewport.scale();
            let _ = viewport.scale_f64();
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Viewport operations too slow: {:?}ms",
            duration.as_millis()
        );
    }
}
