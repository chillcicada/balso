use std::f64::consts::PI;

use super::util::Vec2;

pub const BOND_LENGTH: f64 = 60.0;

#[derive(Debug, Clone, PartialEq)]
pub enum RingType {
    Aromatic,
    Aliphatic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RingOverlapType {
    Fused,
    Spiro,
    Bridged,
}

#[derive(Debug, Clone)]
pub struct RingOverlap {
    pub ring1: usize,
    pub ring2: usize,
    pub shared_atoms: Vec<usize>,
    pub shared_bonds: Vec<usize>,
    pub overlap_type: RingOverlapType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ring {
    pub atom_indices: Vec<usize>,
    pub center: Vec2,
    pub positioned: bool,
    pub ring_type: RingType,
}

impl Ring {
    pub fn new(atom_indices: Vec<usize>) -> Self {
        Self {
            atom_indices,
            center: Vec2 { x: 0.0, y: 0.0 },
            positioned: false,
            ring_type: RingType::Aliphatic,
        }
    }

    pub fn members(&self) -> &[usize] {
        &self.atom_indices
    }

    pub fn size(&self) -> usize {
        self.atom_indices.len()
    }

    pub fn is_member(&self, atom_idx: usize) -> bool {
        self.atom_indices.contains(&atom_idx)
    }

    pub fn mark_positioned(&mut self) {
        self.positioned = true;
    }

    pub fn is_aromatic(&self) -> bool {
        self.ring_type == RingType::Aromatic
    }

    pub fn set_member_positions(&self, center: Vec2, mut set_pos: impl FnMut(usize, f64, f64)) {
        let n = self.atom_indices.len();
        if n == 0 {
            return;
        }

        let angle_step = 2.0 * PI / n as f64;

        for (i, &atom_idx) in self.atom_indices.iter().enumerate() {
            let angle = angle_step * i as f64 - PI / 2.0;
            let x = center.x + BOND_LENGTH * angle.cos();
            let y = center.y + BOND_LENGTH * angle.sin();
            set_pos(atom_idx, x, y);
        }
    }

    pub fn overlaps_with(&self, other: &Ring) -> Option<RingOverlap> {
        let shared_atoms: Vec<usize> = self
            .atom_indices
            .iter()
            .filter(|&&idx| other.is_member(idx))
            .copied()
            .collect();

        if shared_atoms.is_empty() {
            return None;
        }

        let overlap_type = match shared_atoms.len() {
            1 => RingOverlapType::Spiro,
            2 => RingOverlapType::Fused,
            _ => RingOverlapType::Bridged,
        };

        Some(RingOverlap {
            ring1: 0,
            ring2: 1,
            shared_atoms,
            shared_bonds: Vec::new(),
            overlap_type,
        })
    }
}

impl RingOverlap {
    pub fn is_bridge(&self) -> bool {
        self.overlap_type == RingOverlapType::Bridged
    }
}
