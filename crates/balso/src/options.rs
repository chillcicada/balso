//! Drawing options for molecular visualization.
//!
//! This module provides configuration options for controlling
//! the appearance of drawn molecules.

use crate::theme::ThemeName;

/// Visual style for atoms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomVisualization {
    /// Draw atoms as text labels (default).
    Text,
    /// Draw atoms as balls/circles.
    Balls,
}

/// Drawing options for the molecule renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawOptions {
    /// Bond thickness in pixels.
    pub bond_thickness: f64,
    /// Bond length in pixels.
    pub bond_length: f64,
    /// Spacing between parallel bonds (double/triple).
    pub bond_spacing: f64,
    /// Atom label font size.
    pub atom_font_size: f64,
    /// Atom label font family.
    pub atom_font_family: String,
    /// Visual style for atoms.
    pub atom_visualization: AtomVisualization,
    /// Atom ball radius (when using balls mode).
    pub atom_radius: f64,
    /// Padding around the molecule in pixels.
    pub padding: f64,
    /// Whether to draw aromaticity circles.
    pub draw_aromaticity: bool,
    /// Whether to draw terminal carbons.
    pub draw_terminal_carbon: bool,
    /// Whether to draw explicit hydrogens.
    pub draw_explicit_hydrogens: bool,
    /// Whether to draw atom indices (for debugging).
    pub debug_indices: bool,
    /// The theme to use.
    pub theme: ThemeName,
}

impl Default for DrawOptions {
    fn default() -> Self {
        Self {
            bond_thickness: 2.0,
            bond_length: 20.0,
            bond_spacing: 4.0,
            atom_font_size: 14.0,
            atom_font_family: "Arial, Helvetica, sans-serif".to_string(),
            atom_visualization: AtomVisualization::Text,
            atom_radius: 8.0,
            padding: 20.0,
            draw_aromaticity: true,
            draw_terminal_carbon: false,
            draw_explicit_hydrogens: false,
            debug_indices: false,
            theme: ThemeName::Light,
        }
    }
}

impl DrawOptions {
    /// Create a new options struct with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create options for dark theme.
    pub fn dark_theme() -> Self {
        let mut opts = Self::default();
        opts.theme = ThemeName::Dark;
        opts
    }

    /// Create options with balls visualization.
    pub fn with_balls() -> Self {
        let mut opts = Self::default();
        opts.atom_visualization = AtomVisualization::Balls;
        opts
    }

    /// Set the theme.
    pub fn with_theme(mut self, theme: ThemeName) -> Self {
        self.theme = theme;
        self
    }

    /// Set the bond thickness.
    pub fn with_bond_thickness(mut self, thickness: f64) -> Self {
        self.bond_thickness = thickness;
        self
    }

    /// Set the bond length.
    pub fn with_bond_length(mut self, length: f64) -> Self {
        self.bond_length = length;
        self
    }

    /// Set the bond spacing.
    pub fn with_bond_spacing(mut self, spacing: f64) -> Self {
        self.bond_spacing = spacing;
        self
    }

    /// Set the atom font size.
    pub fn with_atom_font_size(mut self, size: f64) -> Self {
        self.atom_font_size = size;
        self
    }

    /// Enable/disable aromaticity circles.
    pub fn with_draw_aromaticity(mut self, draw: bool) -> Self {
        self.draw_aromaticity = draw;
        self
    }

    /// Enable/disable terminal carbon drawing.
    pub fn with_draw_terminal_carbon(mut self, draw: bool) -> Self {
        self.draw_terminal_carbon = draw;
        self
    }

    /// Enable/disable explicit hydrogen drawing.
    pub fn with_draw_explicit_hydrogens(mut self, draw: bool) -> Self {
        self.draw_explicit_hydrogens = draw;
        self
    }

    /// Enable/disable debug mode.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug_indices = debug;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = DrawOptions::default();
        assert_eq!(opts.bond_thickness, 2.0);
        assert_eq!(opts.bond_length, 20.0);
        assert_eq!(opts.bond_spacing, 4.0);
        assert_eq!(opts.theme, ThemeName::Light);
    }

    #[test]
    fn test_builder_pattern() {
        let opts = DrawOptions::new()
            .with_theme(ThemeName::Dark)
            .with_bond_thickness(3.0)
            .with_bond_length(30.0)
            .with_draw_aromaticity(false);

        assert_eq!(opts.theme, ThemeName::Dark);
        assert_eq!(opts.bond_thickness, 3.0);
        assert_eq!(opts.bond_length, 30.0);
        assert!(!opts.draw_aromaticity);
    }

    #[test]
    fn test_dark_theme_options() {
        let opts = DrawOptions::dark_theme();
        assert_eq!(opts.theme, ThemeName::Dark);
    }

    #[test]
    fn test_balls_visualization() {
        let opts = DrawOptions::with_balls();
        assert_eq!(opts.atom_visualization, AtomVisualization::Balls);
    }
}
