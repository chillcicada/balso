use super::Vec2;
use crate::Bond;

use balso_core::BondKind;

/// Distance between the two strokes of a double bond, in SVG user units.
const DOUBLE_BOND_OFFSET: f64 = 2.6;
/// Distance between the outer strokes of a triple bond.
const TRIPLE_BOND_OFFSET: f64 = 3.4;
/// Fraction of a ring double bond trimmed at each end so the inner stroke
/// does not touch the neighboring bonds.
const INNER_BOND_TRIM: f64 = 0.16;

fn render_line(out: &mut String, x1: f64, y1: f64, x2: f64, y2: f64) {
    out.push_str(&format!(
        r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}" style="stroke:white" />"#,
        x1, y1, x2, y2
    ));
}

fn render_offset_line(start: Vec2, end: Vec2, normal: Vec2, offset: f64, out: &mut String) {
    let shift = normal * offset;
    render_line(
        out,
        start.x + shift.x,
        start.y + shift.y,
        end.x + shift.x,
        end.y + shift.y,
    );
}

fn unit_normal(start: Vec2, end: Vec2) -> Vec2 {
    let edge = end - start;
    let length = edge.length();
    if length <= f64::EPSILON {
        Vec2::default()
    } else {
        Vec2::new(-edge.y / length, edge.x / length)
    }
}

/// Render one bond.
///
/// `ring_center` is the centroid of a ring containing the bond. It makes a
/// double bond draw its second stroke on the inside of the ring, the
/// conventional depiction for cyclic alkenes.
pub fn render_bond(start: Vec2, end: Vec2, bond: &Bond, ring_center: Option<Vec2>, out: &mut String) {
    match bond.kind {
        BondKind::Elided | BondKind::Single | BondKind::Up | BondKind::Down => {
            render_line(out, start.x, start.y, end.x, end.y);
        }
        BondKind::Double => {
            if let Some(center) = ring_center {
                render_line(out, start.x, start.y, end.x, end.y);

                let midpoint = (start + end) / 2.0;
                let inward = (center - midpoint).normalized();
                let trim = (end - start) * INNER_BOND_TRIM;
                let inner_start = start + trim + inward * DOUBLE_BOND_OFFSET;
                let inner_end = end - trim + inward * DOUBLE_BOND_OFFSET;
                render_line(out, inner_start.x, inner_start.y, inner_end.x, inner_end.y);
            } else {
                let normal = unit_normal(start, end);
                render_offset_line(start, end, normal, DOUBLE_BOND_OFFSET / 2.0, out);
                render_offset_line(start, end, normal, -DOUBLE_BOND_OFFSET / 2.0, out);
            }
        }
        BondKind::Triple => {
            let normal = unit_normal(start, end);
            render_offset_line(start, end, normal, TRIPLE_BOND_OFFSET, out);
            render_line(out, start.x, start.y, end.x, end.y);
            render_offset_line(start, end, normal, -TRIPLE_BOND_OFFSET, out);
        }
    }
}
