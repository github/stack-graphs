// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::graph::Node;
use crate::graph::StackGraph;
use crate::json::JsonError;
use crate::paths::Path;
use crate::paths::Paths;

static CSS: &'static str = include_str!("visualization.css");
static D3: &'static str = include_str!("d3.v7.min.js");
static D3_DAG: &'static str = include_str!("d3-dag.v0.10.0.min.js");
static JS: &'static str = include_str!("visualization.js");

//-----------------------------------------------------------------------------
// StackGraph

impl StackGraph {
    pub fn to_html_string(&self, paths: &mut Paths, title: &str) -> Result<String, JsonError> {
        let graph = self.to_json().to_string()?;
        let paths = paths
            .to_json(self, include_path_in_visualization)
            .to_string()?;
        let html = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{}</title>
<style>
{}
</style>
<style>
  html, body, #container {{
    width: 100%;
    height: 100%;
    margin: 0;
  }}
</style>
<script type="text/javascript">
{}
</script>
<script type="text/javascript">
{}
</script>
<script charset="utf-8">
{}
</script>
<script type="text/javascript">
  let graph = {};
  let paths = {};
</script>
</head>
<body>
  <div id="container">
  </div>
  <script type="text/javascript">
    let container = d3.select("\#container");
    new StackGraph(container, graph, paths);
  </script>
</body>
</html>
        "#,
            title, CSS, D3, D3_DAG, JS, graph, paths
        );
        Ok(html)
    }
}

fn include_path_in_visualization(graph: &StackGraph, _paths: &Paths, path: &Path) -> bool {
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
