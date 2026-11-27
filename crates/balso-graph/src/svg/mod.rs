mod bond;
mod text;

use std::collections::HashMap;

use super::Atom;
use crate::layout::{self, Layout, Vec2};

use bond::render_bond;
use text::{label_extent, render_atom};

/// Distance between bonded atoms, in SVG user units.
const BOND_LENGTH: f64 = 60.0;
/// Atom label font size, in SVG user units.
const FONT_SIZE: f64 = 17.0;
/// Empty space kept around the drawing.
const PADDING: f64 = 18.0;
/// Aromatic circles sit inside their ring, clear of the bond strokes.
const AROMATIC_CIRCLE_SCALE: f64 = 0.68;
const LINE_STYLE: &str = "line{stroke-width:1.2;stroke-linecap:round;stroke-linejoin:round;fill:none}";

type Bounds = (f64, f64, f64, f64);

fn bond_ring_centers(layout: &Layout) -> HashMap<(usize, usize), Vec2> {
    let mut centers = HashMap::new();

    for ring_index in 0..layout.rings.len() {
        let Some((center, _)) = layout.ring_geometry(ring_index) else {
            continue;
        };
        let ring = &layout.rings[ring_index];
        for (&first, &second) in ring.iter().zip(ring.iter().cycle().skip(1)).take(ring.len()) {
            let key = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            centers.entry(key).or_insert(center);
        }
    }

    centers
}

fn scaled(layout: &Layout) -> Vec<Vec2> {
    layout
        .positions
        .iter()
        .map(|position| Vec2::new(position.x * BOND_LENGTH, position.y * BOND_LENGTH))
        .collect()
}

fn label_position(atom: &Atom, index: usize, positions: &[Vec2]) -> Vec2 {
    let position = positions[index];
    if atom.bonds.len() != 1 {
        return position;
    }

    let Some((half_width, _)) = label_extent(atom, FONT_SIZE) else {
        return position;
    };
    if half_width <= 3.0 {
        return position;
    }

    let neighbor = positions[atom.bonds[0].tid];
    let outward = (position - neighbor).normalized();
    position + outward * (half_width - 3.0)
}

fn bounds(atoms: &[Atom], positions: &[Vec2]) -> Bounds {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for (index, atom) in atoms.iter().enumerate() {
        let position = label_position(atom, index, positions);
        let (half_width, half_height) = label_extent(atom, FONT_SIZE).unwrap_or((0.0, 0.0));
        min_x = min_x.min(position.x - half_width);
        max_x = max_x.max(position.x + half_width);
        min_y = min_y.min(position.y - half_height);
        max_y = max_y.max(position.y + half_height);
    }

    if !min_x.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }

    (min_x - PADDING, max_x + PADDING, min_y - PADDING, max_y + PADDING)
}

/// Render a molecular graph as a standalone SVG document.
pub fn generate_svg(atoms: &[Atom]) -> String {
    if atoms.is_empty() {
        return String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#);
    }

    let layout = layout::compute(atoms);
    let positions = scaled(&layout);
    let ring_centers = bond_ring_centers(&layout);

    let (start_x, end_x, start_y, end_y) = bounds(atoms, &positions);
    let width = (end_x - start_x).max(1.0);
    let height = (end_y - start_y).max(1.0);

    let mut masks = String::new();
    let mut bonds = String::new();
    let mut labels = String::new();

    for (index, atom) in atoms.iter().enumerate() {
        for edge in &atom.bonds {
            if edge.tid <= index || edge.tid >= atoms.len() {
                continue;
            }

            let key = if index <= edge.tid {
                (index, edge.tid)
            } else {
                (edge.tid, index)
            };
            let ring_center = ring_centers
                .get(&key)
                .map(|center| Vec2::new(center.x * BOND_LENGTH, center.y * BOND_LENGTH));

            render_bond(positions[index], positions[edge.tid], edge, ring_center, &mut bonds);
        }

        render_atom(
            atom,
            label_position(atom, index, &positions),
            FONT_SIZE,
            &mut masks,
            &mut labels,
        );
    }

    for &ring_index in &layout.aromatic_rings {
        if let Some((center, inradius)) = layout.ring_geometry(ring_index) {
            bonds.push_str(&format!(
                r#"<circle cx="{:.3}" cy="{:.3}" r="{:.3}" fill="none" stroke="white" stroke-width="1.2" />"#,
                center.x * BOND_LENGTH,
                center.y * BOND_LENGTH,
                inradius * BOND_LENGTH * AROMATIC_CIRCLE_SCALE
            ));
        }
    }

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="auto" viewBox="{:.3} {:.3} {:.3} {:.3}"><style>{}</style>"#,
        start_x, start_y, width, height, LINE_STYLE
    ));

    if masks.is_empty() {
        svg.push_str(&bonds);
    } else {
        svg.push_str(&format!(
            r#"<mask id="atom-label-mask"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="white"/>{}</mask>"#,
            start_x, start_y, width, height, masks
        ));
        svg.push_str(r#"<g mask="url(#atom-label-mask)">"#);
        svg.push_str(&bonds);
        svg.push_str("</g>");
    }

    svg.push_str(&labels);
    svg.push_str("</svg>");

    svg
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::Builder;
    use balso_parser::read;

    fn svg(smiles: &str) -> String {
        let mut builder = Builder::new();
        read(smiles, &mut builder).expect("parse");
        generate_svg(&builder.build())
    }

    #[test]
    fn empty_graph_is_an_empty_svg() {
        assert_eq!(generate_svg(&[]), r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#);
    }

    #[test]
    fn benzene_draws_a_closed_ring_with_an_aromatic_circle() {
        let svg = svg("c1ccccc1");

        assert_eq!(svg.matches("<line").count(), 6);
        assert_eq!(svg.matches("<circle").count(), 1);
        assert_eq!(svg.matches("<ellipse").count(), 0);
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn ethanol_has_one_heteroatom_label() {
        let svg = svg("CCO");

        assert_eq!(svg.matches("<ellipse").count(), 1);
        assert!(svg.contains(">OH</text>"));
    }

    #[test]
    fn naphthalene_has_no_coincident_vertices() {
        let svg = svg("c1ccc2ccccc2c1");
        let mut coordinates = Vec::new();

        for line in svg.split("<line").skip(1) {
            coordinates.push((attribute(line, "x1"), attribute(line, "y1")));
            coordinates.push((attribute(line, "x2"), attribute(line, "y2")));
        }

        coordinates.sort_by(|first, second| first.partial_cmp(second).expect("order"));
        coordinates.dedup();
        assert_eq!(coordinates.len(), 10);
    }

    fn attribute(line: &str, name: &str) -> f64 {
        let prefix = format!(r#"{name}=""#);
        let start = line.find(&prefix).expect("attribute") + prefix.len();
        let end = start + line[start..].find('"').expect("value");
        line[start..end].parse().expect("number")
    }
}
