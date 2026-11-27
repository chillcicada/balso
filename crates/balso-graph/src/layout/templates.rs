//! Fixed perspective views for ring skeletons that cannot be assembled from
//! regular polygons without atoms landing on each other.
//!
//! Each template stores the skeleton's bonds and coordinates in bond-length
//! units. A template is used when its bonds appear exactly among a ring
//! system's atoms; substituents and fused chemistry reuse the same drawing.

use super::Vec2;

struct Template {
    bonds: &'static [(usize, usize)],
    coordinates: &'static [(f64, f64)],
}

/// Four regular pyrroles pointing their nitrogens inward, joined by meso
/// carbons: the square depiction used for porphyrins.
const PORPHINE: Template = Template {
    bonds: &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 0),
        (5, 6),
        (6, 7),
        (7, 8),
        (8, 9),
        (9, 5),
        (10, 11),
        (11, 12),
        (12, 13),
        (13, 14),
        (14, 10),
        (15, 16),
        (16, 17),
        (17, 18),
        (18, 19),
        (19, 15),
        (20, 4),
        (20, 6),
        (21, 9),
        (21, 11),
        (22, 14),
        (22, 16),
        (23, 19),
        (23, 1),
    ],
    coordinates: &[
        (0.000, 1.474),
        (0.809, 2.062),
        (0.500, 3.013),
        (-0.500, 3.013),
        (-0.809, 2.062),
        (-1.474, 0.000),
        (-2.062, 0.809),
        (-3.013, 0.500),
        (-3.013, -0.500),
        (-2.062, -0.809),
        (0.000, -1.474),
        (-0.809, -2.062),
        (-0.500, -3.013),
        (0.500, -3.013),
        (0.809, -2.062),
        (1.474, 0.000),
        (2.062, -0.809),
        (3.013, -0.500),
        (3.013, 0.500),
        (2.062, 0.809),
        (-1.763, 1.763),
        (-1.763, -1.763),
        (1.763, -1.763),
        (1.763, 1.763),
    ],
};

/// Perspective view with the apex methylene on top and one bond passing in
/// front of another.
const ADAMANTANE: Template = Template {
    bonds: &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 8),
        (7, 9),
        (5, 0),
        (8, 1),
        (9, 3),
    ],
    coordinates: &[
        (0.474, -1.358),
        (-0.666, -0.995),
        (-0.561, -0.184),
        (0.101, 0.810),
        (1.236, 0.437),
        (1.142, -0.371),
        (0.561, 0.184),
        (-0.578, 0.555),
        (-1.236, -0.437),
        (-0.474, 1.358),
    ],
};

/// Two offset squares joined at their corners, the usual drawing of a cube.
const CUBANE: Template = Template {
    bonds: &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 5),
        (5, 6),
        (6, 7),
        (3, 0),
        (5, 0),
        (6, 1),
        (7, 2),
        (7, 4),
    ],
    coordinates: &[
        (-0.697, 0.604),
        (-0.977, -0.027),
        (0.076, 0.394),
        (0.355, 1.025),
        (0.977, 0.027),
        (-0.076, -0.394),
        (-0.355, -1.025),
        (0.697, -0.604),
    ],
};

/// A hexagon through both bridgeheads with the third bridge drawn as a
/// zigzag across its middle.
const BICYCLO_2_2_2_OCTANE: Template = Template {
    bonds: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (5, 0), (7, 2)],
    coordinates: &[
        (0.915, -0.528),
        (0.915, 0.528),
        (0.000, 1.057),
        (-0.915, 0.528),
        (-0.915, -0.528),
        (0.000, -1.057),
        (-0.370, -0.264),
        (0.370, 0.264),
    ],
};

/// Bicyclo[3.2.1]octane seen from the front: the one-atom bridge on top,
/// the three-atom bridge in front, and the two-atom bridge behind.
const BICYCLO_3_2_1_OCTANE: Template = Template {
    bonds: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 0), (0, 7), (7, 4)],
    coordinates: &[
        (-1.055, 0.448),
        (-0.897, -0.554),
        (0.000, -1.029),
        (0.897, -0.554),
        (1.055, 0.448),
        (0.422, 0.026),
        (-0.422, 0.026),
        (0.000, 1.187),
    ],
};

/// Bicyclo[2.2.1]heptane seen from the front: the one-atom bridge on top
/// and one two-atom bridge folded behind the other.
const BICYCLO_2_2_1_HEPTANE: Template = Template {
    bonds: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0), (0, 6), (6, 3)],
    coordinates: &[
        (-1.002, 0.306),
        (-0.580, -0.665),
        (0.580, -0.665),
        (1.002, 0.306),
        (0.401, -0.137),
        (-0.401, -0.137),
        (0.000, 0.991),
    ],
};

/// Bicyclo[3.1.1]heptane as a hexagon with the second one-atom bridge at
/// its center.
const BICYCLO_3_1_1_HEPTANE: Template = Template {
    bonds: &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0), (0, 6), (6, 4)],
    coordinates: &[
        (-0.866, 0.500),
        (-0.866, -0.500),
        (0.000, -1.000),
        (0.866, -0.500),
        (0.866, 0.500),
        (0.000, 1.000),
        (0.000, 0.000),
    ],
};

/// A diamond through both bridgeheads with the third bridge folded inside.
const BICYCLO_1_1_1_PENTANE: Template = Template {
    bonds: &[(0, 1), (1, 2), (2, 3), (3, 4), (3, 0), (4, 1)],
    coordinates: &[
        (0.000, 0.771),
        (-0.732, -0.059),
        (0.000, -0.888),
        (0.732, -0.059),
        (0.000, 0.234),
    ],
};

/// Larger cages first, so a skeleton uses the most specific template.
const TEMPLATES: [&Template; 8] = [
    &PORPHINE,
    &ADAMANTANE,
    &CUBANE,
    &BICYCLO_2_2_2_OCTANE,
    &BICYCLO_3_2_1_OCTANE,
    &BICYCLO_2_2_1_HEPTANE,
    &BICYCLO_3_1_1_HEPTANE,
    &BICYCLO_1_1_1_PENTANE,
];

/// Match the first template found among `system_atoms`, returning one
/// position per matched molecule atom.
pub(super) fn positions(adjacency: &[Vec<usize>], system_atoms: &[usize]) -> Option<Vec<(usize, Vec2)>> {
    TEMPLATES.iter().find_map(|template| {
        let assignment = match_template(adjacency, system_atoms, template)?;
        Some(
            assignment
                .iter()
                .enumerate()
                .map(|(template_atom, &molecule_atom)| {
                    let (x, y) = template.coordinates[template_atom];
                    (molecule_atom, Vec2::new(x, y))
                })
                .collect(),
        )
    })
}

fn match_template(adjacency: &[Vec<usize>], system_atoms: &[usize], template: &Template) -> Option<Vec<usize>> {
    let size = template.coordinates.len();
    if system_atoms.len() < size {
        return None;
    }

    let neighbors = template_adjacency(template);
    let order = search_order(&neighbors);
    let mut assignment = vec![usize::MAX; size];
    let mut used = vec![false; adjacency.len()];
    let mut result = None;

    extend(
        adjacency,
        system_atoms,
        &neighbors,
        &order,
        0,
        &mut assignment,
        &mut used,
        &mut result,
    );
    result
}

fn template_adjacency(template: &Template) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); template.coordinates.len()];
    for &(first, second) in template.bonds {
        neighbors[first].push(second);
        neighbors[second].push(first);
    }
    neighbors
}

fn search_order(neighbors: &[Vec<usize>]) -> Vec<usize> {
    let mut order = vec![0];
    let mut seen = vec![false; neighbors.len()];
    seen[0] = true;
    let mut cursor = 0;
    while cursor < order.len() {
        for &neighbor in &neighbors[order[cursor]] {
            if !seen[neighbor] {
                seen[neighbor] = true;
                order.push(neighbor);
            }
        }
        cursor += 1;
    }
    order
}

#[allow(clippy::too_many_arguments)]
fn extend(
    adjacency: &[Vec<usize>],
    system_atoms: &[usize],
    template_neighbors: &[Vec<usize>],
    order: &[usize],
    depth: usize,
    assignment: &mut [usize],
    used: &mut [bool],
    result: &mut Option<Vec<usize>>,
) {
    if result.is_some() {
        return;
    }
    if depth == order.len() {
        *result = Some(assignment.to_vec());
        return;
    }

    let template_atom = order[depth];
    for &candidate in system_atoms {
        if used[candidate] {
            continue;
        }
        if inside_degree(adjacency, system_atoms, candidate) < template_neighbors[template_atom].len() {
            continue;
        }
        if !agrees(adjacency, template_neighbors, template_atom, candidate, assignment) {
            continue;
        }

        assignment[template_atom] = candidate;
        used[candidate] = true;
        extend(
            adjacency,
            system_atoms,
            template_neighbors,
            order,
            depth + 1,
            assignment,
            used,
            result,
        );
        used[candidate] = false;
        assignment[template_atom] = usize::MAX;

        if result.is_some() {
            return;
        }
    }
}

fn inside_degree(adjacency: &[Vec<usize>], system_atoms: &[usize], atom: usize) -> usize {
    adjacency[atom]
        .iter()
        .filter(|neighbor| system_atoms.binary_search(neighbor).is_ok())
        .count()
}

fn agrees(
    adjacency: &[Vec<usize>],
    template_neighbors: &[Vec<usize>],
    template_atom: usize,
    candidate: usize,
    assignment: &[usize],
) -> bool {
    assignment
        .iter()
        .enumerate()
        .filter(|&(_, &assigned)| assigned != usize::MAX)
        .all(|(other_template_atom, &assigned)| {
            let template_bonded = template_neighbors[template_atom].contains(&other_template_atom);
            let molecule_bonded = adjacency[candidate].contains(&assigned);
            template_bonded == molecule_bonded
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubane_skeleton_matches_the_cube_template() {
        let mut builder = crate::Builder::new();
        balso_parser::read("C12C3C4C1C5C2C3C45", &mut builder).expect("parse");
        let atoms = builder.build();
        let adjacency = super::super::adjacency(&atoms);
        let system: Vec<usize> = (0..atoms.len()).collect();

        assert!(positions(&adjacency, &system).is_some());
    }
}
