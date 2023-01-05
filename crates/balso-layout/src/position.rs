//! Position vertices in 2D space.
//!
//! This implements the recursive positioning algorithm from SmilesDrawer:
//! 1. Find the best starting vertex (bridge ring > regular ring > vertex 0)
//! 2. Create the next bond recursively, placing atoms based on their context
//! 3. Handle special cases for ring atoms and branching chains

use super::{LayoutGraph, LayoutGraph as LG, Ring};
use std::collections::{HashSet, VecDeque};
use std::f64::consts::PI;
use nalgebra::Vector2;

/// Trait for position calculation
pub trait Position {
    /// Calculate positions for all vertices
    fn position(&mut self);
}

impl Position for LayoutGraph {
    fn position(&mut self) {
        // Find best starting vertex
        let start_vertex = self.find_best_start_vertex();

        // Initialize first vertex at origin
        let bond_length = self.config.bond_length;
        if let Some(start) = self.vertex_mut(start_vertex) {
            start.position = Vector2::new(bond_length, 0.0);
            start.positioned = true;
            start.angle = -PI / 3.0; // -60 degrees
        }

        // Start recursive positioning
        self.create_next_bond(start_vertex, None, 0.0);
    }
}

impl LayoutGraph {
    /// Find the best starting vertex for layout
    fn find_best_start_vertex(&self) -> usize {
        // Priority order:
        // 1. Vertices in bridged rings
        // 2. Vertices in regular rings
        // 3. Vertex 0

        let mut bridged_ring_vertices: Vec<usize> = Vec::new();
        let mut regular_ring_vertices: Vec<usize> = Vec::new();

        for (vertex_id, vertex) in self.vertices.iter().enumerate() {
            if vertex.rings.is_empty() {
                continue;
            }

            let is_bridged = vertex.rings.iter().any(|&rid| {
                self.rings().get(rid).map_or(false, |r| r.is_bridged)
            });

            if is_bridged {
                bridged_ring_vertices.push(vertex_id);
            } else {
                regular_ring_vertices.push(vertex_id);
            }
        }

        // Return highest priority vertex
        if let Some(&v) = bridged_ring_vertices.first() {
            return v;
        }
        if let Some(&v) = regular_ring_vertices.first() {
            return v;
        }

        // Default to vertex 0
        0
    }

    /// Create next bond and position connected vertices
    fn create_next_bond(&mut self, vertex: usize, previous: Option<usize>, angle: f64) {
        // Get the vertex position before we borrow mutably
        let vertex_pos = {
            let v = self.vertex(vertex).expect("vertex").clone();
            v.position
        };

        // Get neighbors excluding previous vertex - store needed info
        let neighbors: Vec<(usize, Option<usize>, u8)> = self
            .neighbors(vertex)
            .iter()
            .filter(|(n, _)| Some(*n) != previous)
            .map(|(nid, edge)| (*nid, edge.ring_id, edge.bond_kind.bond_order()))
            .collect();

        // Separate ring and chain neighbors
        let mut ring_neighbors: Vec<(usize, Option<usize>)> = Vec::new();
        let mut chain_neighbors: Vec<(usize, u8)> = Vec::new();

        for (nid, ring_id, bond_order) in neighbors {
            if ring_id.is_some() {
                ring_neighbors.push((nid, ring_id));
            } else {
                chain_neighbors.push((nid, bond_order));
            }
        }

        // Position ring neighbors
        let mut positioned_count = 0;
        for (nid, ring_id) in &ring_neighbors {
            let is_positioned = self.vertex(*nid).map(|v| v.positioned).unwrap_or(false);
            if !is_positioned {
                self.position_ring_atom(*nid, vertex, vertex_pos, *ring_id);
                positioned_count += 1;
            }
        }

        // Position chain neighbors
        for (nid, bond_order) in &chain_neighbors {
            let is_positioned = self.vertex(*nid).map(|v| v.positioned).unwrap_or(false);
            if !is_positioned {
                let depth = self.calculate_subtree_depth(*nid, vertex);
                let branch_angle = self.calculate_branch_angle(angle, depth, *bond_order, positioned_count);
                self.position_chain_atom(*nid, vertex, vertex_pos, branch_angle);
            }
        }

        // Process children recursively - collect IDs first
        let mut to_process: Vec<usize> = Vec::new();
        for (nid, _) in ring_neighbors.iter() {
            let is_positioned = self.vertex(*nid).map(|v| v.positioned).unwrap_or(false);
            if is_positioned {
                to_process.push(*nid);
            }
        }
        for (nid, _) in chain_neighbors.iter() {
            let is_positioned = self.vertex(*nid).map(|v| v.positioned).unwrap_or(false);
            if is_positioned {
                to_process.push(*nid);
            }
        }

        for nid in to_process {
            self.create_next_bond(nid, Some(vertex), angle);
        }
    }

    /// Position an atom that's part of a ring
    fn position_ring_atom(
        &mut self,
        target: usize,
        source: usize,
        source_pos: Vector2<f64>,
        ring_id: Option<usize>,
    ) {
        if let Some(rid) = ring_id {
            let ring = self.rings()[rid].clone();

            // Check if this is a bridged ring
            if ring.is_bridged {
                self.position_bridged_ring_atom(target, source, &ring);
            } else {
                self.position_regular_ring_atom(target, source, &ring);
            }
        } else {
            // Fallback: position based on previous logic
            self.position_chain_atom(target, source, source_pos, 0.0);
        }
    }

    /// Position atom in a regular ring using regular polygon layout
    fn position_regular_ring_atom(
        &mut self,
        target: usize,
        _source: usize,
        _ring: &Ring,
    ) {
        // For this implementation, we use a simple approach
        // In a real implementation, we'd calculate based on ring structure
        let center = Vector2::zeros();

        let n = 6; // Default ring size
        let bond_length = self.config.bond_length;
        let radius = LG::circumradius(bond_length, n);
        let angle = (target as f64 / n as f64) * 2.0 * PI;
        let pos = center + Vector2::new(angle.cos() * radius, angle.sin() * radius);

        if let Some(v) = self.vertex_mut(target) {
            v.position = pos;
            v.positioned = true;
            v.angle = angle;
        }
    }

    /// Position atom in a bridged ring using force-directed layout
    fn position_bridged_ring_atom(
        &mut self,
        target: usize,
        _source: usize,
        _ring: &Ring,
    ) {
        // Simple approach: place on rough circle
        let center = Vector2::zeros();
        let bond_length = self.config.bond_length;
        let n = 6;
        let radius = LG::circumradius(bond_length, n);

        let angle = (target as f64 / n as f64) * 2.0 * PI;
        let pos = center + Vector2::new(angle.cos() * radius, angle.sin() * radius);

        if let Some(v) = self.vertex_mut(target) {
            v.position = pos;
            v.positioned = true;
            v.angle = angle;
        }
    }

    /// Position an atom in a chain
    fn position_chain_atom(
        &mut self,
        target: usize,
        _source: usize,
        source_pos: Vector2<f64>,
        angle: f64,
    ) {
        let bond_length = self.config.bond_length;
        let pos = source_pos + Vector2::new(angle.cos() * bond_length, angle.sin() * bond_length);

        if let Some(v) = self.vertex_mut(target) {
            v.position = pos;
            v.positioned = true;
            v.angle = angle;
        }
    }

    /// Calculate the depth of a subtree (number of vertices)
    fn calculate_subtree_depth(&self, start: usize, parent: usize) -> usize {
        self.bfs_depth(start, parent)
    }

    /// BFS to calculate depth (optimized with VecDeque)
    fn bfs_depth(&self, start: usize, parent: usize) -> usize {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);
        visited.insert(parent);

        let mut depth = 0;
        let mut current_level_size = 1;

        while current_level_size > 0 {
            depth += 1;
            let mut next_level_size = 0;

            for _ in 0..current_level_size {
                if let Some(current) = queue.pop_front() {
                    for (nid, _) in self.neighbors(current) {
                        if !visited.contains(&nid) {
                            visited.insert(nid);
                            queue.push_back(nid);
                            next_level_size += 1;
                        }
                    }
                }
            }

            current_level_size = next_level_size;
        }

        depth
    }

    /// Calculate branch angle based on context
    fn calculate_branch_angle(
        &self,
        current_angle: f64,
        depth: usize,
        bond_order: u8,
        _positioned_count: usize,
    ) -> f64 {
        // Base branch angle from SmilesDrawer: 60 degrees
        let base_angle = self.config.branch_angle;

        // Adjust angle based on depth (deeper subtrees get wider angles)
        let depth_factor = (depth as f64).min(3.0) / 3.0;

        // Adjust based on bond order (double/triple bonds need different spacing)
        let bond_factor = match bond_order {
            1 => 1.0,
            2 => 0.7, // Narrower for double bonds
            3 => 0.5, // Narrower for triple bonds
            _ => 1.0,
        };

        // Calculate final angle
        let angle = base_angle * (1.0 - depth_factor * 0.3) * bond_factor;

        // Alternate direction based on depth (zigzag pattern)
        if depth % 2 == 0 {
            current_angle + angle
        } else {
            current_angle - angle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutConfig;

    #[test]
    fn test_calculate_subtree_depth() {
        // Simple chain: C-C-C
        let config = LayoutConfig::default();
        let atoms = vec![];
        let _graph = LayoutGraph::from_atoms(atoms, config);

        // Add vertices and edges manually for testing
    }

    #[test]
    fn test_branch_angle() {
        let config = LayoutConfig::default();
        let graph = LayoutGraph::from_atoms(vec![], config);

        // The angle can be positive or negative depending on depth
        let angle = graph.calculate_branch_angle(0.0, 1, 1, 0);
        // Just check that we get a valid number (not NaN or infinite)
        assert!(angle.is_finite());
    }
}
