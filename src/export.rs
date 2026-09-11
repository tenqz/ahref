use crate::{Graph, Result};
use clap::ValueEnum;
use std::io::Write;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Json,
    Graphml,
    Dot,
}

fn xml(value: &str) -> String {
    value
        .chars()
        .filter(|&c| {
            matches!(c, '\t' | '\n' | '\r')
                || ('\u{20}'..='\u{d7ff}').contains(&c)
                || ('\u{e000}'..='\u{fffd}').contains(&c)
                || ('\u{10000}'..='\u{10ffff}').contains(&c)
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn dot(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

pub fn write(graph: &Graph, format: Format, mut out: impl Write) -> Result<()> {
    graph.validate()?;
    if matches!(format, Format::Json) {
        serde_json::to_writer_pretty(&mut out, graph)?;
        writeln!(out)?;
        return Ok(());
    }
    let ids: std::collections::BTreeMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.url.as_str(), i))
        .collect();
    match format {
        Format::Graphml => {
            writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">")?;
            for (id, scope, name, kind) in [
                ("url", "node", "url", "string"),
                ("kind", "all", "kind", "string"),
                ("status", "node", "status_code", "int"),
                ("depth", "node", "depth", "int"),
                ("rank", "node", "pagerank", "double"),
                ("state", "node", "state", "string"),
                ("in", "node", "in_degree", "int"),
                ("out", "node", "out_degree", "int"),
                ("redirect", "node", "redirect_target", "string"),
                ("anchor", "edge", "anchor", "string"),
                ("rel", "edge", "rel", "string"),
            ] {
                writeln!(out, "  <key id=\"{id}\" for=\"{scope}\" attr.name=\"{name}\" attr.type=\"{kind}\"/>")?;
            }
            writeln!(out, "  <graph id=\"site\" edgedefault=\"directed\">")?;
            for (i, n) in graph.nodes.iter().enumerate() {
                writeln!(out, "    <node id=\"n{i}\">")?;
                let state = serde_json::to_value(n.state)?;
                let kind = serde_json::to_value(n.kind)?;
                for (key, value) in [
                    ("url", n.url.to_string()),
                    ("kind", kind.as_str().unwrap_or_default().into()),
                    ("state", state.as_str().unwrap_or_default().into()),
                    ("in", n.in_degree.to_string()),
                    ("out", n.out_degree.to_string()),
                ] {
                    writeln!(out, "      <data key=\"{key}\">{}</data>", xml(&value))?;
                }
                for (key, value) in [
                    ("status", n.status_code.map(|v| v.to_string())),
                    ("depth", n.depth.map(|v| v.to_string())),
                    ("rank", n.pagerank.map(|v| v.to_string())),
                    (
                        "redirect",
                        n.redirect_target.as_ref().map(ToString::to_string),
                    ),
                ] {
                    if let Some(value) = value {
                        writeln!(out, "      <data key=\"{key}\">{}</data>", xml(&value))?;
                    }
                }
                writeln!(out, "    </node>")?;
            }
            for (i, e) in graph.edges.iter().enumerate() {
                let kind = serde_json::to_value(e.kind)?;
                writeln!(out, "    <edge id=\"e{i}\" source=\"n{}\" target=\"n{}\"><data key=\"anchor\">{}</data><data key=\"rel\">{}</data><data key=\"kind\">{}</data></edge>", ids[e.from.as_str()], ids[e.to.as_str()], xml(&e.anchor), xml(&e.rel.join(" ")), kind.as_str().unwrap_or_default())?;
            }
            writeln!(out, "  </graph>\n</graphml>")?;
        }
        Format::Dot => {
            writeln!(out, "digraph site {{")?;
            for (i, n) in graph.nodes.iter().enumerate() {
                writeln!(
                    out,
                    "  n{i} [label=\"{}\", pagerank=\"{}\"];",
                    dot(n.url.as_str()),
                    n.pagerank.map(|v| v.to_string()).unwrap_or_default()
                )?;
            }
            for e in &graph.edges {
                writeln!(
                    out,
                    "  n{} -> n{} [label=\"{}\", rel=\"{}\"];",
                    ids[e.from.as_str()],
                    ids[e.to.as_str()],
                    dot(&e.anchor),
                    dot(&e.rel.join(" "))
                )?;
            }
            writeln!(out, "}}")?;
        }
        Format::Json => {}
    }
    Ok(())
}
