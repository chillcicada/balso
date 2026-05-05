use super::util::Vec2;
use crate::Bond;

use balso_core::BondKind;

const LINE_OFFSET: f64 = 2.5;
const DOUBLE_BOND_SCALE: f64 = 1.0 / 6.0;
const LINE_STYLE: &str = "stroke-width:1;stroke-linecap:round;stroke-dasharray:none";

#[derive(Debug, Clone)]
pub enum BondOffsetType {
    Parallel,
    CenterOffset(OffsetDirection),
}

#[derive(Debug, Clone)]
pub enum OffsetDirection {
    Up,
    Down,
}

fn render_line(out: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str) {
    out.push_str(&format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" style="{};stroke:{}" />"#,
        x1, y1, x2, y2, LINE_STYLE, stroke
    ));
}

fn perpendicular_offset(start: Vec2, end: Vec2) -> (f64, f64, f64, f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    let nx = -dy / len;
    let ny = dx / len;
    (dx, dy, nx, ny)
}

pub fn render_bond(start: Vec2, end: Vec2, bond: &Bond, out: &mut String, offset_type: BondOffsetType) {
    match bond.kind {
        BondKind::Elided | BondKind::Single => {
            render_line(out, start.x, start.y, end.x, end.y, "white");
        }
        BondKind::Double => {
            let (dx, dy, nx, ny) = perpendicular_offset(start, end);
            match offset_type {
                BondOffsetType::Parallel => {
                    for sign in [-0.75, 0.75] {
                        let ox = nx * LINE_OFFSET * sign;
                        let oy = ny * LINE_OFFSET * sign;
                        render_line(out, start.x + ox, start.y + oy, end.x + ox, end.y + oy, "blue");
                    }
                }
                BondOffsetType::CenterOffset(dir) => {
                    render_line(out, start.x, start.y, end.x, end.y, "white");
                    let sign = match dir {
                        OffsetDirection::Up => -1.5,
                        OffsetDirection::Down => 1.5,
                    };
                    let ox = nx * LINE_OFFSET * sign;
                    let oy = ny * LINE_OFFSET * sign;
                    render_line(
                        out,
                        start.x + ox + dx * DOUBLE_BOND_SCALE,
                        start.y + oy + dy * DOUBLE_BOND_SCALE,
                        end.x + ox - dx * DOUBLE_BOND_SCALE,
                        end.y + oy - dy * DOUBLE_BOND_SCALE,
                        "blue",
                    );
                }
            }
        }
        BondKind::Triple => {
            render_line(out, start.x, start.y, end.x, end.y, "white");
            let (_dx, _dy, nx, ny) = perpendicular_offset(start, end);
            for sign in [-1.0, 1.0] {
                let ox = nx * LINE_OFFSET * sign;
                let oy = ny * LINE_OFFSET * sign;
                render_line(out, start.x + ox, start.y + oy, end.x + ox, end.y + oy, "red");
            }
        }
        BondKind::Down => todo!(),
        BondKind::Up => todo!(),
    }
}
