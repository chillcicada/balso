//! Layout algorithms for molecular graphs based on SmilesDrawer.
//!
//! This module implements the layout algorithm described in the SmilesDrawer paper,
//! adapted for the Balsa molecular line notation parser.
//!
//! # Layout Pipeline
//!
//! The layout process follows these steps:
//!
//! 1. **initRings** - Close open ring bonds and detect rings using SSSR
//! 2. **position** - Calculate initial vertex positions using recursive algorithm
//! 3. **restoreRingInformation** - Restore ring metadata after positioning
//! 4. **resolvePrimaryOverlaps** - Eliminate exact overlaps of subtrees
//! 5. **resolveSecondaryOverlaps** - Resolve steric clashes through rotation
//! 6. **annotateStereochemistry** - Add stereochemistry markers
//! 7. **initPseudoElements** - Initialize pseudo-element labels
//! 8. **rotateDrawing** - Final rotation for optimal presentation

use std::collections::HashMap;
use std::f64::consts::PI;

use balso_core::BondKind;
use balso_graph::Atom;
use nalgebra::Vector2;

/// Configuration for the layout algorithm.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConfig {
    /// Bond length in pixels
    pub bond_length: f64,
    /// Bond spacing for double/triple bonds
    pub bond_spacing: f64,
    /// Ring size for regular polygons
    pub ring_size: usize,
    /// Angle increment for branching (in radians)
    pub branch_angle: f64,
    /// Number of overlap resolution iterations
    pub overlap_resolution_iterations: usize,
    /// Minimum distance between atoms to avoid overlap
    pub atom_radius: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            bond_length: 20.0,
            bond_spacing: 4.0,
            ring_size: 6,
            branch_angle: PI / 3.0,
            overlap_resolution_iterations: 50,
            atom_radius: 8.0,
        }
    }
}

/// A vertex in the layout graph with position and ring information.
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    /// The original atom data
    pub atom: Atom,
    /// Position of the vertex (0,0 is the origin)
    pub position: Vector2<f64>,
    /// Index into the original atoms vector
    pub id: usize,
    /// Whether this vertex has been positioned
    pub positioned: bool,
    /// The ring(s) this vertex belongs to
    pub rings: Vec<usize>,
    /// The previous vertex in the DFS traversal
    pub previous: Option<usize>,
    /// Current angle for branching
    pub angle: f64,
}

impl Vertex {
    /// Create a new unpositioned vertex
    pub fn new(atom: Atom, id: usize) -> Self {
        Self {
            atom,
            position: Vector2::zeros(),
            id,
            positioned: false,
            rings: Vec::new(),
            previous: None,
            angle: 0.0,
        }
    }
}

/// An edge in the layout graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// The source vertex index
    pub source: usize,
    /// The target vertex index
    pub target: usize,
    /// The bond kind
    pub bond_kind: BondKind,
    /// Whether this edge is in a ring
    pub in_ring: bool,
    /// The ring ID if this edge is in a ring
    pub ring_id: Option<usize>,
}

impl Edge {
    /// Create a new edge
    pub fn new(source: usize, target: usize, bond_kind: BondKind) -> Self {
        Self {
            source,
            target,
            bond_kind,
            in_ring: false,
            ring_id: None,
        }
    }
}

/// A ring in the molecule.
#[derive(Debug, Clone, PartialEq)]
pub struct Ring {
    /// The vertices in this ring (in order)
    pub vertices: Vec<usize>,
    /// The edges in this ring
    pub edges: Vec<usize>,
    /// The center of the ring
    pub center: Vector2<f64>,
    /// The radius of the ring
    pub radius: f64,
    /// Whether this is a bridged ring
    pub is_bridged: bool,
    /// Whether this ring has been positioned
    pub positioned: bool,
    /// Neighboring ring IDs
    pub neighbours: Vec<usize>,
}

impl Ring {
    /// Create a new empty ring
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            center: Vector2::zeros(),
            radius: 0.0,
            is_bridged: false,
            positioned: false,
            neighbours: Vec::new(),
        }
    }

    /// Get the size of the ring (number of vertices)
    pub fn size(&self) -> usize {
        self.vertices.len()
    }
}

impl Default for Ring {
    fn default() -> Self {
        Ring::new()
    }
}

/// A connection between two rings (fused or spiro).
#[derive(Debug, Clone, PartialEq)]
pub struct RingConnection {
    /// The first ring ID
    pub ring_a: usize,
    /// The second ring ID
    pub ring_b: usize,
    /// Shared vertices
    pub shared_vertices: Vec<usize>,
    /// Shared edges
    pub shared_edges: Vec<usize>,
    /// Type of connection: "fused" (2 vertices) or "spiro" (1 vertex)
    pub connection_type: RingConnectionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RingConnectionType {
    /// Two rings share exactly two vertices (fused rings)
    Fused,
    /// Two rings share exactly one vertex (spiro rings)
    Spiro,
    /// No shared vertices (independent rings)
    None,
}

impl RingConnection {
    /// Create a new ring connection
    pub fn new(rings: (usize, usize)) -> Self {
        Self {
            ring_a: rings.0,
            ring_b: rings.1,
            shared_vertices: Vec::new(),
            shared_edges: Vec::new(),
            connection_type: RingConnectionType::None,
        }
    }
}

/// The main graph structure for layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutGraph {
    /// Vertices in the graph
    vertices: Vec<Vertex>,
    /// Edges in the graph
    edges: Vec<Edge>,
    /// Rings in the graph
    rings: Vec<Ring>,
    /// Ring connections
    ring_connections: Vec<RingConnection>,
    /// Configuration
    config: LayoutConfig,
    /// Adjacency list cache: vertex_id -> Vec<(neighbor_id, edge_id)>
    adjacency_cache: HashMap<usize, Vec<(usize, usize)>>,
    /// Edge lookup cache: (min, max) -> edge_id
    edge_lookup_cache: HashMap<(usize, usize), usize>,
}

impl LayoutGraph {
    /// Create a new layout graph from atoms
    pub fn from_atoms(atoms: Vec<Atom>, config: LayoutConfig) -> Self {
        let vertex_count = atoms.len();
        let mut vertices = Vec::with_capacity(vertex_count);

        for (id, atom) in atoms.into_iter().enumerate() {
            vertices.push(Vertex::new(atom, id));
        }

        // Build edges from atom bonds
        let mut edges = Vec::new();
        for (source_id, vertex) in vertices.iter().enumerate() {
            for bond in &vertex.atom.bonds {
                let target_id = bond.tid;
                if source_id < target_id {
                    edges.push(Edge::new(
                        source_id,
                        target_id,
                        bond.kind.clone(),
                    ));
                }
            }
        }

        // Build adjacency cache and edge lookup cache
        let mut adjacency_cache: HashMap<usize, Vec<(usize, usize)>> =
            HashMap::new();
        let mut edge_lookup_cache: HashMap<(usize, usize), usize> =
            HashMap::new();

        for (edge_id, edge) in edges.iter().enumerate() {
            let source = edge.source;
            let target = edge.target;

            // Add to adjacency list
            adjacency_cache
                .entry(source)
                .or_insert_with(Vec::new)
                .push((target, edge_id));
            adjacency_cache
                .entry(target)
                .or_insert_with(Vec::new)
                .push((source, edge_id));

            // Add to edge lookup (store with sorted vertices)
            let key = if source < target {
                (source, target)
            } else {
                (target, source)
            };
            edge_lookup_cache.insert(key, edge_id);
        }

        Self {
            vertices,
            edges,
            rings: Vec::new(),
            ring_connections: Vec::new(),
            config,
            adjacency_cache,
            edge_lookup_cache,
        }
    }

    /// Get a vertex by index
    pub fn vertex(&self, id: usize) -> Option<&Vertex> {
        self.vertices.get(id)
    }

    /// Get a mutable vertex by index
    pub fn vertex_mut(&mut self, id: usize) -> Option<&mut Vertex> {
        self.vertices.get_mut(id)
    }

    /// Get all vertices
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Get all vertices (mutable)
    pub fn vertices_mut(&mut self) -> &mut [Vertex] {
        &mut self.vertices
    }

    /// Get an edge by index
    pub fn edge(&self, id: usize) -> Option<&Edge> {
        self.edges.get(id)
    }

    /// Get all edges
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Get all rings
    pub fn rings(&self) -> &[Ring] {
        &self.rings
    }

    /// Get all rings (mutable)
    pub fn rings_mut(&mut self) -> &mut [Ring] {
        &mut self.rings
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get neighbors of a vertex (using cache)
    pub fn neighbors(&self, vertex_id: usize) -> Vec<(usize, &Edge)> {
        if let Some(adj_list) = self.adjacency_cache.get(&vertex_id) {
            let mut neighbors = Vec::with_capacity(adj_list.len());
            for &(nid, edge_id) in adj_list {
                if let Some(edge) = self.edges.get(edge_id) {
                    neighbors.push((nid, edge));
                }
            }
            neighbors
        } else {
            Vec::new()
        }
    }

    /// Get the edge between two vertices (if it exists) (using cache)
    pub fn get_edge(&self, source: usize, target: usize) -> Option<usize> {
        let key = if source < target {
            (source, target)
        } else {
            (target, source)
        };
        self.edge_lookup_cache.get(&key).copied()
    }

    /// Get edge by ID (helper method)
    pub fn edge_by_id(&self, edge_id: usize) -> Option<&Edge> {
        self.edges.get(edge_id)
    }

    /// Add a ring to the graph
    pub fn add_ring(&mut self, ring: Ring) -> usize {
        let id = self.rings.len();
        self.rings.push(ring);
        id
    }

    /// Add a ring connection
    pub fn add_ring_connection(&mut self, connection: RingConnection) {
        self.ring_connections.push(connection);
    }

    /// Get the configuration
    pub fn config(&self) -> &LayoutConfig {
        &self.config
    }

    /// Get the configuration (mutable)
    pub fn config_mut(&mut self) -> &mut LayoutConfig {
        &mut self.config
    }

    /// Restore ring information after initial positioning
    pub fn restore_ring_information(&mut self) {
        // Collect all ring data first to avoid borrow issues
        let ring_data: Vec<(usize, Vector2<f64>, f64)> = self
            .rings()
            .iter()
            .enumerate()
            .filter_map(|(rid, ring)| {
                if ring.vertices.is_empty() {
                    return None;
                }

                let mut sum = Vector2::zeros();
                let mut count = 0;

                for &vid in &ring.vertices {
                    if let Some(v) = self.vertex(vid) {
                        if v.positioned {
                            sum = sum + v.position;
                            count += 1;
                        }
                    }
                }

                if count > 0 {
                    let center = sum / count as f64;
                    let mut radius_sum = 0.0;
                    for &vid in &ring.vertices {
                        if let Some(v) = self.vertex(vid) {
                            if v.positioned {
                                radius_sum += (v.position - center).magnitude();
                            }
                        }
                    }
                    let radius = radius_sum / count as f64;
                    Some((rid, center, radius))
                } else {
                    None
                }
            })
            .collect();

        // Now update rings with collected data
        for (rid, center, radius) in ring_data {
            if let Some(ring) = self.rings_mut().get_mut(rid) {
                ring.center = center;
                ring.radius = radius;
            }
        }
    }

    /// Calculate the center of a ring from positioned vertices (public for use by ring.rs)
    pub fn calculate_ring_center(&self, ring: &Ring) -> Vector2<f64> {
        let mut sum = Vector2::zeros();
        let mut count = 0;

        for &vid in &ring.vertices {
            if let Some(v) = self.vertex(vid) {
                if v.positioned {
                    sum = sum + v.position;
                    count += 1;
                }
            }
        }

        if count > 0 {
            sum / count as f64
        } else {
            Vector2::zeros()
        }
    }
}

/// The main layout orchestrator.
#[derive(Debug, Clone)]
pub struct Layout {
    graph: LayoutGraph,
}

impl Layout {
    /// Create a new layout from parsed atoms
    pub fn new(atoms: &[Atom]) -> Self {
        Self {
            graph: LayoutGraph::from_atoms(
                atoms.to_vec(),
                LayoutConfig::default(),
            ),
        }
    }

    /// Create a new layout with custom configuration
    pub fn with_config(atoms: &[Atom], config: LayoutConfig) -> Self {
        Self {
            graph: LayoutGraph::from_atoms(atoms.to_vec(), config),
        }
    }

    /// Run the full layout algorithm
    pub fn layout(&mut self) -> &LayoutGraph {
        use self::InitRings;
        use self::Position;
        use self::PrimaryOverlapResolver;
        use self::SecondaryOverlapResolver;

        self.graph.init_rings();
        self.graph.position();
        self.graph.restore_ring_information();
        self.graph.resolve_primary_overlaps();
        self.graph.resolve_secondary_overlaps();
        &self.graph
    }

    /// Get the resulting graph
    pub fn graph(&self) -> &LayoutGraph {
        &self.graph
    }
}

/// Utility functions for geometry
impl LayoutGraph {
    /// Calculate the distance between two vertices
    pub fn distance(&self, a: usize, b: usize) -> f64 {
        let pos_a = &self.vertex(a).unwrap().position;
        let pos_b = &self.vertex(b).unwrap().position;
        (pos_a - pos_b).magnitude()
    }

    /// Calculate the circumradius of a regular polygon with given side length
    pub fn circumradius(side_length: f64, n: usize) -> f64 {
        side_length / (2.0 * (PI / n as f64).sin())
    }

    /// Calculate the midpoint between two points
    pub fn midpoint(a: &Vector2<f64>, b: &Vector2<f64>) -> Vector2<f64> {
        (a + b) / 2.0
    }

    /// Rotate a point around an origin
    pub fn rotate(point: &Vector2<f64>, angle: f64) -> Vector2<f64> {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Vector2::new(
            point.x * cos_a - point.y * sin_a,
            point.x * sin_a + point.y * cos_a,
        )
    }
}

/// Import the submodules
mod init_rings;
mod overlap;
mod position;
mod ring;
mod sssr;

pub use init_rings::InitRings;
pub use overlap::{
    OverlapResolver, PrimaryOverlapResolver, SecondaryOverlapResolver,
};
pub use position::Position;
pub use ring::RingLayout;
pub use sssr::SSSR;
