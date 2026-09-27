// Simple Bliss-Traits Tests

#[test]
fn test_shell_error_display() {
    use crate::shell::ShellError;

    let err = ShellError::PermissionDenied("test".to_string());
    assert!(format!("{}", err).contains("Permission denied"));
}

#[test]
fn test_color_scheme_default() {
    use crate::shell::ColorScheme;

    assert_eq!(ColorScheme::default(), ColorScheme::Light);
}

#[test]
fn test_viewport_creation() {
    use crate::shell::ColorScheme;
    use crate::shell::Viewport;

    let viewport = Viewport::new(800, 600, 2.0, ColorScheme::Light);
    assert_eq!(viewport.window_size, (800, 600));
}
