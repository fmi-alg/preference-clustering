use anyhow::Result;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use crate::graph::path::Path;

use super::MyVec;

pub fn read_in_paths(file_name: impl AsRef<std::path::Path>) -> Result<Vec<Path>> {
    let f = File::open(file_name)?;
    let mut f = BufReader::new(f);
    let mut buf: String = String::new();
    f.read_line(&mut buf)?;
    let num_paths = buf.trim().parse::<usize>().unwrap();
    let mut paths: Vec<Path> = Vec::with_capacity(num_paths);
    for _ in 0..num_paths {
        buf = String::from("");
        f.read_line(&mut buf)?;
        let num_edges = buf.trim().parse::<usize>().unwrap();
        let mut edges: MyVec<u32> = MyVec::new();
        for _ in 0..num_edges {
            buf = String::from("");
            f.read_line(&mut buf)?;
            edges.push(buf.trim().parse::<u32>().unwrap());
        }
        buf = String::from("");
        f.read_line(&mut buf)?;
        let num_nodes = buf.trim().parse::<usize>().unwrap();
        let mut nodes: MyVec<u32> = MyVec::new();
        for _ in 0..num_nodes {
            buf = String::from("");
            f.read_line(&mut buf)?;
            nodes.push(buf.trim().parse::<u32>().unwrap());
        }
        buf = String::from("");
        f.read_line(&mut buf)?;
        let dim = buf.trim().parse::<usize>().unwrap();
        let mut costs: MyVec<f64> = MyVec::new();
        for _ in 0..dim {
            buf = String::from("");
            f.read_line(&mut buf)?;
            costs.push(buf.trim().parse::<f64>().unwrap());
        }
        paths.push(Path {
            edges,
            nodes,
            total_dimension_costs: costs,
        })
    }

    Ok(paths)
}

pub fn read_in_pref_indices(file_name: PathBuf) -> Result<Vec<usize>> {
    let f = File::open(file_name)?;
    let mut f = BufReader::new(f);
    let mut buf: String = String::new();
    f.read_line(&mut buf)?;
    let num_prefs = buf.trim().parse::<usize>().unwrap();
    for _ in 0..num_prefs {
        buf = String::from("");
        f.read_line(&mut buf)?;
        let num_values = buf.trim().parse::<usize>().unwrap();
        for _ in 0..num_values {
            buf = String::from("");
            f.read_line(&mut buf)?;
        }
    }
    buf = String::from("");
    f.read_line(&mut buf)?;
    let num_pref_indices = buf.trim().parse::<usize>().unwrap();
    let mut pref_indices: Vec<usize> = Vec::new();
    for _ in 0..num_pref_indices {
        buf = String::from("");
        f.read_line(&mut buf)?;
        pref_indices.push(buf.trim().parse::<usize>().unwrap());
    }

    Ok(pref_indices)
}

pub fn write_out_pref_info(
    file_name: &str,
    prefs: Vec<MyVec<f64>>,
    pref_indices: Vec<usize>,
) -> std::io::Result<()> {
    let mut data = String::from("");
    data = format!("{}{}", data, prefs.len());
    for p in prefs {
        data = format!("{}\n{}", data, p.len());
        for x in p.iter() {
            data = format!("{}\n{}", data, x);
        }
    }
    data = format!("{}\n{}", data, pref_indices.len());
    for p in pref_indices {
        data = format!("{}\n{}", data, p);
    }
    fs::write(file_name, data).expect("Unable to write file");
    Ok(())
}

pub fn write_out_paths(file_name: &str, paths: Vec<Path>) -> std::io::Result<()> {
    let mut data = String::from("");
    data = format!("{}{}", data, paths.len());
    for p in paths {
        data = format!("{}\n{}", data, p.edges.len());
        for e in p.edges.iter() {
            data = format!("{}\n{}", data, e);
        }
        data = format!("{}\n{}", data, p.nodes.len());
        for n in p.nodes.iter() {
            data = format!("{}\n{}", data, n);
        }
        data = format!("{}\n{}", data, p.total_dimension_costs.len());
        for c in p.total_dimension_costs.iter() {
            data = format!("{}\n{}", data, c);
        }
    }
    fs::write(file_name, data).expect("Unable to write file");
    Ok(())
}
