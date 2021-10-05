pub use edge::Edge;
pub use node::Node;

use edge::{EdgeDirection, GraphEdges, HalfEdgeIter};

use crate::utils::MyVec;
use std::{collections::HashMap, time::Instant};

pub mod dijkstra;
mod edge;
mod node;
pub mod path;

#[derive(Debug)]
pub struct Graph {
    pub nodes: MyVec<Node>,
    pub edges: MyVec<Edge>,
    pub dim: u32,
    edges_in: GraphEdges,
    edges_out: GraphEdges,
}

impl Graph {
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Graph {
        println!("Constructing graph...");
        let mut nodes = MyVec(nodes);
        let mut edges = MyVec(edges);
        let dim = edges.first().map(|e| e.edge_costs.len()).unwrap_or(0) as u32;

        nodes.sort_by_key(|n| n.ch_level);
        let mut id_map = HashMap::new();
        for (i, n) in nodes.0.iter_mut().enumerate() {
            id_map.insert(n.id, i as u32);
            n.id = i as u32;
        }

        edges.iter_mut().for_each(|e| {
            e.source_id = id_map[&e.source_id];
            e.target_id = id_map[&e.target_id]
        });

        edges.sort_by(|a, b| {
            a.source_id.cmp(&b.source_id).then_with(|| {
                // high level nodes first, so that a loop can stop early
                nodes[b.target_id]
                    .ch_level
                    .cmp(&nodes[a.target_id].ch_level)
            })
        });

        let edges_out = GraphEdges::new(nodes.len(), &edges, dim as usize, EdgeDirection::Out);

        edges.sort_by(|a, b| {
            a.target_id.cmp(&b.target_id).then_with(|| {
                nodes[b.source_id]
                    .ch_level
                    .cmp(&nodes[a.source_id].ch_level)
            })
        });

        let edges_in = GraphEdges::new(nodes.len(), &edges, dim as usize, EdgeDirection::In);

        // finish offset arrays
        // for index in 1..offsets_out.len() {
        //     offsets_out[index] += offsets_out[index - 1];
        //     offsets_in[index] += offsets_in[index - 1];
        // }

        // sort edges by id
        edges.sort_by(|a, b| a.id.cmp(&b.id));

        Graph {
            nodes,
            edges,
            dim,
            edges_in,
            edges_out,
        }
    }

    pub fn edges_of(&self, node_id: u32, dir: EdgeDirection) -> HalfEdgeIter {
        match dir {
            EdgeDirection::In => self.edges_in.get_edges_for_node(node_id, self.dim as usize),
            EdgeDirection::Out => self
                .edges_out
                .get_edges_for_node(node_id, self.dim as usize),
        }
    }

    pub fn unpack_edge(&self, edge: u32) -> Vec<u32> {
        if let Some((edge1, edge2)) = self.edges[edge].replaced_edges {
            if self.edges[edge1].source_id != self.edges[edge].source_id {
                panic!(
                    "shortcut {} starts at different edge than replaced edge {}",
                    edge, edge1
                );
            }
            if self.edges[edge2].target_id != self.edges[edge].target_id {
                panic!(
                    "shortcut {} ends at different edge than replaced edge {}",
                    edge, edge2
                );
            }
            if self.edges[edge1].target_id != self.edges[edge2].source_id {
                panic!("shortcut edges not connected {} & {}", edge1, edge2);
            }
            let mut first = self.unpack_edge(edge1);
            first.extend(self.unpack_edge(edge2).iter());
            return first;
        }
        vec![edge]
    }
}

pub fn parse_graph_file(file_path: &str) -> Result<Graph, Box<dyn std::error::Error>> {
    // use crate::EDGE_COST_DIMENSION;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    println!("Parsing graph...");
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    for _i in 0..4 {
        // comments and blanks
        lines.next();
    }
    let cost_dim: usize = lines.next().expect("No edge cost dim given")?.parse()?;
    // assert_eq!(EDGE_COST_DIMENSION, cost_dim);
    let num_of_nodes = lines
        .next()
        .expect("Number of nodes not present in file")?
        .parse()?;
    let num_of_edges = lines
        .next()
        .expect("Number of edges not present in file")?
        .parse()?;

    let mut parsed_nodes: usize = 0;
    let mut parsed_edges: u32 = 0;
    while let Some(Ok(line)) = lines.next() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens[0] == "#" || tokens[0] == "\n" {
            continue;
        }
        if parsed_nodes < num_of_nodes {
            nodes.push(Node::new(
                tokens[0].parse()?,
                // tokens[2].parse()?,
                // tokens[3].parse()?,
                // tokens[4].parse()?,
                tokens[5].parse()?,
            ));
            parsed_nodes += 1;
        } else if parsed_edges < num_of_edges {
            let replaced_edges = if tokens[tokens.len() - 2] == "-1" {
                None
            } else {
                Some((
                    tokens[tokens.len() - 2].parse()?,
                    tokens[tokens.len() - 1].parse()?,
                ))
            };

            let costs = edge::parse_costs(&tokens[2..tokens.len() - 2]);
            assert_eq!(
                costs.len(),
                cost_dim,
                "Edge with wrong amount of costs parsed"
            );
            edges.push(Edge::new(
                parsed_edges,
                tokens[0].parse()?,
                tokens[1].parse()?,
                costs,
                replaced_edges,
            ));
            parsed_edges += 1;
        } else {
            panic!("Something doesn't add up with the amount of nodes and edges in graph file");
        }
    }
    Ok(Graph::new(nodes, edges))
}

pub fn parse_minimal_graph_file(file_path: impl AsRef<std::path::Path>) -> anyhow::Result<Graph> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let start = Instant::now();

    println!("Parsing graph...");
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    loop {
        if let Some(Ok(line)) = lines.next() {
            if !line.starts_with('#') {
                break;
            }
        }
    }

    let cost_dim: usize = lines.next().expect("No edge cost dim given")?.parse()?;

    let metric_name_line = lines.next().expect("No metric names given")?;
    let metric_names = metric_name_line.split(' ');
    assert_eq!(
        metric_names.count(),
        cost_dim,
        "Wrong number of metric names in graph file"
    );

    let num_of_nodes = lines
        .next()
        .expect("Number of nodes not present in file")?
        .parse()?;
    let num_of_edges = lines
        .next()
        .expect("Number of edges not present in file")?
        .parse()?;

    let mut parsed_nodes: usize = 0;
    let mut parsed_edges = 0;
    while let Some(Ok(line)) = lines.next() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens[0] == "#" || tokens[0] == "\n" {
            continue;
        }
        if parsed_nodes < num_of_nodes {
            if tokens.len() != 2 {
                panic!("Not right amount of information for a node");
            }
            nodes.push(Node::new(
                tokens[0].parse()?,
                tokens[1].parse().unwrap_or(0),
            ));
            parsed_nodes += 1;
        } else if parsed_edges < num_of_edges {
            if tokens.len() != 5 + cost_dim {
                panic!(
                    "Not right amount of information for an edge, {} instead of {}",
                    tokens.len(),
                    5 + cost_dim
                );
            }
            let replaced_edges = if tokens[tokens.len() - 2] == "-1" {
                None
            } else {
                Some((
                    tokens[tokens.len() - 2].parse()?,
                    tokens[tokens.len() - 1].parse()?,
                ))
            };
            edges.push(Edge::new(
                parsed_edges,
                tokens[1].parse()?,
                tokens[2].parse()?,
                edge::parse_costs(&tokens[3..tokens.len() - 2]),
                replaced_edges,
            ));
            parsed_edges += 1;
        } else {
            panic!("Something doesn't add up with the amount of nodes and edges in graph file");
        }

        if nodes.len() != parsed_nodes || edges.len() != parsed_edges as usize {
            panic!("Not enough nodes or edges parsed");
        }
    }
    let graph = Graph::new(nodes, edges);
    let time = start.elapsed();
    println!("graph loading time: {}s", time.as_secs_f64());
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use crate::utils::same_array;

    use super::*;

    #[test]
    fn test_graph_structure() {
        let graph = parse_graph_file("./resources/concTestGraph").unwrap();

        for n in &graph.nodes.0 {
            for e in graph.edges_of(n.id, EdgeDirection::Out) {
                let edge = &graph.edges[e.edge_id];
                assert_eq!(n.id, edge.source_id);
                assert_eq!(edge.target_id, e.target_id);
                assert!(same_array(e.edge_costs, &edge.edge_costs))
            }

            for e in graph.edges_of(n.id, EdgeDirection::In) {
                let edge = &graph.edges[e.edge_id];
                assert_eq!(n.id, edge.target_id);
                assert_eq!(edge.source_id, e.target_id);
                assert!(same_array(e.edge_costs, &edge.edge_costs))
            }
        }
    }
}
