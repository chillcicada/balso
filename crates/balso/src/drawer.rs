//! Main molecular drawer.
//!
//! This module provides the main drawing functionality for rendering
//! molecules as SVG based on layout information.

use std::collections::HashSet;

use balso_core::{AtomKind, BondKind, Element};
use balso_layout::{LayoutGraph, Ring};
use nalgebra::Vector2;

use crate::options::DrawOptions;
use crate::svg_wrapper::{geometry, SvgWrapper};
use crate::theme::Theme;

/// Information about a vertex for drawing purposes.
#[derive(Debug, Clone)]
pub struct VertexInfo {
    pub id: usize,
    pub position: Vector2<f64>,
    pub element: Option<Element>,
    pub is_aromatic: bool,
    pub is_terminal: bool,
    pub bond_count: usize,
    pub implicit_hydrogens: u8,
    pub rings: Vec<usize>,
}

/// Information about an edge for drawing purposes.
#[derive(Debug, Clone)]
pub struct EdgeInfo {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub bond_kind: BondKind,
    pub in_ring: bool,
    pub ring_id: Option<usize>,
}

/// The main molecular drawer.
pub struct SvgDrawer<'a> {
    graph: &'a LayoutGraph,
    options: DrawOptions,
    theme: Theme,
    svg: SvgWrapper,
    drawn_edges: HashSet<usize>,
}

impl<'a> SvgDrawer<'a> {
    /// Create a new SVG drawer with a layout graph.
    pub fn new(graph: &'a LayoutGraph, options: DrawOptions, theme: Theme) -> Self {
        Self {
            graph,
            options: options.clone(),
            theme,
            svg: SvgWrapper::with_padding(options.padding),
            drawn_edges: HashSet::new(),
        }
    }

    /// Run the full drawing pipeline.
    pub fn draw(&mut self) -> String {
        // Step 1: Determine canvas dimensions
        self.determine_dimensions();

        // Step 2: Draw edges (bonds)
        self.draw_edges();

        // Step 3: Draw vertices (atoms)
        self.draw_vertices();

        // Step 4: Draw aromaticity circles
        if self.options.draw_aromaticity {
            self.draw_aromaticity_circles();
        }

        // Step 5: Generate SVG output
        self.svg.to_string()
    }

    /// Determine the canvas dimensions based on vertex positions.
    fn determine_dimensions(&mut self) {
        let vertices = self.graph.vertices();

        if vertices.is_empty() {
            self.svg.set_dimensions(100.0, 100.0);
            return;
        }

        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for vertex in vertices {
            let pos = vertex.position;
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
        }

        // Add padding
        let padding = self.options.padding;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        let width = max_x - min_x;
        let height = max_y - min_y;

        self.svg.set_dimensions(width, height);
        self.svg.set_viewbox(min_x, min_y, width, height);
    }

    /// Draw all edges (bonds) using BFS order to avoid duplicates.
    fn draw_edges(&mut self) {
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start BFS from vertex 0
        if self.graph.vertex_count() > 0 {
            queue.push_back(0);
            visited.insert(0);
        }

        while let Some(vertex_id) = queue.pop_front() {
            // Get neighbors and find edge IDs
            for (neighbor_id, _edge) in self.graph.neighbors(vertex_id) {
                // Find the edge ID between current vertex and neighbor
                let edge_id = self.graph.get_edge(vertex_id, neighbor_id);

                if let Some(eid) = edge_id {
                    // Check if this edge has already been drawn
                    if !self.drawn_edges.contains(&eid) {
                        self.drawn_edges.insert(eid);
                        self.draw_edge(eid);
                    }
                }

                // Add neighbor to queue if not visited
                if !visited.contains(&neighbor_id) {
                    visited.insert(neighbor_id);
                    queue.push_back(neighbor_id);
                }
            }
        }
    }

    /// Draw a single edge.
    fn draw_edge(&mut self, edge_id: usize) {
        let edge = match self.graph.edge_by_id(edge_id) {
            Some(e) => e,
            None => return,
        };

        let source = match self.graph.vertex(edge.source) {
            Some(v) => v,
            None => return,
        };

        let target = match self.graph.vertex(edge.target) {
            Some(v) => v,
            None => return,
        };

        let a = source.position;
        let b = target.position;

        match edge.bond_kind {
            BondKind::Double => self.draw_double_bond(&a, &b, edge),
            BondKind::Triple => self.draw_triple_bond(&a, &b),
            BondKind::Up => self.draw_wedge(&a, &b),
            BondKind::Down => self.draw_hashed_wedge(&a, &b),
            _ => self.draw_single_bond(&a, &b),
        }
    }

    /// Draw a single bond.
    fn draw_single_bond(&mut self, a: &Vector2<f64>, b: &Vector2<f64>) {
        self.svg.add_line(
            *a,
            *b,
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
    }

    /// Draw a double bond.
    fn draw_double_bond(&mut self, a: &Vector2<f64>, b: &Vector2<f64>, edge: &balso_layout::Edge) {
        let spacing = self.options.bond_spacing;
        let normals = geometry::double_bond_normals(a, b, spacing / 2.0);

        // Check if this is a ring bond (needs special handling)
        if edge.in_ring {
            // For ring bonds, offset outward from ring center
            if let Some(ring_id) = edge.ring_id {
                if let Some(ring) = self.graph.rings().get(ring_id) {
                    self.draw_offset_double_bond(a, b, ring, spacing);
                    return;
                }
            }
        }

        // Regular double bond: two parallel lines
        let a1 = a + normals[0];
        let b1 = b + normals[0];
        let a2 = a + normals[1];
        let b2 = b + normals[1];

        self.svg.add_line(
            a1,
            b1,
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
        self.svg.add_line(
            a2,
            b2,
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
    }

    /// Draw a double bond offset from the ring.
    fn draw_offset_double_bond(
        &mut self,
        a: &Vector2<f64>,
        b: &Vector2<f64>,
        ring: &Ring,
        spacing: f64,
    ) {
        let center = ring.center;
        let mid = (a + b) / 2.0;

        // Direction from ring center to bond midpoint
        let dir = (mid - center).normalize();
        let offset = dir * spacing;

        let a1 = *a + offset;
        let b1 = *b + offset;
        let a2 = *a + offset * 2.0;
        let b2 = *b + offset * 2.0;

        self.svg.add_line(
            a1,
            b1,
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
        self.svg.add_line(
            a2,
            b2,
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
    }

    /// Draw a triple bond.
    fn draw_triple_bond(&mut self, a: &Vector2<f64>, b: &Vector2<f64>) {
        let spacing = self.options.bond_spacing;
        let normals = geometry::double_bond_normals(a, b, spacing);

        // Draw three parallel lines
        self.svg.add_line(
            *a,
            *b,
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
        self.svg.add_line(
            a + normals[0],
            b + normals[0],
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
        self.svg.add_line(
            a + normals[1],
            b + normals[1],
            self.theme.bond_color().to_string(),
            self.options.bond_thickness,
        );
    }

    /// Draw a solid wedge (up stereochemistry).
    fn draw_wedge(&mut self, a: &Vector2<f64>, b: &Vector2<f64>) {
        let width = self.options.bond_length * 0.15;
        self.svg.add_wedge(
            *a,
            *b,
            width,
            self.theme.wedge_color().to_string(),
        );
    }

    /// Draw a hashed wedge (down stereochemistry).
    fn draw_hashed_wedge(&mut self, a: &Vector2<f64>, b: &Vector2<f64>) {
        let width = self.options.bond_length * 0.15;
        self.svg.add_hashed_wedge(
            *a,
            *b,
            width,
            self.theme.background_color().to_string(),
            self.theme.hash_color().to_string(),
            self.options.bond_thickness,
        );
    }

    /// Draw all vertices (atoms).
    fn draw_vertices(&mut self) {
        for vertex in self.graph.vertices() {
            self.draw_vertex(vertex);
        }
    }

    /// Draw a single vertex (atom).
    fn draw_vertex(&mut self, vertex: &balso_layout::Vertex) {
        let info = self.get_vertex_info(vertex);

        // Decide whether to draw the atom label
        let should_draw = self.should_draw_atom(&info);

        if !should_draw {
            return;
        }

        match self.options.atom_visualization {
            crate::options::AtomVisualization::Balls => {
                self.draw_ball(&info);
            }
            crate::options::AtomVisualization::Text => {
                self.draw_atom_label(&info);
            }
        }

        // Debug: draw indices
        if self.options.debug_indices {
            self.draw_debug_index(&info);
        }
    }

    /// Get information about a vertex for drawing.
    fn get_vertex_info(&self, vertex: &balso_layout::Vertex) -> VertexInfo {
        let element = self.get_element(&vertex.atom.kind);
        let bond_count = vertex.atom.bonds.len();
        let implicit_hydrogens = vertex.atom.hydrogens();

        // Check if terminal (only one bond)
        let is_terminal = bond_count <= 1;

        // Check if part of aromatic system
        let is_aromatic = match &vertex.atom.kind {
            AtomKind::Selection(_) => true, // Aromatic atoms are selections (lowercase)
            _ => false,
        };

        VertexInfo {
            id: vertex.id,
            position: vertex.position,
            element,
            is_aromatic,
            is_terminal,
            bond_count,
            implicit_hydrogens,
            rings: vertex.rings.clone(),
        }
    }

    /// Get the element from an atom kind.
    fn get_element(&self, kind: &AtomKind) -> Option<Element> {
        match kind {
            AtomKind::Shortcut(s) => Some(s.into()),
            AtomKind::Selection(s) => Some(s.into()),
            AtomKind::Bracket(b) => match &b.symbol {
                balso_core::Symbol::Element(e) => Some(e.clone()),
                _ => None,
            },
            AtomKind::Star => None,
        }
    }

    /// Determine if an atom should be drawn.
    fn should_draw_atom(&self, info: &VertexInfo) -> bool {
        let element = match &info.element {
            Some(e) => e,
            None => return true, // Always draw non-element atoms
        };

        // Carbon has special handling
        if *element == Element::C {
            // Draw carbon if:
            // 1. It's terminal
            // 2. Has explicit flag (not implemented here)
            // 3. Has implicit hydrogens we need to show
            // 4. Terminal carbon option is enabled
            if info.is_terminal && self.options.draw_terminal_carbon {
                return true;
            }
            if info.implicit_hydrogens > 0 && self.options.draw_explicit_hydrogens {
                return true;
            }
            // Default: don't draw carbon
            return false;
        }

        // Non-carbon atoms are always drawn
        true
    }

    /// Draw an atom as a ball/circle.
    fn draw_ball(&mut self, info: &VertexInfo) {
        if let Some(element) = &info.element {
            self.svg.add_circle(
                info.position,
                self.options.atom_radius,
                Some(self.get_element_color(element)),
                Some(self.theme.bond_color().to_string()),
                Some(self.options.bond_thickness),
            );
        }
    }

    /// Draw an atom label with implicit hydrogens.
    fn draw_atom_label(&mut self, info: &VertexInfo) {
        let label = self.format_atom_label(info);

        if label.is_empty() {
            return;
        }

        self.svg.add_text_full(
            info.position,
            label,
            self.theme.atom_color().to_string(),
            self.options.atom_font_size,
            self.options.atom_font_family.clone(),
            "middle".to_string(),
            "middle".to_string(),
        );
    }

    /// Format an atom label with its symbol and implicit hydrogens.
    fn format_atom_label(&self, info: &VertexInfo) -> String {
        let element_symbol = match &info.element {
            Some(e) => e.to_string(),
            None => return String::new(),
        };

        let hydrogens = info.implicit_hydrogens;

        // Special case: don't draw H alone
        if element_symbol == "H" && hydrogens == 0 {
            return String::new();
        }

        match hydrogens {
            0 => element_symbol,
            1 => format!("{}H", element_symbol),
            2 => format!("{}H2", element_symbol),
            3 => format!("{}H3", element_symbol),
            _ => format!("{}H{}", element_symbol, hydrogens),
        }
    }

    /// Get the color for an element (CPK coloring).
    fn get_element_color(&self, element: &Element) -> String {
        // CPK-style coloring
        match element {
            Element::C => "#909090".to_string(),      // Carbon - gray
            Element::N => "#3050F8".to_string(),      // Nitrogen - blue
            Element::O => "#FF0D0D".to_string(),      // Oxygen - red
            Element::H => "#FFFFFF".to_string(),      // Hydrogen - white
            Element::S => "#FFFF30".to_string(),      // Sulfur - yellow
            Element::P => "#FF8000".to_string(),      // Phosphorus - orange
            Element::Cl => "#1FF01F".to_string(),     // Chlorine - green
            Element::Br => "#A62929".to_string(),     // Bromine - dark red
            Element::I => "#940094".to_string(),      // Iodine - purple
            Element::F => "#90E050".to_string(),      // Fluorine - light green
            Element::B => "#FFB5B5".to_string(),      // Boron - pink
            Element::Li => "#CC80FF".to_string(),     // Lithium - violet
            Element::Na => "#AB5CF2".to_string(),     // Sodium - purple
            Element::K => "#8F40D4".to_string(),      // Potassium - violet
            Element::Ca => "#3DFF00".to_string(),     // Calcium - green
            Element::Fe => "#E06633".to_string(),     // Iron - orange
            Element::Cu => "#C88033".to_string(),    // Copper - copper
            Element::Zn => "#7D80B0".to_string(),      // Zinc - blue-gray
            Element::Ag => "#C0C0C0".to_string(),     // Silver - silver
            Element::Au => "#FFD123".to_string(),    // Gold - gold
            Element::Hg => "#B8B8D0".to_string(),     // Mercury - gray
            _ => self.theme.atom_color().to_string(), // Default color
        }
    }

    /// Draw debug index for a vertex.
    fn draw_debug_index(&mut self, info: &VertexInfo) {
        let offset = Vector2::new(5.0, -5.0);
        self.svg.add_text_full(
            info.position + offset,
            format!("{}", info.id),
            "#ff0000".to_string(),
            10.0,
            "monospace".to_string(),
            "start".to_string(),
            "middle".to_string(),
        );
    }

    /// Draw aromaticity circles for aromatic rings.
    fn draw_aromaticity_circles(&mut self) {
        for ring in self.graph.rings() {
            if self.is_aromatic_ring(ring) {
                self.draw_aromatic_ring(ring);
            }
        }
    }

    /// Check if a ring is aromatic.
    fn is_aromatic_ring(&self, ring: &Ring) -> bool {
        // A ring is aromatic if all its bonds are aromatic (single-double alternation)
        // For now, we check if all vertices in the ring are aromatic selections
        let mut aromatic_count = 0;
        let total = ring.vertices.len();

        for &vid in &ring.vertices {
            if let Some(vertex) = self.graph.vertex(vid) {
                if matches!(vertex.atom.kind, AtomKind::Selection(_)) {
                    aromatic_count += 1;
                }
            }
        }

        // Consider aromatic if all vertices are selections
        aromatic_count == total && total > 0
    }

    /// Draw an aromaticity circle for a ring.
    fn draw_aromatic_ring(&mut self, ring: &Ring) {
        // Draw a circle inside the ring
        self.svg.add_aromatic_circle(
            ring.center,
            ring.radius * 0.6,
            self.theme.aromatic_color().to_string(),
            self.options.bond_thickness * 0.5,
        );
    }
}

/// Helper function to get bond color from theme.
impl Theme {
    /// Get wedge color.
    fn wedge_color(&self) -> &str {
        self.get("wedge")
    }

    /// Get hash color.
    fn hash_color(&self) -> &str {
        self.get("hash")
    }

    /// Get aromatic color.
    fn aromatic_color(&self) -> &str {
        self.get("aromatic")
    }
}

#[cfg(test)]
mod tests {
    use balso_core::Shortcut;
    use balso_graph::Atom;
    use balso_layout::Layout;

    use crate::{DrawOptions, SvgDrawer, ThemeManager, ThemeName};

    #[test]
    fn test_draw_simple_molecule() {
        // Test drawing a simple molecule
        let atoms = vec![
            Atom::shortcut(Shortcut::C, vec![]),
        ];

        let mut layout = Layout::with_config(
            &atoms,
            balso_layout::LayoutConfig {
                bond_length: 30.0,
                ..Default::default()
            },
        );
        let graph = layout.layout();

        let options = DrawOptions::default();
        let theme = ThemeManager::with_theme(ThemeName::Light).current_theme().clone();
        let mut drawer = SvgDrawer::new(&graph, options, theme);

        let svg = drawer.draw();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_draw_water() {
        // O (water) with implicit hydrogens
        let atoms = vec![
            Atom::shortcut(Shortcut::O, vec![]),
        ];

        let mut layout = Layout::new(&atoms);
        let graph = layout.layout();

        let options = DrawOptions::default();
        let theme = ThemeManager::with_theme(ThemeName::Light).current_theme().clone();
        let mut drawer = SvgDrawer::new(&graph, options, theme);

        let svg = drawer.draw();
        assert!(svg.contains("<svg"));
        // Water should show "O" or "OH2"
        assert!(svg.contains("O"));
    }
}
