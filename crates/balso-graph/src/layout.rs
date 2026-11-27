//! Two-dimensional coordinates for molecular graphs.
//!
//! The layout follows the usual organic-chemistry depiction rules:
//!
//! 1. perceive the smallest set of smallest rings (SSSR),
//! 2. place every ring system as regular polygons, fusing rings across their
//!    shared bond,
//! 3. grow chains and substituents outward at ~120° bond angles,
//! 4. separate dot-disconnected fragments left to right.
//!
//! Coordinates use bond lengths as their unit; the renderer scales them.

use std::collections::{HashMap, HashSet, VecDeque};
use std::f64::consts::PI;

use balso_core::AtomKind;

use super::Atom;

mod templates;

/// A point in layout space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn from_angle(angle: f64) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance(self, other: Vec2) -> f64 {
        (self - other).length()
    }

    pub fn normalized(self) -> Self {
        let length = self.length();
        if length <= f64::EPSILON {
            Self::new(0.0, 0.0)
        } else {
            self * (1.0 / length)
        }
    }

    pub fn rotated(self, angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl std::ops::Div<f64> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

/// The complete result of laying out a molecular graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// One position per atom, in bond-length units.
    pub positions: Vec<Vec2>,
    /// Rings as cyclic atom-index lists.
    pub rings: Vec<Vec<usize>>,
    /// Rings whose atoms are all aromatic (lowercase or bracketed lowercase).
    pub aromatic_rings: Vec<usize>,
    ring_bonds: HashSet<(usize, usize)>,
}

impl Layout {
    /// True for an edge that belongs to at least one perceived ring.
    pub fn is_ring_bond(&self, first: usize, second: usize) -> bool {
        self.ring_bonds.contains(&edge_key(first, second))
    }

    /// Centroid and inradius of a ring (zero for a degenerate ring).
    pub fn ring_geometry(&self, ring_index: usize) -> Option<(Vec2, f64)> {
        let ring = self.rings.get(ring_index)?;
        if ring.len() < 3 {
            return None;
        }

        let center = ring
            .iter()
            .map(|&atom| self.positions[atom])
            .fold(Vec2::default(), |sum, point| sum + point)
            / ring.len() as f64;

        let inradius = ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
            .map(|(&first, &second)| (self.positions[first] + self.positions[second]) / 2.0)
            .map(|midpoint| midpoint.distance(center))
            .fold(f64::INFINITY, f64::min);

        Some((center, inradius))
    }
}

fn edge_key(first: usize, second: usize) -> (usize, usize) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

/// Compute a 2D layout for the graph.
pub fn compute(atoms: &[Atom]) -> Layout {
    let adjacency = adjacency(atoms);
    let rings = find_rings(&adjacency);
    let mut ring_bonds = HashSet::new();
    for ring in &rings {
        for (&first, &second) in ring.iter().zip(ring.iter().cycle().skip(1)).take(ring.len()) {
            ring_bonds.insert(edge_key(first, second));
        }
    }

    let ring_systems = ring_systems(&rings);
    let system_of_ring = system_lookup(&ring_systems, rings.len());
    let components = connected_components(&adjacency);

    let mut positions = vec![Vec2::default(); atoms.len()];
    let mut cursor = 0.0;

    for (component_index, component) in components.iter().enumerate() {
        let mut placer = Placer {
            adjacency: &adjacency,
            rings: &rings,
            ring_systems: &ring_systems,
            system_of_ring: &system_of_ring,
            positions: vec![Vec2::default(); atoms.len()],
            placed: vec![false; atoms.len()],
            templated: false,
        };

        placer.place_component(component);
        placer.relax_component(component);
        placer.center_component(component);

        let (min_x, max_x, min_y, max_y) = placer.component_bounds(component);
        let center_y = (min_y + max_y) / 2.0;
        if component_index > 0 {
            cursor += FRAGMENT_GAP;
        }
        for &atom in component {
            positions[atom] = Vec2::new(
                placer.positions[atom].x - min_x + cursor,
                placer.positions[atom].y - center_y,
            );
        }
        cursor += max_x - min_x;
    }

    apply_cis_trans(atoms, &adjacency, &ring_bonds, &mut positions);

    let aromatic_rings = (0..rings.len())
        .filter(|&ring_index| rings[ring_index].iter().all(|&atom| is_aromatic_atom(&atoms[atom])))
        .collect();

    Layout {
        positions,
        rings,
        aromatic_rings,
        ring_bonds,
    }
}

/// Gap between dot-disconnected fragments, in bond lengths.
const FRAGMENT_GAP: f64 = 1.5;

/// Apply the `\` and `/` directional markers around non-ring double bonds.
/// The default acyclic layout produces the trans arrangement; when the
/// markers ask for cis, one substituent subtree is reflected across the
/// double-bond axis. Reflection is an isometry, so bond lengths are unchanged.
fn apply_cis_trans(
    atoms: &[Atom],
    adjacency: &[Vec<usize>],
    ring_bonds: &HashSet<(usize, usize)>,
    positions: &mut [Vec2],
) {
    let mut bond_kinds: HashMap<(usize, usize), &balso_core::BondKind> = HashMap::new();
    for (atom_index, atom) in atoms.iter().enumerate() {
        for bond in &atom.bonds {
            if bond.tid > atom_index {
                bond_kinds.insert((atom_index, bond.tid), &bond.kind);
            }
        }
    }

    for (&(first, second), kind) in &bond_kinds {
        if !matches!(kind, balso_core::BondKind::Double) || ring_bonds.contains(&(first, second)) {
            continue;
        }

        let left = directional_neighbor(adjacency, &bond_kinds, first, second);
        let right = directional_neighbor(adjacency, &bond_kinds, second, first);
        let (Some((left_neighbor, left_kind)), Some((right_neighbor, right_kind))) = (left, right) else {
            continue;
        };

        let first_position = positions[first];
        let second_position = positions[second];
        let left_side = side_of_line(first_position, second_position, positions[left_neighbor]);
        let right_side = side_of_line(first_position, second_position, positions[right_neighbor]);
        if left_side == 0 || right_side == 0 {
            continue;
        }

        // Same marker on both ends means trans in SMILES; opposite markers cis.
        let want_same_side = left_kind != right_kind;
        let are_same_side = left_side == right_side;
        if are_same_side == want_same_side {
            continue;
        }

        let subtree = collect_subtree(adjacency, right_neighbor, second, first);
        for atom in subtree {
            positions[atom] = reflect_across_line(positions[atom], first_position, second_position);
        }
    }
}

fn directional_neighbor(
    adjacency: &[Vec<usize>],
    bond_kinds: &HashMap<(usize, usize), &balso_core::BondKind>,
    center: usize,
    double_partner: usize,
) -> Option<(usize, balso_core::BondKind)> {
    let mut found = None;
    for &neighbor in &adjacency[center] {
        if neighbor == double_partner {
            continue;
        }
        let key = edge_key(center, neighbor);
        let Some(kind) = bond_kinds.get(&key) else {
            continue;
        };
        if matches!(kind, balso_core::BondKind::Up | balso_core::BondKind::Down) {
            if found.is_some() {
                return None;
            }
            found = Some((neighbor, (*kind).clone()));
        }
    }
    found
}

fn side_of_line(start: Vec2, end: Vec2, point: Vec2) -> i8 {
    let cross = (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x);
    if cross > 1e-8 {
        1
    } else if cross < -1e-8 {
        -1
    } else {
        0
    }
}

fn reflect_across_line(point: Vec2, start: Vec2, end: Vec2) -> Vec2 {
    let direction = end - start;
    let squared_length = direction.x * direction.x + direction.y * direction.y;
    if squared_length < 1e-12 {
        return point;
    }
    let projection = (point - start).x * direction.x + (point - start).y * direction.y;
    let projection = start + direction * (projection / squared_length);
    projection * 2.0 - point
}

fn collect_subtree(adjacency: &[Vec<usize>], root: usize, parent: usize, blocked: usize) -> Vec<usize> {
    let mut seen = vec![false; adjacency.len()];
    seen[parent] = true;
    seen[blocked] = true;
    let mut atoms = Vec::new();
    let mut stack = vec![root];
    seen[root] = true;
    while let Some(atom) = stack.pop() {
        atoms.push(atom);
        for &neighbor in &adjacency[atom] {
            if !seen[neighbor] {
                seen[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }
    atoms
}

fn is_aromatic_atom(atom: &Atom) -> bool {
    match &atom.kind {
        AtomKind::Selection(_) => true,
        AtomKind::Bracket(bracket) => matches!(bracket.symbol, balso_core::Symbol::Selection(_)),
        _ => false,
    }
}

fn adjacency(atoms: &[Atom]) -> Vec<Vec<usize>> {
    let mut adjacency: Vec<Vec<usize>> = atoms
        .iter()
        .map(|atom| {
            let mut neighbors: Vec<usize> = Vec::new();
            for bond in &atom.bonds {
                if bond.tid < atoms.len() && !neighbors.contains(&bond.tid) {
                    neighbors.push(bond.tid);
                }
            }
            neighbors
        })
        .collect();

    // Keep both sides of every edge even if a hand-built graph stores it once.
    for atom in 0..atoms.len() {
        let neighbors = adjacency[atom].clone();
        for neighbor in neighbors {
            if !adjacency[neighbor].contains(&atom) {
                adjacency[neighbor].push(atom);
            }
        }
    }

    adjacency
}

fn connected_components(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; adjacency.len()];
    let mut components = Vec::new();

    for start in 0..adjacency.len() {
        if seen[start] {
            continue;
        }

        let mut component = Vec::new();
        let mut stack = vec![start];
        seen[start] = true;

        while let Some(atom) = stack.pop() {
            component.push(atom);
            for &neighbor in &adjacency[atom] {
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }

        component.sort_unstable();
        components.push(component);
    }

    components
}

// ── Ring perception ──────────────────────────────────────────────────────────

fn find_rings(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let atom_count = adjacency.len();
    if atom_count == 0 {
        return Vec::new();
    }

    let mut edge_indices = HashMap::new();
    for (atom, neighbors) in adjacency.iter().enumerate() {
        for &neighbor in neighbors {
            let key = edge_key(atom, neighbor);
            let next = edge_indices.len();
            edge_indices.entry(key).or_insert(next);
        }
    }

    let components = connected_components(adjacency).len();
    let target = edge_indices.len() + components;
    let target = target.saturating_sub(atom_count);
    if target == 0 {
        return Vec::new();
    }

    let words = edge_indices.len().div_ceil(64);
    let mut seen = HashSet::new();
    let mut candidates: Vec<(Vec<usize>, Vec<u64>)> = Vec::new();

    let mut edges: Vec<((usize, usize), usize)> = edge_indices.iter().map(|(&edge, &index)| (edge, index)).collect();
    edges.sort_by_key(|&(_, index)| index);

    for ((first, second), edge_index) in edges {
        let Some(path) = shortest_path(adjacency, first, second, edge_index) else {
            continue;
        };
        if path.len() < 3 {
            continue;
        }

        let mut bits = vec![0_u64; words];
        for (&start, &end) in path.iter().zip(path.iter().cycle().skip(1)).take(path.len()) {
            let index = edge_indices[&edge_key(start, end)];
            bits[index / 64] |= 1_u64 << (index % 64);
        }
        let index = edge_indices[&edge_key(first, second)];
        bits[index / 64] |= 1_u64 << (index % 64);

        if seen.insert(bits.clone()) {
            candidates.push((path, bits));
        }
    }

    candidates.sort_by(|(first_path, first_bits), (second_path, second_bits)| {
        first_path
            .len()
            .cmp(&second_path.len())
            .then_with(|| first_bits.cmp(second_bits))
    });

    let mut basis: Vec<Vec<u64>> = Vec::new();
    let mut rings = Vec::new();

    for (path, bits) in candidates {
        if add_independent_cycle(&mut basis, bits) {
            rings.push(path);
            if rings.len() == target {
                break;
            }
        }
    }

    rings
}

/// BFS between the endpoints of `excluded_edge`, which is skipped while
/// searching so that the returned path plus the edge closes a cycle.
fn shortest_path(adjacency: &[Vec<usize>], from: usize, to: usize, excluded_edge: usize) -> Option<Vec<usize>> {
    let mut edge_indices = HashMap::new();
    for (atom, neighbors) in adjacency.iter().enumerate() {
        for &neighbor in neighbors {
            let next = edge_indices.len();
            edge_indices.entry(edge_key(atom, neighbor)).or_insert(next);
        }
    }

    let mut parent = vec![None; adjacency.len()];
    parent[from] = Some(from);
    let mut queue = VecDeque::from([from]);

    while let Some(atom) = queue.pop_front() {
        for &neighbor in &adjacency[atom] {
            if edge_indices[&edge_key(atom, neighbor)] == excluded_edge {
                continue;
            }
            if parent[neighbor].is_some() {
                continue;
            }
            parent[neighbor] = Some(atom);

            if neighbor == to {
                let mut path = Vec::new();
                let mut current = to;
                loop {
                    path.push(current);
                    if current == from {
                        break;
                    }
                    current = parent[current]?;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(neighbor);
        }
    }

    None
}

fn add_independent_cycle(basis: &mut Vec<Vec<u64>>, candidate: Vec<u64>) -> bool {
    let mut reduced = candidate;

    for existing in basis.iter() {
        if let Some(pivot) = pivot_bit(existing)
            && bit_is_set(&reduced, pivot)
        {
            xor_assign(&mut reduced, existing);
        }
    }

    let Some(pivot) = pivot_bit(&reduced) else {
        return false;
    };

    for existing in basis.iter_mut() {
        if bit_is_set(existing, pivot) {
            xor_assign(existing, &reduced);
        }
    }

    basis.push(reduced);
    basis.sort_by_key(|bits| pivot_bit(bits).unwrap_or(usize::MAX));
    true
}

fn pivot_bit(bits: &[u64]) -> Option<usize> {
    for (word_index, word) in bits.iter().enumerate() {
        if *word != 0 {
            return Some(word_index * 64 + word.trailing_zeros() as usize);
        }
    }
    None
}

fn bit_is_set(bits: &[u64], bit: usize) -> bool {
    bits[bit / 64] & (1_u64 << (bit % 64)) != 0
}

fn xor_assign(lhs: &mut [u64], rhs: &[u64]) {
    for (left, right) in lhs.iter_mut().zip(rhs.iter()) {
        *left ^= *right;
    }
}

fn ring_systems(rings: &[Vec<usize>]) -> Vec<Vec<usize>> {
    if rings.is_empty() {
        return Vec::new();
    }

    let mut atom_rings: HashMap<usize, Vec<usize>> = HashMap::new();
    for (ring_index, ring) in rings.iter().enumerate() {
        for &atom in ring {
            atom_rings.entry(atom).or_default().push(ring_index);
        }
    }

    let mut neighbors = vec![Vec::new(); rings.len()];
    for ring_indices in atom_rings.values() {
        for &first in ring_indices {
            for &second in ring_indices {
                if first != second && !neighbors[first].contains(&second) {
                    neighbors[first].push(second);
                }
            }
        }
    }

    let mut seen = vec![false; rings.len()];
    let mut systems = Vec::new();

    for start in 0..rings.len() {
        if seen[start] {
            continue;
        }

        let mut system = Vec::new();
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(ring_index) = stack.pop() {
            system.push(ring_index);
            for &neighbor in &neighbors[ring_index] {
                if !seen[neighbor] {
                    seen[neighbor] = true;
                    stack.push(neighbor);
                }
            }
        }

        system.sort_unstable();
        systems.push(system);
    }

    systems
}

fn system_lookup(systems: &[Vec<usize>], ring_count: usize) -> Vec<usize> {
    let mut lookup = vec![usize::MAX; ring_count];
    for (system_index, system) in systems.iter().enumerate() {
        for &ring_index in system {
            lookup[ring_index] = system_index;
        }
    }
    lookup
}

// ── Placement ────────────────────────────────────────────────────────────────

struct Placer<'a> {
    adjacency: &'a [Vec<usize>],
    rings: &'a [Vec<usize>],
    ring_systems: &'a [Vec<usize>],
    system_of_ring: &'a [usize],
    positions: Vec<Vec2>,
    placed: Vec<bool>,
    templated: bool,
}

impl Placer<'_> {
    fn place_component(&mut self, component: &[usize]) {
        let systems: Vec<usize> = self
            .ring_systems
            .iter()
            .enumerate()
            .filter(|(_, system)| {
                system
                    .iter()
                    .any(|&ring| self.rings[ring].iter().any(|atom| component.contains(atom)))
            })
            .map(|(index, _)| index)
            .collect();

        if systems.is_empty() {
            let root = component[0];
            self.positions[root] = Vec2::default();
            self.placed[root] = true;
        } else {
            let system = systems
                .iter()
                .copied()
                .max_by_key(|&index| self.system_priority(index))
                .expect("ring system");
            if !self.place_template(system) {
                self.place_system_initial(system);
            }
        }

        let mut queue: VecDeque<(usize, Option<f64>, f64)> = VecDeque::new();
        for &atom in component {
            if self.placed[atom] {
                queue.push_back((atom, None, 1.0));
            }
        }

        self.grow(&mut queue);
    }

    fn system_priority(&self, system_index: usize) -> (usize, usize, usize) {
        let system = &self.ring_systems[system_index];
        let fused_neighbors = system
            .iter()
            .map(|&ring_index| {
                system
                    .iter()
                    .filter(|&&other| {
                        other != ring_index && rings_share_edge(&self.rings[ring_index], &self.rings[other])
                    })
                    .count()
            })
            .max()
            .unwrap_or(0);
        let atoms = system_atoms(self.rings, system).len();
        (atoms, fused_neighbors, system.len())
    }

    /// Use a fixed perspective drawing when the ring system matches one of
    /// the cage skeletons that regular polygons cannot represent.
    fn place_template(&mut self, system_index: usize) -> bool {
        let atoms = system_atoms(self.rings, &self.ring_systems[system_index]);
        let Some(positions) = templates::positions(self.adjacency, &atoms) else {
            return false;
        };

        for (atom, position) in positions {
            self.positions[atom] = position;
            self.placed[atom] = true;
        }
        self.templated = true;
        true
    }

    fn place_system_initial(&mut self, system_index: usize) {
        let system = self.ring_systems[system_index].clone();
        let seed_ring = system
            .iter()
            .copied()
            .max_by_key(|&ring_index| {
                system
                    .iter()
                    .filter(|&&other| {
                        other != ring_index && rings_share_edge(&self.rings[ring_index], &self.rings[other])
                    })
                    .count()
            })
            .unwrap_or(system[0]);

        self.set_polygon_centered(seed_ring, Vec2::default(), -PI / 2.0);
        self.complete_system(system_index);
    }

    fn place_system_at(&mut self, system_index: usize, anchor: usize, direction: f64) {
        let system = self.ring_systems[system_index].clone();
        let seed_ring = system
            .iter()
            .copied()
            .find(|&ring_index| self.rings[ring_index].contains(&anchor));

        if let Some(seed_ring) = seed_ring
            && !self.ring_fully_placed(seed_ring)
        {
            let radius = circumradius(self.rings[seed_ring].len());
            let center = self.positions[anchor] + Vec2::from_angle(direction) * radius;
            self.set_polygon(seed_ring, center, anchor, direction + PI);
        }

        self.complete_system(system_index);
    }

    fn complete_system(&mut self, system_index: usize) {
        let mut remaining: Vec<usize> = self.ring_systems[system_index]
            .iter()
            .copied()
            .filter(|&ring_index| !self.ring_fully_placed(ring_index))
            .collect();

        loop {
            let mut progress = false;
            remaining.retain(|&ring_index| {
                if self.ring_fully_placed(ring_index) {
                    return false;
                }
                if self.place_ring_from_shared(ring_index) {
                    progress = true;
                }
                !self.ring_fully_placed(ring_index)
            });

            if !progress {
                break;
            }
        }

        for ring_index in remaining {
            if !self.ring_fully_placed(ring_index) {
                self.place_ring_fallback(ring_index);
            }
        }
    }

    fn place_ring_from_shared(&mut self, ring_index: usize) -> bool {
        let ring = self.rings[ring_index].clone();
        let size = ring.len();

        for start in 0..size {
            let first = ring[start];
            let second = ring[(start + 1) % size];
            if self.placed[first] && self.placed[second] {
                self.place_ring_from_edge(ring_index, first, second);
                return true;
            }
        }

        if let Some(&anchor) = ring.iter().find(|&&atom| self.placed[atom]) {
            self.place_ring_from_atom(ring_index, anchor);
            return true;
        }

        false
    }

    fn place_ring_from_edge(&mut self, ring_index: usize, first: usize, second: usize) {
        let ring = self.rings[ring_index].clone();
        let size = ring.len();
        if size < 3 {
            return;
        }

        let first_position = self.positions[first];
        let second_position = self.positions[second];
        let midpoint = (first_position + second_position) / 2.0;
        let edge = second_position - first_position;
        let edge_length = edge.length();
        if edge_length <= f64::EPSILON {
            return;
        }
        let edge_direction = edge / edge_length;
        let normal = Vec2::new(-edge_direction.y, edge_direction.x);
        let apothem = 1.0 / (2.0 * (PI / size as f64).tan());
        let radius = circumradius(size);

        let reference = self.reference_point(&ring, first, second);
        let mut center = midpoint + normal * apothem;
        if let Some(reference) = reference {
            let toward_reference =
                (center - midpoint).x * (reference - midpoint).x + (center - midpoint).y * (reference - midpoint).y;
            if toward_reference > 0.0 {
                center = midpoint - normal * apothem;
            }
        }

        let first_index = ring.iter().position(|&atom| atom == first).unwrap_or(0);
        let second_index = ring.iter().position(|&atom| atom == second).unwrap_or(1);
        let step = 2.0 * PI / size as f64;
        let first_angle = (first_position - center).angle();
        let second_angle = (second_position - center).angle();
        let angle_direction = if normalize_angle(second_angle - first_angle) >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let index_direction = if (first_index + 1) % size == second_index {
            1_isize
        } else if (second_index + 1) % size == first_index {
            -1
        } else {
            1
        };

        for offset in 0..size {
            let index = (first_index as isize + index_direction * offset as isize).rem_euclid(size as isize) as usize;
            let atom = ring[index];
            let angle = first_angle + angle_direction * offset as f64 * step;
            let position = center + Vec2::from_angle(angle) * radius;
            if !self.placed[atom] {
                self.positions[atom] = position;
                self.placed[atom] = true;
            }
        }
    }

    fn place_ring_from_atom(&mut self, ring_index: usize, anchor: usize) {
        let ring = self.rings[ring_index].clone();
        let size = ring.len();
        if size < 3 {
            return;
        }

        let radius = circumradius(size);
        let occupied: Vec<f64> = self.adjacency[anchor]
            .iter()
            .filter(|&&neighbor| self.placed[neighbor])
            .map(|&neighbor| (self.positions[neighbor] - self.positions[anchor]).angle())
            .collect();
        let outward = free_angle(&occupied).unwrap_or(PI / 2.0);
        let center = self.positions[anchor] + Vec2::from_angle(outward) * radius;

        self.set_polygon(ring_index, center, anchor, outward + PI);
    }

    fn place_ring_fallback(&mut self, ring_index: usize) {
        let ring = self.rings[ring_index].clone();
        let size = ring.len();
        if size < 3 {
            return;
        }

        let placed: Vec<usize> = ring.iter().copied().filter(|&atom| self.placed[atom]).collect();
        let center = if placed.is_empty() {
            Vec2::default()
        } else {
            placed
                .iter()
                .map(|&atom| self.positions[atom])
                .fold(Vec2::default(), |sum, point| sum + point)
                / placed.len() as f64
        };
        let radius = circumradius(size);
        let step = 2.0 * PI / size as f64;

        for (offset, &atom) in ring.iter().enumerate() {
            if !self.placed[atom] {
                let angle = -PI / 2.0 + offset as f64 * step;
                self.positions[atom] = center + Vec2::from_angle(angle) * radius;
                self.placed[atom] = true;
            }
        }
    }

    fn set_polygon_centered(&mut self, ring_index: usize, center: Vec2, start_angle: f64) {
        let ring = self.rings[ring_index].clone();
        let size = ring.len();
        if size < 3 {
            return;
        }

        let radius = circumradius(size);
        let step = 2.0 * PI / size as f64;
        for (offset, atom) in ring.iter().enumerate() {
            if !self.placed[*atom] {
                self.positions[*atom] = center + Vec2::from_angle(start_angle + offset as f64 * step) * radius;
                self.placed[*atom] = true;
            }
        }
    }

    fn set_polygon(&mut self, ring_index: usize, center: Vec2, anchor_atom: usize, anchor_angle: f64) {
        let ring = self.rings[ring_index].clone();
        let size = ring.len();
        if size < 3 {
            return;
        }

        let Some(anchor_index) = ring.iter().position(|&atom| atom == anchor_atom) else {
            self.set_polygon_centered(ring_index, center, anchor_angle);
            return;
        };

        let radius = circumradius(size);
        let step = 2.0 * PI / size as f64;

        for offset in 0..size {
            let atom = ring[(anchor_index + offset) % size];
            if !self.placed[atom] {
                self.positions[atom] = center + Vec2::from_angle(anchor_angle + offset as f64 * step) * radius;
                self.placed[atom] = true;
            }
        }
    }

    fn reference_point(&self, ring: &[usize], first: usize, second: usize) -> Option<Vec2> {
        let excluded: HashSet<usize> = ring.iter().copied().collect();
        let mut points = Vec::new();

        for &endpoint in &[first, second] {
            for &neighbor in &self.adjacency[endpoint] {
                if !excluded.contains(&neighbor) && self.placed[neighbor] {
                    points.push(self.positions[neighbor]);
                }
            }
        }

        if points.is_empty() {
            for (atom, &placed) in self.placed.iter().enumerate() {
                if placed && !excluded.contains(&atom) {
                    points.push(self.positions[atom]);
                }
            }
        }

        if points.is_empty() {
            None
        } else {
            Some(points.iter().fold(Vec2::default(), |sum, &point| sum + point) / points.len() as f64)
        }
    }

    fn ring_fully_placed(&self, ring_index: usize) -> bool {
        self.rings[ring_index].iter().all(|&atom| self.placed[atom])
    }

    fn system_fully_placed(&self, system_index: usize) -> bool {
        self.ring_systems[system_index]
            .iter()
            .all(|&ring_index| self.ring_fully_placed(ring_index))
    }

    fn grow(&mut self, queue: &mut VecDeque<(usize, Option<f64>, f64)>) {
        while let Some((atom, incoming, turn)) = queue.pop_front() {
            let unplaced: Vec<usize> = self.adjacency[atom]
                .iter()
                .copied()
                .filter(|&neighbor| !self.placed[neighbor])
                .collect();

            if unplaced.is_empty() {
                continue;
            }

            let directions = self.free_directions(atom, incoming, turn, unplaced.len());

            for (index, &neighbor) in unplaced.iter().enumerate() {
                if self.placed[neighbor] {
                    continue;
                }

                let mut direction = directions[index];
                direction = self.clear_direction(atom, direction, incoming);

                self.positions[neighbor] = self.positions[atom] + Vec2::from_angle(direction);
                self.placed[neighbor] = true;

                let ring_system = self
                    .rings_of_atom(neighbor)
                    .into_iter()
                    .map(|ring_index| self.system_of_ring[ring_index])
                    .find(|&system_index| !self.system_fully_placed(system_index));

                if let Some(system_index) = ring_system {
                    self.place_system_at(system_index, neighbor, direction);
                    for &system_atom in &system_atoms(self.rings, &self.ring_systems[system_index]) {
                        queue.push_back((system_atom, None, -turn));
                    }
                } else {
                    queue.push_back((neighbor, Some(direction), -turn));
                }
            }
        }
    }

    fn rings_of_atom(&self, atom: usize) -> Vec<usize> {
        self.rings
            .iter()
            .enumerate()
            .filter(|(_, ring)| ring.contains(&atom))
            .map(|(index, _)| index)
            .collect()
    }

    fn free_directions(&self, atom: usize, incoming: Option<f64>, turn: f64, count: usize) -> Vec<f64> {
        if count == 0 {
            return Vec::new();
        }

        let occupied: Vec<f64> = self.adjacency[atom]
            .iter()
            .filter(|&&neighbor| self.placed[neighbor])
            .map(|&neighbor| (self.positions[neighbor] - self.positions[atom]).angle())
            .collect();

        if occupied.is_empty() {
            let base = incoming.unwrap_or(-PI / 6.0);
            return spread_around(base, count, PI / 3.0);
        }

        if occupied.len() == 1 {
            let base = normalize_angle(occupied[0] + PI);
            if count == 1 {
                return vec![normalize_angle(base + turn * PI / 3.0)];
            }
            return spread_around(base, count, 2.0 * PI / 3.0 * (count as f64 - 1.0));
        }

        if !occupied.is_empty() {
            let gaps = angular_gaps(&occupied);
            let (start, width) = gaps
                .iter()
                .copied()
                .find(|&(start, width)| !self.gap_enters_ring(atom, normalize_angle(start + width / 2.0)))
                .unwrap_or(gaps[0]);
            if count == 1 {
                return vec![normalize_angle(start + width / 2.0)];
            }
            return spread_around(normalize_angle(start + width / 2.0), count, width * 0.8);
        }

        spread_around(incoming.unwrap_or(-PI / 6.0), count, PI / 3.0)
    }

    fn gap_enters_ring(&self, atom: usize, direction: f64) -> bool {
        let probe = self.positions[atom] + Vec2::from_angle(direction) * 0.6;
        self.rings.iter().any(|ring| {
            ring.contains(&atom)
                && ring.iter().all(|&ring_atom| self.placed[ring_atom])
                && point_in_polygon(
                    probe,
                    &ring
                        .iter()
                        .map(|&ring_atom| self.positions[ring_atom])
                        .collect::<Vec<_>>(),
                )
        })
    }

    fn clear_direction(&self, atom: usize, direction: f64, incoming: Option<f64>) -> f64 {
        let origin = self.positions[atom];
        let clear = |candidate: f64| {
            let point = origin + Vec2::from_angle(candidate);
            self.positions
                .iter()
                .zip(self.placed.iter())
                .enumerate()
                .all(|(index, (position, placed))| index == atom || !placed || position.distance(point) >= CLEARANCE)
        };

        if clear(direction) {
            return direction;
        }

        let turns = [PI / 3.0, -PI / 3.0, 2.0 * PI / 3.0, -2.0 * PI / 3.0];
        for turn in turns {
            let candidate = normalize_angle(direction + turn);
            if clear(candidate) {
                return candidate;
            }
        }

        if let Some(incoming) = incoming {
            let candidate = normalize_angle(incoming + PI - (direction - incoming));
            if clear(candidate) {
                return candidate;
            }
        }

        direction
    }

    /// Enforce ideal bond lengths where ring fusion had to compromise, as in
    /// bridged ring systems where two rings share more than one bond. Regular
    /// rings and chains are already exact, so this pass leaves them untouched.
    fn relax_component(&mut self, component: &[usize]) {
        if self.templated {
            return;
        }

        const ITERATIONS: usize = 80;

        for _ in 0..ITERATIONS {
            for &atom in component {
                for &neighbor in &self.adjacency[atom] {
                    if neighbor < atom || !component.contains(&neighbor) {
                        continue;
                    }

                    let delta = self.positions[neighbor] - self.positions[atom];
                    let distance = delta.length();
                    if distance <= f64::EPSILON {
                        continue;
                    }

                    let correction = (distance - 1.0) / distance * 0.5;
                    self.positions[atom] = self.positions[atom] + delta * correction;
                    self.positions[neighbor] = self.positions[neighbor] - delta * correction;
                }
            }

            for (index, &first) in component.iter().enumerate() {
                for &second in component.iter().skip(index + 1) {
                    if self.adjacency[first].contains(&second) {
                        continue;
                    }

                    let delta = self.positions[second] - self.positions[first];
                    let distance = delta.length();
                    if distance >= NONBONDED_CLEARANCE {
                        continue;
                    }

                    let distance = distance.max(1e-6);
                    let correction = (NONBONDED_CLEARANCE - distance) / distance * 0.5;
                    self.positions[first] = self.positions[first] - delta * correction;
                    self.positions[second] = self.positions[second] + delta * correction;
                }
            }
        }
    }

    fn center_component(&mut self, component: &[usize]) {
        if component.is_empty() {
            return;
        }

        let centroid = component
            .iter()
            .map(|&atom| self.positions[atom])
            .fold(Vec2::default(), |sum, point| sum + point)
            / component.len() as f64;

        for &atom in component {
            self.positions[atom] = self.positions[atom] - centroid;
        }
    }

    fn component_bounds(&self, component: &[usize]) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for &atom in component {
            let position = self.positions[atom];
            min_x = min_x.min(position.x);
            max_x = max_x.max(position.x);
            min_y = min_y.min(position.y);
            max_y = max_y.max(position.y);
        }

        if !min_x.is_finite() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            (min_x, max_x, min_y, max_y)
        }
    }
}

/// Minimum distance between a new atom and any placed atom, in bond lengths.
const CLEARANCE: f64 = 0.7;
/// Minimum distance between non-bonded atoms after relaxation.
const NONBONDED_CLEARANCE: f64 = 0.85;

fn circumradius(ring_size: usize) -> f64 {
    1.0 / (2.0 * (PI / ring_size.max(3) as f64).sin())
}

fn system_atoms(rings: &[Vec<usize>], system: &[usize]) -> Vec<usize> {
    let mut atoms: Vec<usize> = system
        .iter()
        .flat_map(|&ring_index| rings[ring_index].iter().copied())
        .collect();
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn rings_share_edge(first: &[usize], second: &[usize]) -> bool {
    first
        .iter()
        .zip(first.iter().cycle().skip(1))
        .take(first.len())
        .any(|(&start, &end)| {
            second
                .iter()
                .zip(second.iter().cycle().skip(1))
                .take(second.len())
                .any(|(&other_start, &other_end)| {
                    (start == other_start && end == other_end) || (start == other_end && end == other_start)
                })
        })
}

fn normalize_angle(angle: f64) -> f64 {
    let mut angle = angle;
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle <= -PI {
        angle += 2.0 * PI;
    }
    angle
}

fn angular_gaps(angles: &[f64]) -> Vec<(f64, f64)> {
    if angles.is_empty() {
        return Vec::new();
    }

    let mut sorted = angles.to_vec();
    sorted.sort_by(|first, second| first.partial_cmp(second).unwrap_or(std::cmp::Ordering::Equal));

    let mut gaps: Vec<(f64, f64)> = (0..sorted.len())
        .map(|index| {
            let start = sorted[index];
            let end = if index + 1 < sorted.len() {
                sorted[index + 1]
            } else {
                sorted[0] + 2.0 * PI
            };
            (start, end - start)
        })
        .collect();

    gaps.sort_by(|first, second| second.1.partial_cmp(&first.1).unwrap_or(std::cmp::Ordering::Equal));
    gaps
}

fn largest_gap(angles: &[f64]) -> Option<(f64, f64)> {
    angular_gaps(angles).into_iter().next()
}

fn point_in_polygon(point: Vec2, corners: &[Vec2]) -> bool {
    let mut inside = false;
    for index in 0..corners.len() {
        let start = corners[index];
        let end = corners[(index + 1) % corners.len()];
        if (start.y > point.y) != (end.y > point.y)
            && point.x < start.x + (point.y - start.y) / (end.y - start.y) * (end.x - start.x)
        {
            inside = !inside;
        }
    }
    inside
}

fn free_angle(occupied: &[f64]) -> Option<f64> {
    largest_gap(occupied).map(|(start, width)| start + width / 2.0)
}

fn spread_around(center: f64, count: usize, width: f64) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![normalize_angle(center)];
    }
    (0..count)
        .map(|index| {
            let offset = -width / 2.0 + width * index as f64 / (count - 1) as f64;
            normalize_angle(center + offset)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::Builder;
    use balso_parser::read;

    fn graph(smiles: &str) -> Vec<Atom> {
        let mut builder = Builder::new();
        read(smiles, &mut builder).expect("parse");
        builder.build()
    }

    fn bond_lengths(layout: &Layout, atoms: &[Atom]) -> Vec<f64> {
        let mut lengths = Vec::new();
        for (index, atom) in atoms.iter().enumerate() {
            for bond in &atom.bonds {
                if bond.tid > index {
                    lengths.push(layout.positions[index].distance(layout.positions[bond.tid]));
                }
            }
        }
        lengths
    }

    #[test]
    fn chain_has_regular_bond_lengths() {
        let atoms = graph("CCCCCCO");
        let layout = compute(&atoms);

        for length in bond_lengths(&layout, &atoms) {
            assert!((length - 1.0).abs() < 1e-9, "{length}");
        }
    }

    #[test]
    fn benzene_is_a_closed_regular_ring() {
        let atoms = graph("c1ccccc1");
        let layout = compute(&atoms);

        assert_eq!(layout.rings.len(), 1);
        assert_eq!(layout.ring_bonds.len(), 6);
        assert_eq!(layout.aromatic_rings, vec![0]);

        for length in bond_lengths(&layout, &atoms) {
            assert!((length - 1.0).abs() < 1e-9, "{length}");
        }

        let (center, _) = layout.ring_geometry(0).expect("ring");
        for &position in &layout.positions {
            assert!((position.distance(center) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn fused_rings_share_one_edge() {
        let atoms = graph("c1ccc2ccccc2c1");
        let layout = compute(&atoms);

        assert_eq!(layout.rings.len(), 2);

        let shared = layout
            .rings
            .iter()
            .flat_map(|ring| {
                ring.iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                    .map(|(&first, &second)| (first.min(second), first.max(second)))
            })
            .fold(HashMap::new(), |mut counts, edge| {
                *counts.entry(edge).or_insert(0) += 1;
                counts
            })
            .into_iter()
            .filter(|(_, count)| *count == 2)
            .count();

        assert_eq!(shared, 1);

        for (first, first_position) in layout.positions.iter().enumerate() {
            for second_position in layout.positions.iter().skip(first + 1) {
                assert!(first_position.distance(*second_position) > 0.9);
            }
        }
    }

    #[test]
    fn disconnected_fragments_do_not_overlap() {
        let atoms = graph("C.CC");
        let layout = compute(&atoms);

        assert!(layout.positions[0].x < layout.positions[1].x);
        assert!(layout.positions[0].distance(layout.positions[1]) > 1.5);
    }

    #[test]
    fn spiro_rings_keep_unit_bonds() {
        let atoms = graph("C1CCC2(C1)CCCC2");
        let layout = compute(&atoms);

        assert_eq!(layout.rings.len(), 2);
        for length in bond_lengths(&layout, &atoms) {
            assert!((length - 1.0).abs() < 1e-9, "{length}");
        }
    }

    #[test]
    fn bridged_rings_keep_bonds_near_unit_and_atoms_apart() {
        let atoms = graph("C1CC2CCC1C2");
        let layout = compute(&atoms);

        assert_eq!(layout.rings.len(), 2);
        for length in bond_lengths(&layout, &atoms) {
            assert!((0.7..=1.3).contains(&length), "{length}");
        }
        for (first, first_position) in layout.positions.iter().enumerate() {
            for second_position in layout.positions.iter().skip(first + 1) {
                assert!(first_position.distance(*second_position) > 0.5);
            }
        }
    }

    #[test]
    fn directional_bonds_choose_cis_or_trans() {
        let trans = compute(&graph("C/C=C/C"));
        let cis = compute(&graph("C/C=C\\C"));

        let axis_start = trans.positions[1];
        let axis_end = trans.positions[2];
        let trans_left = side_of_line(axis_start, axis_end, trans.positions[0]);
        let trans_right = side_of_line(axis_start, axis_end, trans.positions[3]);
        let cis_left = side_of_line(axis_start, axis_end, cis.positions[0]);
        let cis_right = side_of_line(axis_start, axis_end, cis.positions[3]);

        assert_eq!(trans_left, -trans_right);
        assert_eq!(cis_left, cis_right);
    }

    #[test]
    fn aromatic_atoms_are_detected() {
        let atoms = graph("c1ccncc1");
        let layout = compute(&atoms);

        assert_eq!(layout.aromatic_rings, vec![0]);
    }
}
