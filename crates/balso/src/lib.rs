//! Balso-draw: Molecular visualization library for Balsa molecular notation.
//!
//! This crate provides SVG-based drawing functionality for molecules
//! parsed from Balsa notation strings. It uses the layout algorithms
//! from balso-layout to calculate 2D positions and renders them as SVG.
//!
//! # Example
//!
//! ```rust,ignore
//! use balso_parser::Parser;
//! use balso_layout::Layout;
//! use balso_draw::{SvgDrawer, DrawOptions, ThemeManager, ThemeName};
//!
//! let smiles = "CCO";  // ethanol
//! let parser = Parser::new();
//! let atoms = parser.parse(smiles).unwrap();
//!
//! let mut layout = Layout::new(&atoms);
//! let graph = layout.layout();
//!
//! let options = DrawOptions::default();
//! let theme = ThemeManager::with_theme(ThemeName::Light);
//! let svg = SvgDrawer::new(&graph, options, theme.current_theme().clone()).draw();
//! ```
//!
//! # Features
//!
//! - SVG output for web and publication quality
//! - Multiple themes (light, dark, high-contrast)
//! - Support for different atom visualization styles
//! - Aromaticity circles for aromatic rings
//! - Stereochemistry wedges and hashed bonds
//! - Debug mode for development

pub mod drawer;
pub mod options;
pub mod svg_wrapper;
pub mod theme;

pub use drawer::SvgDrawer;
pub use options::{AtomVisualization, DrawOptions};
pub use theme::{Theme, ThemeManager, ThemeName};

use balso_layout::LayoutGraph;

/// Draw a molecule to SVG.
///
/// This is the main entry point for drawing molecules.
///
/// # Arguments
///
/// * `graph` - A layout graph containing positioned atoms and bonds.
/// * `options` - Drawing options controlling the appearance.
/// * `theme` - The color theme to use.
///
/// # Returns
///
/// An SVG string representing the drawn molecule.
pub fn draw(
    graph: &LayoutGraph,
    options: &DrawOptions,
    theme: &ThemeManager,
) -> String {
    let theme = theme.current_theme().clone();
    let mut drawer = SvgDrawer::new(graph, options.clone(), theme);
    drawer.draw()
}

/// Draw a molecule with default options and light theme.
pub fn draw_default(graph: &LayoutGraph) -> String {
    let options = DrawOptions::default();
    let theme = ThemeManager::with_theme(ThemeName::Light);
    draw(graph, &options, &theme)
}

/// Draw a molecule with debug information.
pub fn draw_debug(graph: &LayoutGraph) -> String {
    let options = DrawOptions::default().with_debug(true);
    let theme = ThemeManager::with_theme(ThemeName::Light);
    draw(graph, &options, &theme)
}
