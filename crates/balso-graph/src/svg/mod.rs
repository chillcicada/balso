use std::f64::consts::PI;

use super::Atom;

const BOND_LENGTH: f64 = 60.0;
const START_ANGLE: f64 = -PI / 6.0;

fn position_atoms(atoms: &[Atom]) -> Vec<(f64, f64)> {
    let n = atoms.len();
    if n == 0 {
        return vec![];
    }

    let mut positions = vec![(0.0, 0.0); n];
    let mut visited = vec![false; n];

    fn dfs(
        atom_idx: usize,
        atoms: &[Atom],
        positions: &mut [(f64, f64)],
        visited: &mut [bool],
        angle_from_parent: f64,
        flip: bool,
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
            let angle = if child_count == 1 {
                if flip {
                    angle_from_parent + PI / 3.0
                } else {
                    angle_from_parent - PI / 3.0
                }
            } else {
                match i {
                    0 => angle_from_parent + PI / 3.0,
                    1 => angle_from_parent - PI / 3.0,
                    _ => angle_from_parent,
                }
            };
            positions[neighbor] = (
                positions[atom_idx].0 + BOND_LENGTH * angle.cos(),
                positions[atom_idx].1 + BOND_LENGTH * angle.sin(),
            );
            dfs(neighbor, atoms, positions, visited, angle, !flip);
        }
    }

    dfs(0, atoms, &mut positions, &mut visited, START_ANGLE, true);

    // for i in 0..n {
    //     if !visited[i] {
    //         todo!()
    //     }
    // }

    positions
}

pub fn generate_svg(atoms: &[Atom]) -> String {
    let positions = position_atoms(atoms);

    if positions.is_empty() {
        return String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#);
    }

    let min_x = positions.iter().map(|(x, _)| x).fold(f64::MAX, |a, b| a.min(*b));
    let min_y = positions.iter().map(|(_, y)| y).fold(f64::MAX, |a, b| a.min(*b));
    let max_x = positions.iter().map(|(x, _)| x).fold(f64::MIN, |a, b| a.max(*b));
    let max_y = positions.iter().map(|(_, y)| y).fold(f64::MIN, |a, b| a.max(*b));

    let padding = BOND_LENGTH / 4.0;
    let width = max_x - min_x + padding * 2.0;
    let height = max_y - min_y + padding * 2.0;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        width, height, width, height
    ));

    for (i, atom) in atoms.iter().enumerate() {
        let (x, y) = positions[i];
        let adjusted_x = x - min_x + padding;
        let adjusted_y = y - min_y + padding;

        for bond in &atom.bonds {
            if bond.tid > i {
                let (x2, y2) = positions[bond.tid];
                let adjusted_x2 = x2 - min_x + padding;
                let adjusted_y2 = y2 - min_y + padding;
                svg.push_str(&format!(
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" style="stroke-width:1;stroke-linecap:round;stroke-dasharray:none" stroke="black" />"#,
                    adjusted_x, adjusted_y, adjusted_x2, adjusted_y2
                ));
            }
        }
    }

    svg.push_str("</svg>");

    svg
}
