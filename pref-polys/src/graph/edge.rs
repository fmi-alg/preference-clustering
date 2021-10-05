use crate::utils::MyVec;

#[derive(Debug)]
pub struct Edge {
    pub id: u32,
    pub source_id: u32,
    pub target_id: u32,
    pub edge_costs: Vec<f64>,
    pub replaced_edges: Option<(u32, u32)>,
}

pub fn parse_costs(tokens: &[&str]) -> Vec<f64> {
    tokens.iter().map(|c| c.parse().unwrap()).collect()
}

impl Edge {
    pub fn new(
        id: u32,
        source_id: u32,
        target_id: u32,
        edge_costs: Vec<f64>,
        replaced_edges: Option<(u32, u32)>,
    ) -> Edge {
        Edge {
            id,
            source_id,
            target_id,
            edge_costs,
            replaced_edges,
        }
    }
}

#[derive(Debug)]
pub struct HalfEdge<'a> {
    pub edge_id: u32,
    pub target_id: u32,
    pub edge_costs: &'a [f64],
}

impl<'a> HalfEdge<'a> {
    pub fn new(edge_id: u32, target_id: u32, edge_costs: &'a [f64]) -> HalfEdge<'a> {
        HalfEdge {
            edge_id,
            target_id,
            edge_costs,
        }
    }
}

#[derive(Debug)]
pub struct GraphEdges {
    offset: MyVec<u32>,
    edge_id: MyVec<u32>,
    target_id: MyVec<u32>,
    costs: MyVec<f64>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum EdgeDirection {
    In,
    Out,
}

impl GraphEdges {
    pub fn new(node_count: usize, edges: &[Edge], dim: usize, dir: EdgeDirection) -> Self {
        let edge_count = edges.len();
        let offset = MyVec(vec![0; node_count + 1]);
        let edge_id = MyVec(Vec::with_capacity(edge_count));
        let target_id = MyVec(Vec::with_capacity(edge_count));
        let costs = MyVec(Vec::with_capacity(edge_count * dim));

        let mut me = Self {
            offset,
            edge_id,
            target_id,
            costs,
        };

        match dir {
            EdgeDirection::In => edges.iter().for_each(|e| me.add_edge_in(e)),
            EdgeDirection::Out => edges.iter().for_each(|e| me.add_edge_out(e)),
        }

        for index in 1..me.offset.len() {
            me.offset[index] += me.offset[index - 1];
        }

        debug_assert_eq!(edges.len(), me.edge_id.len());
        debug_assert_eq!(edges.len(), me.target_id.len());
        debug_assert_eq!(edges.len() * dim, me.costs.len());
        debug_assert_eq!(*me.offset.last().unwrap() as usize, me.edge_id.len());

        me
    }

    fn add_edge_in(&mut self, e: &Edge) {
        self.offset[e.target_id + 1] += 1;
        self.edge_id.push(e.id);
        self.target_id.push(e.source_id);
        self.costs.extend_from_slice(&e.edge_costs);
    }

    fn add_edge_out(&mut self, e: &Edge) {
        self.offset[e.source_id + 1] += 1;
        self.edge_id.push(e.id);
        self.target_id.push(e.target_id);
        self.costs.extend_from_slice(&e.edge_costs);
    }

    pub fn get_edges_for_node(&self, node_id: u32, dim: usize) -> HalfEdgeIter {
        let cur = self.offset[node_id] as usize;
        let stop = self.offset[node_id + 1] as usize;
        let dim = dim;

        HalfEdgeIter {
            graph_edges: self,
            dim,
            cur,
            stop,
        }
    }
}

pub struct HalfEdgeIter<'a> {
    graph_edges: &'a GraphEdges,
    dim: usize,
    cur: usize,
    stop: usize,
}

impl<'a> Iterator for HalfEdgeIter<'a> {
    type Item = HalfEdge<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.stop {
            None
        } else {
            // SAFETY: By construction of the offset array which provides
            // self.cur and self.stop all the accessed vectors have length
            // greater than self.stop (or self.stop * self.dim for the costs)
            let (edge_id, target_id, costs) = unsafe {
                (
                    self.graph_edges.edge_id.get_unchecked(self.cur),
                    self.graph_edges.target_id.get_unchecked(self.cur),
                    self.graph_edges
                        .costs
                        .get_unchecked(self.cur * self.dim..(self.cur + 1) * self.dim),
                )
            };

            let halfedge = HalfEdge::new(*edge_id, *target_id, costs);
            self.cur += 1;
            Some(halfedge)
        }
    }
}
