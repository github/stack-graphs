// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::graph::StackGraph;
use crate::json::JsonError;
use crate::paths::Paths;

static CSS: &'static str = include_str!("visualization.css");
static JS: &'static str = include_str!("visualization.js");

//-----------------------------------------------------------------------------
// StackGraph

impl StackGraph {
    pub fn to_html_string(&self, paths: &mut Paths, title: &str) -> Result<String, JsonError> {
        let graph = self.to_json_string()?;
        let paths = paths.to_json_string(self)?;
        let html = format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{}</title>
<script src="https://d3js.org/d3.v7.min.js" charset="utf-8"></script>
<style>
{}
</style>
<script type="text/javascript">
{}
</script>
<script type="text/javascript">
  let graph = {};
  let paths = {};
  d3.select(window).on("load", function() {{
    let container = d3.select("\#container");
    sg_visualize(container, graph, paths);
  }});
</script>
</head>
<body>
  <div id="container">
  </div>
</body>
</html>
        "#,
            title, CSS, JS, graph, paths
        );
        Ok(html)
    }
}
