// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;
use crate::json::Filter;
use crate::json::JsonError;
use crate::partial::PartialPath;
use crate::partial::PartialPaths;
use crate::paths::Path;
use crate::paths::Paths;
use crate::stitching::Database;

static CSS: &'static str = include_str!("visualization/visualization.css");
static D3: &'static str = include_str!("visualization/d3.v7.min.js");
static D3_DAG: &'static str = include_str!("visualization/d3-dag.v0.10.0.min.js");
static JS: &'static str = include_str!("visualization/visualization.js");

static PKG: &'static str = env!("CARGO_PKG_NAME");
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

//-----------------------------------------------------------------------------
// StackGraph

impl StackGraph {
    pub fn to_html_string(
        &self,
        title: &str,
        partials: &mut PartialPaths,
        db: &mut Database,
        filter: &dyn Filter,
    ) -> Result<String, JsonError> {
        let filter = VisualizationFilter(filter);
        let graph = self.to_json(&filter).to_string()?;
        let paths = db.to_json(self, partials, &filter).to_string()?;
        let html = format!(
            r#"
<!DOCTYPE html>
<html lang="en">

<head>

<meta charset="utf-8">
<title>{title}</title>

<!-- <link href="visualization.css" type="text/css" rel="stylesheet"></link> -->
<style>
{CSS}
</style>

<!-- <script type="text/javascript" src="d3.v7.min.js"></script> -->
<script type="text/javascript">
{D3}
</script>

<!-- <script type="text/javascript" src="d3-dag.v0.10.0.min.js"></script> -->
<script type="text/javascript">
{D3_DAG}
</script>

<!-- <script type="text/javascript" src="visualization.js"></script> -->
<script charset="utf-8">
{JS}
</script>

<script type="text/javascript">
  let graph = {graph};
  let paths = {paths};
</script>

<style>
  html, body, #container {{
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
  }}
</style>

</head>

<body>
  <div id="container">
  </div>
  <script type="text/javascript">
    const container = d3.select("\#container");
    new StackGraph(container, graph, paths, {{ version: "{PKG} {VERSION}" }});
  </script>
</body>

</html>
"#
        );
        Ok(html)
    }
}

struct VisualizationFilter<'a>(&'a dyn Filter);

impl Filter for VisualizationFilter<'_> {
    fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
        self.0.include_file(graph, file)
    }

    fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool {
        self.0.include_node(graph, node)
    }

    fn include_edge(&self, graph: &StackGraph, source: &Handle<Node>, sink: &Handle<Node>) -> bool {
        self.0.include_edge(graph, source, sink)
    }

    fn include_path(&self, graph: &StackGraph, paths: &Paths, path: &Path) -> bool {
        if !self.0.include_path(graph, paths, path) {
            return false;
        }
        if path.start_node == path.end_node {
            return false;
        }
        if !match &graph[path.start_node] {
            Node::PushScopedSymbol(_) | Node::PushSymbol(_) => true,
            Node::Root(_) => true,
            Node::Scope(node) => node.is_exported,
            _ => false,
        } {
            return false;
        }
        if !match &graph[path.end_node] {
            Node::PopScopedSymbol(_) | Node::PopSymbol(_) => {
                path.symbol_stack.is_empty() && path.scope_stack.is_empty()
            }
            Node::Root(_) => true,
            Node::Scope(node) => node.is_exported,
            _ => false,
        } {
            return false;
        }
        return true;
    }

    fn include_partial_path(
        &self,
        graph: &StackGraph,
        paths: &PartialPaths,
        path: &PartialPath,
    ) -> bool {
        if !self.0.include_partial_path(graph, paths, path) {
            return false;
        }
        if path.start_node == path.end_node && path.edges.len() == 0 {
            return false;
        }
        if !match &graph[path.start_node] {
            Node::PushScopedSymbol(_) | Node::PushSymbol(_) => {
                path.symbol_stack_precondition.can_match_empty()
                    && path.scope_stack_precondition.can_match_empty()
            }
            Node::Root(_) => true,
            Node::Scope(node) => node.is_exported,
            _ => false,
        } {
            return false;
        }
        if !match &graph[path.end_node] {
            Node::PopScopedSymbol(_) | Node::PopSymbol(_) => {
                path.symbol_stack_postcondition.can_match_empty()
                    && path.scope_stack_postcondition.can_match_empty()
            }
            Node::Root(_) => true,
            Node::Scope(node) => node.is_exported,
            _ => false,
        } {
            return false;
        }
        return true;
    }
}
