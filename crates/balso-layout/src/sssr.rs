//! SSSR (Smallest Set of Smallest Rings) algorithm implementation.
//!
//! This implements the algorithm from SmilesDrawer to find all minimal rings
//! in a molecular graph using the Frobenius formula:
//!
//! n_rings = n_edges - n_vertices + 1
//!
//! The algorithm uses:
//! 1. Build adjacency matrix
//! 2. Floyd-Warshall for distance matrix
//! 3. Find candidate rings (paths where distance(u, v) + distance(v, u) = distance(u, u))
//! 4. Greedy selection of smallest rings

use super::{LayoutGraph, Ring, RingConnection, RingConnectionType};
use std::collections::{HashMap, HashSet};

/// SSSR ring detector
pub struct SSSR<'a> {
    graph: &'a mut LayoutGraph,
}

impl<'a> SSSR<'a> {
    /// Create a new SSSR detector
    pub fn new(graph: &'a mut LayoutGraph) -> Self {
        Self { graph }
    }

    /// Detect all rings in the graph using SSSR algorithm
    pub fn detect_rings(&mut self) -> Vec<usize> {
        let vertex_count = self.graph.vertex_count();
        let edge_count = self.graph.edge_count();

        // Calculate expected number of rings: n_rings = n_edges - n_vertices + 1
        let expected_rings = edge_count.saturating_sub(vertex_count).saturating_add(1);

        // Build adjacency matrix
        let adj_matrix = self.build_adjacency_matrix();

        // Compute distance matrix using Floyd-Warshall
        let dist_matrix = self.floyd_warshall(&adj_matrix);

        // Find candidate rings
        let candidates = self.find_ring_candidates(&dist_matrix);

        // Greedily select smallest rings
        let selected_rings = self.select_smallest_rings(candidates, expected_rings);

        // Create Ring structures and add to graph
        let mut ring_ids = Vec::new();
        for vertices in &selected_rings {
            let ring = Ring::new();
            let ring_id = self.graph.add_ring(ring.clone());
            self.graph.rings_mut()[ring_id].vertices = vertices.clone();
            ring_ids.push(ring_id);
        }

        ring_ids
    }

    /// Build adjacency matrix from graph
    fn build_adjacency_matrix(&self) -> Vec<Vec<usize>> {
        let n = self.graph.vertex_count();
        let mut adj = vec![vec![usize::MAX; n]; n];

        for i in 0..n {
            adj[i][i] = 0;
        }

        for edge in self.graph.edges() {
            adj[edge.source][edge.target] = 1;
            adj[edge.target][edge.source] = 1;
        }

        adj
    }

    /// Floyd-Warshall algorithm to compute shortest paths
    fn floyd_warshall(&self, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
        let n = adj.len();
        let mut dist = adj.to_vec();

        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if dist[i][k] != usize::MAX && dist[k][j] != usize::MAX {
                        let new_dist = dist[i][k] + dist[k][j];
                        if new_dist < dist[i][j] {
                            dist[i][j] = new_dist;
                        }
                    }
                }
            }
        }

        dist
    }

    /// Find candidate rings from distance matrix
    fn find_ring_candidates(&self, dist: &[Vec<usize>]) -> Vec<Vec<usize>> {
        let mut candidates: Vec<Vec<usize>> = Vec::new();
        let n = dist.len();

        for v in 0..n {
            for u in (v + 1)..n {
                if dist[v][u] == usize::MAX {
                    continue;
                }

                // Find all paths from v to u
                let paths = self.find_all_paths(v, u, dist);

                for path in paths {
                    if path.len() >= 3 {
                        // Check if it's a valid ring (path + edge closes the ring)
                        let ring_length = path.len();
                        let ring_size = ring_length + 1; // Including the closing edge

                        // Only consider paths where the direct distance matches the path length
                        if dist[v][u] < ring_size {
                            candidates.push(path);
                        }
                    }
                }
            }
        }

        candidates
    }

    /// Find all simple paths from source to target
    fn find_all_paths(
        &self,
        source: usize,
        target: usize,
        dist: &[Vec<usize>],
    ) -> Vec<Vec<usize>> {
        let mut paths = Vec::new();
        let max_path_len = dist[source][target] * 2;
        let mut visited = HashSet::new();
        visited.insert(source);

        self.dfs_paths(source, target, &mut vec![source], &mut paths, &visited, max_path_len, dist);

        paths
    }

    fn dfs_paths(
        &self,
        current: usize,
        target: usize,
        path: &mut Vec<usize>,
        paths: &mut Vec<Vec<usize>>,
        visited: &HashSet<usize>,
        max_len: usize,
        dist: &[Vec<usize>],
    ) {
        if current == target {
            paths.push(path.clone());
            return;
        }

        if path.len() >= max_len {
            return;
        }

        for (neighbor, _) in self.graph.neighbors(current) {
            if !visited.contains(&neighbor) {
                // Only consider paths that maintain shortest path property
                let current_path_len = path.len();
                if dist[path[0]][neighbor] == current_path_len {
                    let mut new_visited = visited.clone();
                    new_visited.insert(neighbor);
                    path.push(neighbor);
                    self.dfs_paths(
                        target,
                        neighbor,
                        path,
                        paths,
                        &new_visited,
                        max_len,
                        dist,
                    );
                    path.pop();
                }
            }
        }
    }

    /// Select smallest rings greedily
    fn select_smallest_rings(
        &mut self,
        mut candidates: Vec<Vec<usize>>,
        expected: usize,
    ) -> Vec<Vec<usize>> {
        // Sort by ring size (ascending)
        candidates.sort_by(|a, b| a.len().cmp(&b.len()));

        let mut selected: Vec<Vec<usize>> = Vec::new();
        let mut used_vertices: HashSet<usize> = HashSet::new();

        for ring in candidates {
            // Check if ring uses only unused vertices
            let all_unused = ring.iter().all(|v| !used_vertices.contains(v));

            if all_unused {
                selected.push(ring.clone());
                for v in &ring {
                    used_vertices.insert(*v);
                }

                if selected.len() >= expected {
                    break;
                }
            }
        }

        selected
    }
}

/// Detect ring connections between rings (fused, spiro)
pub fn detect_ring_connections(graph: &mut LayoutGraph) {
    let rings = graph.rings().len();

    for i in 0..rings {
        for j in (i + 1)..rings {
            if let (Some(ring_i), Some(ring_j)) = (
                graph.rings().get(i),
                graph.rings().get(j),
            ) {
                let shared_vertices: Vec<usize> = ring_i
                    .vertices
                    .iter()
                    .filter(|v| ring_j.vertices.contains(v))
                    .cloned()
                    .collect();

                if !shared_vertices.is_empty() {
                    let mut connection = RingConnection::new((i, j));
                    connection.shared_vertices = shared_vertices;

                    // Determine connection type
                    if connection.shared_vertices.len() == 2 {
                        connection.connection_type = RingConnectionType::Fused;
                    } else if connection.shared_vertices.len() == 1 {
                        connection.connection_type = RingConnectionType::Spiro;
                    }

                    graph.add_ring_connection(connection);
                }
            }
        }
    }
}

/// Detect bridged ring systems
pub fn detect_bridged_rings(graph: &mut LayoutGraph) {
    // A bridged ring has vertices that belong to multiple rings
    // and the rings share more than 2 vertices or have complex connectivity

    let mut vertex_rings: HashMap<usize, Vec<usize>> = HashMap::new();

    for (ring_id, ring) in graph.rings().iter().enumerate() {
        for &vertex in &ring.vertices {
            vertex_rings
                .entry(vertex)
                .or_insert_with(Vec::new)
                .push(ring_id);
        }
    }

    // Find bridged ring vertices (vertices in 3+ rings)
    let bridged_vertices: Vec<usize> = vertex_rings
        .iter()
        .filter(|(_, rings)| rings.len() >= 3)
        .map(|(&v, _)| v)
        .collect();

    // Mark rings containing bridged vertices as bridged
    for (_, ring) in graph.rings_mut().iter_mut().enumerate() {
        for &bv in &bridged_vertices {
            if ring.vertices.contains(&bv) {
                ring.is_bridged = true;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circumradius() {
        // Hexagon with side length 20
        let r = LayoutGraph::circumradius(20.0, 6);
        assert!((r - 20.0).abs() < 0.001);
    }
}
