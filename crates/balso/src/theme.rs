//! Theme management for molecular visualization.
//!
//! This module provides theme support for different visual styles
//! (light, dark, etc.) with configurable colors for bonds, atoms,
//! backgrounds, and other visual elements.

use std::collections::HashMap;

/// Color type represented as hex string.
pub type Color = String;

/// A theme defines the visual appearance for drawing molecules.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Theme name
    name: String,
    /// Color palette
    colors: HashMap<String, Color>,
}

impl Theme {
    /// Create a new theme with the given name and colors.
    pub fn new(name: String, colors: HashMap<String, Color>) -> Self {
        Self { name, colors }
    }

    /// Get a color by name, returning a default if not found.
    pub fn get(&self, key: &str) -> &str {
        self.colors.get(key).map(|s| s.as_str()).unwrap_or("#000000")
    }

    /// Get the theme name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get bond color.
    pub fn bond_color(&self) -> &str {
        self.get("bond")
    }

    /// Get atom/text color.
    pub fn atom_color(&self) -> &str {
        self.get("atom")
    }

    /// Get carbon atom color.
    pub fn carbon_color(&self) -> &str {
        self.get("carbon")
    }

    /// Get background color.
    pub fn background_color(&self) -> &str {
        self.get("background")
    }

    /// Get highlight color.
    pub fn highlight_color(&self) -> &str {
        self.get("highlight")
    }

    /// Get error/error color.
    pub fn error_color(&self) -> &str {
        self.get("error")
    }
}

/// Predefined themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    /// Light theme (default).
    Light,
    /// Dark theme.
    Dark,
    /// High contrast theme.
    HighContrast,
    /// Colorful theme.
    Colorful,
}

/// Theme manager for loading and switching themes.
#[derive(Debug, Clone)]
pub struct ThemeManager {
    themes: HashMap<String, Theme>,
    current_theme: ThemeName,
}

impl ThemeManager {
    /// Create a new theme manager with default themes.
    pub fn new() -> Self {
        let mut themes = HashMap::new();

        // Light theme (default)
        themes.insert(
            ThemeName::Light.to_string(),
            Theme::new(
                "light".to_string(),
                vec![
                    ("bond".to_string(), "#333333".to_string()),
                    ("atom".to_string(), "#333333".to_string()),
                    ("carbon".to_string(), "#333333".to_string()),
                    ("background".to_string(), "#ffffff".to_string()),
                    ("highlight".to_string(), "#f0f0f0".to_string()),
                    ("error".to_string(), "#ff0000".to_string()),
                    ("aromatic".to_string(), "#666666".to_string()),
                    ("wedge".to_string(), "#333333".to_string()),
                    ("hash".to_string(), "#333333".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        );

        // Dark theme
        themes.insert(
            ThemeName::Dark.to_string(),
            Theme::new(
                "dark".to_string(),
                vec![
                    ("bond".to_string(), "#e0e0e0".to_string()),
                    ("atom".to_string(), "#e0e0e0".to_string()),
                    ("carbon".to_string(), "#e0e0e0".to_string()),
                    ("background".to_string(), "#1a1a2e".to_string()),
                    ("highlight".to_string(), "#2d2d44".to_string()),
                    ("error".to_string(), "#ff6b6b".to_string()),
                    ("aromatic".to_string(), "#888888".to_string()),
                    ("wedge".to_string(), "#e0e0e0".to_string()),
                    ("hash".to_string(), "#e0e0e0".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        );

        // High contrast theme
        themes.insert(
            ThemeName::HighContrast.to_string(),
            Theme::new(
                "high-contrast".to_string(),
                vec![
                    ("bond".to_string(), "#000000".to_string()),
                    ("atom".to_string(), "#000000".to_string()),
                    ("carbon".to_string(), "#000000".to_string()),
                    ("background".to_string(), "#ffffff".to_string()),
                    ("highlight".to_string(), "#ffff00".to_string()),
                    ("error".to_string(), "#ff0000".to_string()),
                    ("aromatic".to_string(), "#000000".to_string()),
                    ("wedge".to_string(), "#000000".to_string()),
                    ("hash".to_string(), "#000000".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        );

        // Colorful theme (for educational purposes)
        themes.insert(
            ThemeName::Colorful.to_string(),
            Theme::new(
                "colorful".to_string(),
                vec![
                    ("bond".to_string(), "#2c3e50".to_string()),
                    ("atom".to_string(), "#2c3e50".to_string()),
                    ("carbon".to_string(), "#2c3e50".to_string()),
                    ("background".to_string(), "#fafafa".to_string()),
                    ("highlight".to_string(), "#e8f4f8".to_string()),
                    ("error".to_string(), "#e74c3c".to_string()),
                    ("aromatic".to_string(), "#9b59b6".to_string()),
                    ("wedge".to_string(), "#2c3e50".to_string()),
                    ("hash".to_string(), "#2c3e50".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
        );

        Self {
            themes,
            current_theme: ThemeName::Light,
        }
    }

    /// Create a theme manager with a specific theme.
    pub fn with_theme(theme_name: ThemeName) -> Self {
        let mut manager = Self::new();
        manager.set_theme(theme_name);
        manager
    }

    /// Get the current theme.
    pub fn current_theme(&self) -> &Theme {
        self.themes
            .get(&self.current_theme.to_string())
            .expect("Theme should always exist")
    }

    /// Set the current theme.
    pub fn set_theme(&mut self, theme_name: ThemeName) {
        self.current_theme = theme_name;
    }

    /// Get a specific theme by name.
    pub fn get_theme(&self, theme_name: ThemeName) -> Option<&Theme> {
        self.themes.get(&theme_name.to_string())
    }

    /// Add a custom theme.
    pub fn add_theme(&mut self, theme: Theme) {
        self.themes.insert(theme.name().to_string(), theme);
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToString for ThemeName {
    fn to_string(&self) -> String {
        match self {
            ThemeName::Light => "light".to_string(),
            ThemeName::Dark => "dark".to_string(),
            ThemeName::HighContrast => "high-contrast".to_string(),
            ThemeName::Colorful => "colorful".to_string(),
        }
    }
}

impl TryFrom<&str> for ThemeName {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "light" => Ok(ThemeName::Light),
            "dark" => Ok(ThemeName::Dark),
            "high-contrast" | "highcontrast" => Ok(ThemeName::HighContrast),
            "colorful" => Ok(ThemeName::Colorful),
            _ => Err("Unknown theme name"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_theme() {
        let manager = ThemeManager::new();
        let theme = manager.current_theme();

        assert_eq!(theme.name(), "light");
        assert_eq!(theme.bond_color(), "#333333");
        assert_eq!(theme.background_color(), "#ffffff");
    }

    #[test]
    fn test_dark_theme() {
        let mut manager = ThemeManager::new();
        manager.set_theme(ThemeName::Dark);
        let theme = manager.current_theme();

        assert_eq!(theme.name(), "dark");
        assert_eq!(theme.bond_color(), "#e0e0e0");
        assert_eq!(theme.background_color(), "#1a1a2e");
    }

    #[test]
    fn test_theme_switching() {
        let mut manager = ThemeManager::new();

        assert_eq!(manager.current_theme().name(), "light");

        manager.set_theme(ThemeName::Dark);
        assert_eq!(manager.current_theme().name(), "dark");

        manager.set_theme(ThemeName::Light);
        assert_eq!(manager.current_theme().name(), "light");
    }

    #[test]
    fn test_theme_name_parsing() {
        assert_eq!(ThemeName::try_from("light"), Ok(ThemeName::Light));
        assert_eq!(ThemeName::try_from("dark"), Ok(ThemeName::Dark));
        assert_eq!(
            ThemeName::try_from("high-contrast"),
            Ok(ThemeName::HighContrast)
        );
        assert_eq!(ThemeName::try_from("unknown"), Err("Unknown theme name"));
    }
}
