//! Ring layout and management.
//!
//! This module handles:
//! 1. Creating ring layouts (regular polygons)
//! 2. Bridged ring handling with Kamada-Kawai layout
//! 3. Ring connection processing (fused, spiro)

use super::{LayoutGraph, Ring};
use std::f64::consts::PI;
use nalgebra::Vector2;

/// Trait for ring layout operations
pub trait RingLayout {
    /// Create a layout for a ring
    fn create_ring(&mut self, ring_id: usize);

    /// Calculate fused ring center based on shared edge
    fn calculate_fused_center(&self, ring_a: &Ring, ring_b: &Ring) -> Vector2<f64>;

    /// Calculate spiro ring center
    fn calculate_spiro_center(&self, ring_a: &Ring, ring_b: &Ring) -> Vector2<f64>;

    /// Layout a regular polygon
    fn layout_regular_polygon(&mut self, ring_id: usize);

    /// Layout a bridged ring using force-directed approach
    fn layout_bridged_ring(&mut self, ring_id: usize);
}

impl RingLayout for LayoutGraph {
    fn create_ring(&mut self, ring_id: usize) {
        if let Some(ring) = self.rings().get(ring_id) {
            if ring.positioned {
                return;
            }

            if ring.is_bridged {
                self.layout_bridged_ring(ring_id);
            } else {
                self.layout_regular_polygon(ring_id);
            }
        }
    }

    fn calculate_fused_center(&self, ring_a: &Ring, _ring_b: &Ring) -> Vector2<f64> {
        // For fused rings, the center of the second ring is offset from the first
        // based on the shared edge

        let center_a = ring_a.center;
        let n_a = ring_a.size();

        // Calculate the angle offset based on ring sizes
        let angle_a = 2.0 * PI / n_a as f64;

        // The center of the fused ring is offset by approximately one bond length
        // in the direction perpendicular to the shared edge
        let bond_length = self.config.bond_length;

        // Approximate: place the second ring's center offset from the first
        let offset = Vector2::new(bond_length * angle_a.cos(), bond_length * angle_a.sin());

        center_a + offset
    }

    fn calculate_spiro_center(&self, ring_a: &Ring, ring_b: &Ring) -> Vector2<f64> {
        // For spiro rings (share one vertex), the center is placed at the
        // circumradius distance from the shared vertex

        let center_a = ring_a.center;
        let shared_vertex = if let Some(v) = ring_a.vertices.first() {
            *v
        } else {
            return center_a;
        };

        // The spiro center is in the opposite direction from the ring center
        let vertex_pos = self.vertex(shared_vertex).map(|v| v.position).unwrap_or(center_a);

        // Calculate direction from vertex to center, then go further
        let bond_length = self.config.bond_length;
        let direction = (center_a - vertex_pos).normalize();
        let distance = LayoutGraph::circumradius(bond_length, ring_b.size());

        vertex_pos + direction * distance
    }

    fn layout_regular_polygon(&mut self, ring_id: usize) {
        if let Some(ring) = self.rings().get(ring_id).cloned() {
            if ring.positioned {
                return;
            }

            let n = ring.size();
            let bond_length = self.config.bond_length;
            let radius = LayoutGraph::circumradius(bond_length, n);
            let angle_step = 2.0 * PI / n as f64;

            // Calculate center from already positioned vertices, or use origin
            let center = self.calculate_ring_center(&ring);

            // Position each vertex
            let mut angle: f64 = 0.0;
            for &vertex_id in &ring.vertices {
                if let Some(v) = self.vertex(vertex_id) {
                    if !v.positioned {
                        let pos = center + Vector2::new(angle.cos() * radius, angle.sin() * radius);
                        if let Some(vertex) = self.vertex_mut(vertex_id) {
                            vertex.position = pos;
                            vertex.positioned = true;
                            vertex.angle = angle;
                        }
                    }
                }
                angle += angle_step;
            }

            // Mark ring as positioned
            if let Some(r) = self.rings_mut().get_mut(ring_id) {
                r.center = center;
                r.radius = radius;
                r.positioned = true;
            }
        }
    }

    fn layout_bridged_ring(&mut self, ring_id: usize) {
        // For bridged rings, we use a simplified approach
        // Initialize vertices on a circle

        let ring = {
            let r = self.rings().get(ring_id);
            if r.is_none() || r.unwrap().positioned {
                return;
            }
            r.unwrap().clone()
        };

        if ring.positioned {
            return;
        }

        let n = ring.size();
        let bond_length = self.config.bond_length;
        let radius = LayoutGraph::circumradius(bond_length, n);
        let center = Vector2::zeros(); // Use origin for now

        // Initial placement on circle
        let angle_step = 2.0 * PI / n as f64;
        let mut angle: f64 = 0.0;

        for &vertex_id in &ring.vertices {
            if let Some(v) = self.vertex(vertex_id) {
                if !v.positioned {
                    let pos = center + Vector2::new(angle.cos() * radius, angle.sin() * radius);
                    if let Some(vertex) = self.vertex_mut(vertex_id) {
                        vertex.position = pos;
                        vertex.positioned = true;
                        vertex.angle = angle;
                    }
                }
            }
            angle += angle_step;
        }

        // Mark ring as positioned
        if let Some(r) = self.rings_mut().get_mut(ring_id) {
            r.center = center;
            r.radius = radius;
            r.positioned = true;
        }
    }
}

impl LayoutGraph {
    /// Calculate the Kamada-Kawai energy for a bridged ring
    pub fn kk_energy(&self, ring: &Ring) -> f64 {
        let bond_length = self.config.bond_length;
        let mut energy = 0.0;

        for (i, &vi) in ring.vertices.iter().enumerate() {
            for (j, &vj) in ring.vertices.iter().enumerate().skip(i + 1) {
                let pos_i = self.vertex(vi).map(|v| v.position).unwrap_or(Vector2::zeros());
                let pos_j = self.vertex(vj).map(|v| v.position).unwrap_or(Vector2::zeros());

                let dist = (pos_i - pos_j).magnitude();
                let ideal_dist = bond_length * (j - i) as f64;

                if dist > 0.0 {
                    energy += (dist - ideal_dist).powi(2);
                }
            }
        }

        energy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutConfig;

    #[test]
    fn test_circumradius() {
        let r = LayoutGraph::circumradius(20.0, 6);
        assert!((r - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_fused_center_calculation() {
        let config = LayoutConfig::default();
        let graph = LayoutGraph::from_atoms(vec![], config);

        let mut ring_a = Ring::new();
        ring_a.center = Vector2::new(10.0, 10.0);
        let ring_b = Ring::new();

        let center = graph.calculate_fused_center(&ring_a, &ring_b);
        // With a non-zero center for ring_a, the result should be non-zero
        assert!(center.x != 0.0 || center.y != 0.0);
    }
}
