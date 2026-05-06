mod bond;
mod text;
mod util;

use std::f64::consts::PI;

use balso_core::BondKind;

use super::Atom;

use bond::{render_bond, BondOffsetType, OffsetDirection};
use text::render_text;
use util::Vec2;

const BOND_LENGTH: f64 = 60.0;
const START_ANGLE: f64 = -PI / 6.0;

fn position_atoms(atoms: &[Atom]) -> Vec<Vec2> {
    let n = atoms.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![Vec2 { x: 0.0, y: 0.0 }];
    }

    let mut positions = vec![Vec2 { x: 0.0, y: 0.0 }; n];
    let mut visited = vec![false; n];

    fn dfs(
        atom_idx: usize,
        atoms: &[Atom],
        positions: &mut [Vec2],
        visited: &mut [bool],
        angle_from_parent: f64,
        flip: bool,
        collinear: bool,
    ) {
        visited[atom_idx] = true;

        let unvisited_neighbors: Vec<usize> = atoms[atom_idx]
            .bonds
            .iter()
            .filter(|bond| !visited[bond.tid])
            .map(|bond| bond.tid)
            .collect();

        let child_count = unvisited_neighbors.len();

        for (i, &neighbor) in unvisited_neighbors.iter().enumerate() {
            let is_collinear = atoms[atom_idx]
                .bonds
                .iter()
                .any(|bond| bond.tid == neighbor && bond.kind == BondKind::Triple);

            let angle = if is_collinear || collinear {
                angle_from_parent
            } else {
                // todo!() handle cis and trans isomers for double bonds

                match child_count {
                    1 => angle_from_parent + PI / 3.0 * if flip { -1.0 } else { 1.0 },
                    2 => match i {
                        0 => angle_from_parent + PI / 3.0,
                        1 => angle_from_parent - PI / 3.0,
                        _ => unreachable!(),
                    },
                    _ => angle_from_parent + (i as f64 - 1.0) * 2.0 * PI / (child_count as f64 + 1.0),
                }
            };
            positions[neighbor] = Vec2 {
                x: positions[atom_idx].x + BOND_LENGTH * angle.cos(),
                y: positions[atom_idx].y + BOND_LENGTH * angle.sin(),
            };
            dfs(neighbor, atoms, positions, visited, angle, !flip, is_collinear);
        }
    }

    dfs(0, atoms, &mut positions, &mut visited, START_ANGLE, false, false);

    positions
}

pub fn generate_svg(atoms: &[Atom]) -> String {
    let positions = position_atoms(atoms);

    if positions.is_empty() {
        return String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#);
    }
    if positions.len() == 1 {
        return format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="auto" viewBox="-20 -20 40 40"><text x="0" y="0" style="text-anchor:middle;dominant-baseline:middle;font-family:sans-serif;font-size:14;fill:white">{}</text></svg>"#,
            atoms[0].kind
        );
    }

    let min_x = positions.iter().map(|p| p.x).fold(f64::MAX, |a, b| a.min(b));
    let min_y = positions.iter().map(|p| p.y).fold(f64::MAX, |a, b| a.min(b));
    let max_x = positions.iter().map(|p| p.x).fold(f64::MIN, |a, b| a.max(b));
    let max_y = positions.iter().map(|p| p.y).fold(f64::MIN, |a, b| a.max(b));

    let padding = BOND_LENGTH / 4.0;
    let start_x = min_x - padding;
    let start_y = min_y - padding;
    let width = max_x - min_x + padding * 2.0;
    let height = max_y - min_y + padding * 2.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="auto" viewBox="{:.3} {:.3} {:.3} {:.3}">"#,
        start_x, start_y, width, height
    ));

    let mut masks = String::new();
    let mut bonds = String::new();
    let mut texts = String::new();

    for (i, atom) in atoms.iter().enumerate() {
        for bond in &atom.bonds {
            if bond.tid > i {
                let offset_type = if bond.kind != BondKind::Double
                    || (atom.bonds.len() > 2 && atoms[bond.tid].bonds.len() < 2)
                    || (atom.bonds.len() < 2 && atoms[bond.tid].bonds.len() > 2)
                {
                    BondOffsetType::Parallel
                } else {
                    if positions[i].y > positions[bond.tid].y {
                        BondOffsetType::CenterOffset(OffsetDirection::Up)
                    } else {
                        BondOffsetType::CenterOffset(OffsetDirection::Down)
                    }
                };
                render_bond(positions[i], positions[bond.tid], bond, &mut bonds, offset_type);
            }
        }

        println!("Atom {} at ({:.3}, {:.3})", atom.kind, positions[i].x, positions[i].y);

        render_text(atom, positions[i], atom.is_heteroatom(), &mut masks, &mut texts);
    }

    if !masks.is_empty() {
        svg.push_str(&format!(
            r#"<mask id="text-mask"><rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="white"/>{}</mask>"#,
            start_x, start_y, width, height, masks
        ));
        svg.push_str(r#"<g mask="url(#text-mask)">"#);
        svg.push_str(&bonds);
        svg.push_str("</g>");
    } else {
        svg.push_str(&bonds);
    }

    svg.push_str(&texts);

    svg.push_str("</svg>");

    svg
}
