//! Overlap resolution for molecular layouts.
//!
//! This module implements two types of overlap resolution:
//! 1. **Primary Overlaps** - Exact overlaps where multiple subtrees share a common ring atom
//! 2. **Secondary Overlaps** - Steric clashes that occur when atoms are too close
//!
//! The algorithm rotates subtrees around rotatable bonds to eliminate overlaps.

use super::LayoutGraph;
use std::collections::HashSet;
use std::f64::consts::PI;
use nalgebra::Vector2;

/// Trait for overlap resolution
pub trait OverlapResolver {
    /// Resolve all overlaps in the graph
    fn resolve_overlaps(&mut self);

    /// Calculate overlap score (lower is better)
    fn calculate_overlap_score(&self) -> f64;
}

/// Trait for primary overlap resolution (exact overlaps)
pub trait PrimaryOverlapResolver {
    /// Resolve exact overlaps where multiple subtrees share a common atom
    fn resolve_primary_overlaps(&mut self);
}

/// Trait for secondary overlap resolution (steric clashes)
pub trait SecondaryOverlapResolver {
    /// Resolve steric clashes through rotation
    fn resolve_secondary_overlaps(&mut self);
}

impl OverlapResolver for LayoutGraph {
    fn resolve_overlaps(&mut self) {
        self.resolve_primary_overlaps();
        self.resolve_secondary_overlaps();
    }

    fn calculate_overlap_score(&self) -> f64 {
        let min_dist = self.config.atom_radius * 2.0;
        let min_dist_sq = min_dist * min_dist;

        self.vertices
            .iter()
            .enumerate()
            .skip(1)
            .flat_map(|(i, vi)| {
                self.vertices[..i]
                    .iter()
                    .map(move |vj| (vi, vj))
            })
            .filter_map(|(vi, vj)| {
                let diff = vi.position - vj.position;
                let dist_sq = diff.x * diff.x + diff.y * diff.y;
                if dist_sq < min_dist_sq {
                    Some(min_dist - dist_sq.sqrt())
                } else {
                    None
                }
            })
            .map(|d| d * d)
            .sum()
    }
}

impl PrimaryOverlapResolver for LayoutGraph {
    fn resolve_primary_overlaps(&mut self) {
        // Find overlaps: multiple non-ring neighbors of the same ring atom
        let overlaps = self.find_primary_overlaps();

        for overlap in overlaps {
            self.resolve_exact_overlap(&overlap);
        }
    }
}

impl SecondaryOverlapResolver for LayoutGraph {
    fn resolve_secondary_overlaps(&mut self) {
        // Get rotatable edges
        let rotatable_edges = self.find_rotatable_edges();

        // Iteratively resolve overlaps
        let iterations = self.config.overlap_resolution_iterations;

        for _ in 0..iterations {
            let mut improved = false;

            for edge_id in &rotatable_edges {
                let edge = {
                    let e = self.edges().get(*edge_id);
                    if e.is_none() { continue; }
                    e.unwrap().clone()
                };

                // Calculate current overlap score
                let before_score = self.calculate_overlap_score();

                // Get subtree depths
                let depth_a = self.get_subtree_depth(edge.source, edge.target);
                let depth_b = self.get_subtree_depth(edge.target, edge.source);

                // Rotate the shorter subtree
                if depth_a <= depth_b {
                    let angle = (2.0 * PI - PI / 3.0) / 6.0; // 60 degrees total
                    self.rotate_subtree_around_edge(edge.target, edge.source, angle);
                } else {
                    let angle = (2.0 * PI - PI / 3.0) / 6.0;
                    self.rotate_subtree_around_edge(edge.source, edge.target, angle);
                }

                let after_score = self.calculate_overlap_score();

                if after_score >= before_score {
                    // Revert rotation
                    if depth_a <= depth_b {
                        let angle = (2.0 * PI - PI / 3.0) / 6.0;
                        self.rotate_subtree_around_edge(edge.target, edge.source, -angle);
                    } else {
                        let angle = (2.0 * PI - PI / 3.0) / 6.0;
                        self.rotate_subtree_around_edge(edge.source, edge.target, -angle);
                    }
                } else {
                    improved = true;
                }
            }

            if !improved {
                break;
            }
        }
    }
}

impl LayoutGraph {
    /// Find primary overlaps (multiple non-ring neighbors sharing a ring atom)
    fn find_primary_overlaps(&self) -> Vec<PrimaryOverlap> {
        let mut overlaps: Vec<PrimaryOverlap> = Vec::new();

        for ring in self.rings() {
            for &ring_vertex in &ring.vertices {
                // Get non-ring neighbors
                let non_ring_neighbors: Vec<usize> = self
                    .neighbors(ring_vertex)
                    .iter()
                    .filter(|(nid, _)| {
                        !ring.vertices.contains(nid)
                    })
                    .map(|(nid, _)| *nid)
                    .collect();

                if non_ring_neighbors.len() >= 2 {
                    overlaps.push(PrimaryOverlap {
                        common: ring_vertex,
                        vertices: non_ring_neighbors,
                    });
                }
            }
        }

        overlaps
    }

    /// Resolve an exact overlap by rotating subtrees
    fn resolve_exact_overlap(&mut self, overlap: &PrimaryOverlap) {
        if overlap.vertices.len() != 2 {
            return;
        }

        let a = overlap.vertices[0];
        let b = overlap.vertices[1];
        let common = overlap.common;

        // Calculate rotation angle (use ring size or default to 6)
        let ring_size = self.vertex(common)
            .and_then(|v| v.rings.first())
            .and_then(|rid| self.rings().get(*rid))
            .map(|r| r.size())
            .unwrap_or(6);

        let ring_angle = 2.0 * PI / ring_size as f64;
        let angle = (2.0 * PI - ring_angle) / 6.0;

        // Try positive rotation
        self.rotate_subtree(a, common, angle);
        self.rotate_subtree(b, common, -angle);
        let positive_score = self.calculate_overlap_score();

        // Try negative rotation
        self.rotate_subtree(a, common, -2.0 * angle);
        self.rotate_subtree(b, common, 2.0 * angle);
        let negative_score = self.calculate_overlap_score();

        // Choose the better rotation
        if negative_score < positive_score {
            // Keep negative rotation
            self.rotate_subtree(a, common, 2.0 * angle);
            self.rotate_subtree(b, common, -2.0 * angle);
        }
        // Otherwise keep positive rotation (already applied)
    }

    /// Find rotatable edges (single bonds not in rings)
    fn find_rotatable_edges(&self) -> Vec<usize> {
        let mut rotatable = Vec::new();

        for (edge_id, edge) in self.edges().iter().enumerate() {
            // Check if edge is rotatable (single bond not in ring)
            if edge.bond_kind.bond_order() == 1 && !edge.in_ring {
                // Check if both ends have more than 2 bonds (not terminal)
                let source_degree = self.neighbors(edge.source).len();
                let target_degree = self.neighbors(edge.target).len();

                if source_degree >= 2 && target_degree >= 2 {
                    rotatable.push(edge_id);
                }
            }
        }

        rotatable
    }

    /// Get the depth of a subtree
    fn get_subtree_depth(&self, start: usize, parent: usize) -> usize {
        let mut visited = HashSet::new();
        visited.insert(parent);
        self.dfs_depth(start, &mut visited)
    }

    /// DFS to calculate depth
    fn dfs_depth(&self, current: usize, visited: &mut HashSet<usize>) -> usize {
        visited.insert(current);

        let mut max_depth = 0;
        for (nid, edge) in self.neighbors(current) {
            if !visited.contains(&nid) && !edge.in_ring {
                let depth = self.dfs_depth(nid, visited);
                max_depth = max_depth.max(depth + 1);
            }
        }

        max_depth
    }

    /// Rotate a subtree around an edge (optimized: collect vertices first)
    fn rotate_subtree(&mut self, vertex: usize, pivot: usize, angle: f64) {
        let pivot_pos = self.vertex(pivot)
            .map(|v| v.position)
            .unwrap_or(Vector2::zeros());

        // Get all vertices in the subtree
        let subtree = self.get_subtree_vertices(vertex, pivot);

        // Collect vertex IDs and their positions first, then modify
        let rotations: Vec<(usize, Vector2<f64>)> = subtree
            .iter()
            .filter_map(|&sub_v| {
                self.vertex(sub_v).map(|v| (sub_v, v.position))
            })
            .collect();

        // Apply rotations
        for (sub_v, pos) in rotations {
            if let Some(v) = self.vertex_mut(sub_v) {
                let rel_pos = pos - pivot_pos;
                let rotated = LayoutGraph::rotate(&rel_pos, angle);
                v.position = pivot_pos + rotated;
            }
        }
    }

    /// Rotate subtree around an edge by angle (optimized)
    fn rotate_subtree_around_edge(&mut self, vertex: usize, edge_source: usize, angle: f64) {
        let pivot = self.vertex(edge_source)
            .map(|v| v.position)
            .unwrap_or(Vector2::zeros());

        // Get all vertices in the subtree
        let subtree = self.get_subtree_vertices(vertex, edge_source);

        // Collect vertex IDs and their positions first, then modify
        let rotations: Vec<(usize, Vector2<f64>)> = subtree
            .iter()
            .filter_map(|&sub_v| {
                self.vertex(sub_v).map(|v| (sub_v, v.position))
            })
            .collect();

        // Apply rotations
        for (sub_v, pos) in rotations {
            if let Some(v) = self.vertex_mut(sub_v) {
                let rel_pos = pos - pivot;
                let rotated = LayoutGraph::rotate(&rel_pos, angle);
                v.position = pivot + rotated;
            }
        }
    }

    /// Get all vertices in a subtree
    fn get_subtree_vertices(&self, start: usize, parent: usize) -> HashSet<usize> {
        let mut visited = HashSet::new();
        self.collect_subtree(start, parent, &mut visited);
        visited
    }

    /// Collect subtree vertices recursively
    fn collect_subtree(
        &self,
        current: usize,
        parent: usize,
        visited: &mut HashSet<usize>,
    ) {
        visited.insert(current);

        for (nid, edge) in self.neighbors(current) {
            if nid != parent && !edge.in_ring && !visited.contains(&nid) {
                self.collect_subtree(nid, current, visited);
            }
        }
    }
}

/// A primary overlap (multiple non-ring neighbors of a ring atom)
#[derive(Debug)]
struct PrimaryOverlap {
    /// The common ring vertex
    common: usize,
    /// The non-ring neighbor vertices
    vertices: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutConfig;

    #[test]
    fn test_calculate_overlap_score() {
        let config = LayoutConfig::default();
        let graph = LayoutGraph::from_atoms(vec![], config);

        let score = graph.calculate_overlap_score();
        assert!(score >= 0.0);
    }

    #[test]
    fn test_find_rotatable_edges() {
        let config = LayoutConfig::default();
        let graph = LayoutGraph::from_atoms(vec![], config);

        let rotatable = graph.find_rotatable_edges();
        assert!(rotatable.is_empty()); // Empty graph has no edges
    }
}
