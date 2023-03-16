use crate::arena::Handle;
pub use filter::{Filter, NoFilter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod filter {
    use crate::{
        arena::Handle,
        graph::{File, Node, StackGraph},
        partial::{PartialPath, PartialPaths},
    };

    pub trait Filter {
        /// Return whether elements for the given file must be included.
        fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool;

        /// Return whether the given node must be included.
        /// Nodes of excluded files are always excluded.
        fn include_node(&self, graph: &StackGraph, node: &Handle<Node>) -> bool;

        /// Return whether the given edge must be included.
        /// Edges via excluded nodes are always excluded.
        fn include_edge(
            &self,
            graph: &StackGraph,
            source: &Handle<Node>,
            sink: &Handle<Node>,
        ) -> bool;

        /// Return whether the given path must be included.
        /// Paths via excluded nodes or edges are always excluded.
        fn include_partial_path(
            &self,
            graph: &StackGraph,
            paths: &PartialPaths,
            path: &PartialPath,
        ) -> bool;
    }

    impl<F> Filter for F
    where
        F: Fn(&StackGraph, &Handle<File>) -> bool,
    {
        fn include_file(&self, graph: &StackGraph, file: &Handle<File>) -> bool {
            self(graph, file)
        }

        fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
            true
        }

        fn include_edge(
            &self,
            _graph: &StackGraph,
            _source: &Handle<Node>,
            _sink: &Handle<Node>,
        ) -> bool {
            true
        }

        fn include_partial_path(
            &self,
            _graph: &StackGraph,
            _paths: &PartialPaths,
            _path: &PartialPath,
        ) -> bool {
            true
        }
    }

    // Filter implementation that includes everything.
    pub struct NoFilter;

    impl Filter for NoFilter {
        fn include_file(&self, _graph: &StackGraph, _file: &Handle<File>) -> bool {
            true
        }

        fn include_node(&self, _graph: &StackGraph, _node: &Handle<Node>) -> bool {
            true
        }

        fn include_edge(
            &self,
            _graph: &StackGraph,
            _source: &Handle<Node>,
            _sink: &Handle<Node>,
        ) -> bool {
            true
        }

        fn include_partial_path(
            &self,
            _graph: &StackGraph,
            _paths: &PartialPaths,
            _path: &PartialPath,
        ) -> bool {
            true
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Clone)]
pub struct StackGraph {
    files: Files,
    nodes: Nodes,
    edges: Edges,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("failed to load file `{0}`")]
    FileNotFound(String),

    #[error("duplicate file `{0}`")]
    FileAlreadyPresent(String),

    #[error("failed to locate node `{0}` in graph")]
    NodeNotFound(u32),

    #[error("no file data for node `{0}`")]
    NoFileData(u32),

    #[error("node `{0}` is an invalid node")]
    InvalidNode(u32),
}

impl StackGraph {
    pub fn from_graph<'a>(graph: &crate::graph::StackGraph) -> Self {
        Self::from_graph_filter(graph, &NoFilter)
    }

    pub fn from_graph_filter<'a>(graph: &crate::graph::StackGraph, filter: &'a dyn Filter) -> Self {
        let files = graph.filter_files(filter);
        let nodes = graph.filter_nodes(filter);
        let edges = graph.filter_edges(filter);
        Self {
            files,
            nodes,
            edges,
        }
    }

    pub fn load_into(&self, graph: &mut crate::graph::StackGraph) -> Result<(), Error> {
        self.load_files(graph)?;
        self.load_nodes(graph)?;
        self.load_edges(graph)?;
        Ok(())
    }

    fn load_files(&self, graph: &mut crate::graph::StackGraph) -> Result<(), Error> {
        // we check if any of the files we are about to introduce, are
        // already present in this graph, to avoid accidental merges
        if let Some(f) = self
            .files
            .data
            .iter()
            .find(|f| graph.get_file(f.as_str()).is_some())
        {
            return Err(Error::FileAlreadyPresent(f.to_owned()));
        }

        for f in self.files.data.iter() {
            graph.add_file(f.as_str()).unwrap();
        }

        Ok(())
    }

    fn load_nodes(&self, graph: &mut crate::graph::StackGraph) -> Result<(), Error> {
        for n in self.nodes.data.as_slice() {
            let handle = match n {
                Node::DropScopes { id, .. } => {
                    let node_id = id.into_node_id(graph)?;
                    graph.add_drop_scopes_node(node_id)
                }
                Node::PopScopedSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let node_id = id.into_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_pop_scoped_symbol_node(node_id, symbol_handle, *is_definition)
                }
                Node::PopSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let node_id = id.into_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_pop_symbol_node(node_id, symbol_handle, *is_definition)
                }
                Node::PushScopedSymbol {
                    id,
                    symbol,
                    scope,
                    is_reference,
                    ..
                } => {
                    let node_id = id.into_node_id(graph)?;
                    let scope_id = scope.into_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_push_scoped_symbol_node(
                        node_id,
                        symbol_handle,
                        scope_id,
                        *is_reference,
                    )
                }
                Node::PushSymbol {
                    id,
                    symbol,
                    is_reference,
                    ..
                } => {
                    let node_id = id.into_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_push_symbol_node(node_id, symbol_handle, *is_reference)
                }
                Node::Scope {
                    id, is_exported, ..
                } => {
                    let node_id = id.into_node_id(graph)?;
                    graph.add_scope_node(node_id, *is_exported)
                }
                Node::JumpToScope { .. } | Node::Root { .. } => None,
            };

            if let Some(handle) = handle {
                // load source-info of each node
                if let Some(source_info) = n.source_info() {
                    *graph.source_info_mut(handle) = crate::graph::SourceInfo {
                        span: source_info.span.clone(),
                        syntax_type: source_info
                            .syntax_type
                            .as_ref()
                            .map(|st| graph.add_string(st.as_str())),
                        ..Default::default()
                    };
                }

                // load debug-info of each node
                if let Some(debug_info) = n.debug_info() {
                    *graph.debug_info_mut(handle) = debug_info.data.iter().fold(
                        crate::graph::DebugInfo::default(),
                        |mut info, entry| {
                            let key = graph.add_string(entry.key.as_str());
                            let value = graph.add_string(entry.value.as_str());
                            info.add(key, value);
                            info
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn load_edges(&self, graph: &mut crate::graph::StackGraph) -> Result<(), Error> {
        // load edges into stack-graph
        for Edge {
            source,
            sink,
            precedence,
        } in self.edges.data.as_slice()
        {
            let source_id = source.into_node_id(graph)?;
            let sink_id = sink.into_node_id(graph)?;

            let source_handle = graph
                .node_for_id(source_id)
                .ok_or(Error::InvalidNode(source.local_id))?;
            let sink_handle = graph
                .node_for_id(sink_id)
                .ok_or(Error::InvalidNode(sink.local_id))?;

            graph.add_edge(source_handle, sink_handle, *precedence);
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Clone)]
#[serde(transparent)]
pub struct Files {
    data: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Clone)]
#[serde(transparent)]
pub struct Nodes {
    data: Vec<Node>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    DropScopes {
        id: NodeID,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    JumpToScope {
        id: NodeID,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    PopScopedSymbol {
        id: NodeID,
        symbol: String,
        is_definition: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    PopSymbol {
        id: NodeID,
        symbol: String,
        is_definition: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    PushScopedSymbol {
        id: NodeID,
        symbol: String,
        scope: NodeID,
        is_reference: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    PushSymbol {
        id: NodeID,
        symbol: String,
        is_reference: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    Root {
        id: NodeID,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    Scope {
        id: NodeID,
        is_exported: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },
}

impl Node {
    fn source_info(&self) -> Option<&SourceInfo> {
        match self {
            Self::DropScopes { source_info, .. } => source_info,
            Self::JumpToScope { source_info, .. } => source_info,
            Self::PopScopedSymbol { source_info, .. } => source_info,
            Self::PopSymbol { source_info, .. } => source_info,
            Self::PushScopedSymbol { source_info, .. } => source_info,
            Self::PushSymbol { source_info, .. } => source_info,
            Self::Root { source_info, .. } => source_info,
            Self::Scope { source_info, .. } => source_info,
        }
        .as_ref()
    }

    fn debug_info(&self) -> Option<&DebugInfo> {
        match self {
            Self::DropScopes { debug_info, .. } => debug_info,
            Self::JumpToScope { debug_info, .. } => debug_info,
            Self::PopScopedSymbol { debug_info, .. } => debug_info,
            Self::PopSymbol { debug_info, .. } => debug_info,
            Self::PushScopedSymbol { debug_info, .. } => debug_info,
            Self::PushSymbol { debug_info, .. } => debug_info,
            Self::Root { debug_info, .. } => debug_info,
            Self::Scope { debug_info, .. } => debug_info,
        }
        .as_ref()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SourceInfo {
    span: lsp_positions::Span,
    syntax_type: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(transparent)]
pub struct DebugInfo {
    data: Vec<DebugEntry>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct DebugEntry {
    key: String,
    value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct NodeID {
    file: Option<String>,
    local_id: u32,
}

impl NodeID {
    fn into_node_id(
        &self,
        graph: &crate::graph::StackGraph,
    ) -> Result<crate::graph::NodeID, Error> {
        if let Some(file) = self.file.as_ref() {
            let handle = graph
                .get_file(file.as_str())
                .ok_or(Error::FileNotFound(file.to_owned()))?;
            Ok(crate::graph::NodeID::new_in_file(handle, self.local_id))
        } else if self.is_root() {
            Ok(crate::graph::NodeID::root())
        } else if self.is_jump_to() {
            Ok(crate::graph::NodeID::jump_to())
        } else {
            Err(Error::InvalidNode(self.local_id))
        }
    }
}

impl NodeID {
    fn is_root(&self) -> bool {
        self.local_id == crate::graph::NodeID::root().local_id()
    }

    fn is_jump_to(&self) -> bool {
        self.local_id == crate::graph::NodeID::jump_to().local_id()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Clone)]
#[serde(transparent)]
pub struct Edges {
    data: Vec<Edge>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Edge {
    source: NodeID,
    sink: NodeID,
    precedence: i32,
}

impl crate::graph::StackGraph {
    fn filter_files<'a>(&self, filter: &'a dyn Filter) -> Files {
        Files {
            data: self
                .iter_files()
                .filter(|f| filter.include_file(self, f))
                .map(|f| self[f].name().to_owned())
                .collect::<Vec<_>>(),
        }
    }

    fn filter_node<'a>(&self, _filter: &'a dyn Filter, id: crate::graph::NodeID) -> NodeID {
        let file = id.file().map(|idx| self[idx].name().to_owned());
        let local_id = id.local_id();
        NodeID { file, local_id }
    }

    fn filter_source_info<'a>(
        &self,
        _filter: &'a dyn Filter,
        handle: Handle<crate::graph::Node>,
    ) -> Option<SourceInfo> {
        self.source_info(handle).map(|info| SourceInfo {
            span: info.span.clone(),
            syntax_type: info.syntax_type.map(|ty| self[ty].to_owned()),
        })
    }

    fn filter_debug_info<'a>(
        &self,
        _filter: &'a dyn Filter,
        handle: Handle<crate::graph::Node>,
    ) -> Option<DebugInfo> {
        self.debug_info(handle).map(|info| DebugInfo {
            data: info
                .iter()
                .map(|entry| DebugEntry {
                    key: self[entry.key].to_owned(),
                    value: self[entry.value].to_owned(),
                })
                .collect(),
        })
    }

    fn filter_nodes<'a>(&self, filter: &'a dyn Filter) -> Nodes {
        Nodes {
            data: self
                .iter_nodes()
                .filter(|n| filter.include_node(self, &n))
                .map(|handle| {
                    let node = &self[handle];
                    let id = self.filter_node(filter, node.id());
                    let source_info = self.filter_source_info(filter, handle);
                    let debug_info = self.filter_debug_info(filter, handle);

                    match node {
                        crate::graph::Node::DropScopes(_node) => Node::DropScopes {
                            id,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::JumpTo(_node) => Node::JumpToScope {
                            id,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::PopScopedSymbol(node) => Node::PopScopedSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            is_definition: node.is_definition,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::PopSymbol(node) => Node::PopSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            is_definition: node.is_definition,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::PushScopedSymbol(node) => Node::PushScopedSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            scope: self.filter_node(filter, node.scope),
                            is_reference: node.is_reference,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::PushSymbol(node) => Node::PushSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            is_reference: node.is_reference,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::Root(_node) => Node::Root {
                            id,
                            source_info,
                            debug_info,
                        },
                        crate::graph::Node::Scope(node) => Node::Scope {
                            id,
                            is_exported: node.is_exported,
                            source_info,
                            debug_info,
                        },
                    }
                })
                .collect::<Vec<_>>(),
        }
    }

    fn filter_edges<'a>(&self, filter: &'a dyn Filter) -> Edges {
        Edges {
            data: self
                .iter_nodes()
                .map(|source| {
                    self.outgoing_edges(source)
                        .filter(|e| filter.include_edge(self, &e.source, &e.sink))
                        .map(|e| Edge {
                            source: self.filter_node(filter, self[e.source].id()),
                            sink: self.filter_node(filter, self[e.sink].id()),
                            precedence: e.precedence,
                        })
                })
                .flatten()
                .collect::<Vec<_>>(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(feature = "json")]
    fn serde_json_stack_graph() {
        let expected = StackGraph {
            files: Files {
                data: vec!["index.ts".to_owned()],
            },
            nodes: Nodes {
                data: vec![Node::Root {
                    id: NodeID {
                        local_id: 1,
                        file: None,
                    },
                    source_info: Some(SourceInfo {
                        span: lsp_positions::Span {
                            start: lsp_positions::Position {
                                line: 0,
                                column: lsp_positions::Offset {
                                    utf8_offset: 0,
                                    utf16_offset: 0,
                                    grapheme_offset: 0,
                                },
                                containing_line: 0..0,
                                trimmed_line: 0..0,
                            },
                            end: lsp_positions::Position {
                                line: 0,
                                column: lsp_positions::Offset {
                                    utf8_offset: 0,
                                    utf16_offset: 0,
                                    grapheme_offset: 0,
                                },
                                containing_line: 0..0,
                                trimmed_line: 0..0,
                            },
                        },
                        syntax_type: None,
                    }),
                    debug_info: Some(DebugInfo { data: vec![] }),
                }],
            },
            edges: Edges {
                data: vec![Edge {
                    source: NodeID {
                        file: None,
                        local_id: 1,
                    },
                    sink: NodeID {
                        file: Some("index.ts".to_owned()),
                        local_id: 0,
                    },
                    precedence: 0,
                }],
            },
        };

        let json_data = serde_json::json!({
            "files": [
                "index.ts"
            ],
            "nodes": [{
                "type": "root",
                "id": {
                    "local_id": 1
                },
                "source_info": {
                    "span": {
                        "start": {
                            "line": 0,
                            "column": {
                                "utf8_offset": 0,
                                "utf16_offset": 0,
                                "grapheme_offset": 0
                            }
                        },
                        "end": {
                            "line": 0,
                            "column": {
                                "utf8_offset": 0,
                                "utf16_offset": 0,
                                "grapheme_offset": 0
                            }
                        }
                    }
                },
                "debug_info": []
            }],
            "edges": [{
                "source": {
                    "local_id": 1
                },
                "sink": {
                    "file": "index.ts",
                    "local_id": 0
                },
                "precedence": 0
            }]

        });

        let observed = serde_json::from_value::<super::StackGraph>(json_data).unwrap();

        assert_eq!(observed, expected);
    }

    #[test]
    #[cfg(feature = "json")]
    fn reconstruct() {
        let json_data = serde_json::json!(
            {
              "files": [
                "index.ts"
              ],
              "nodes": [
                {
                  "type": "root",
                  "id": {
                    "local_id": 1
                  },
                  "source_info": {
                    "span": {
                      "start": {
                        "line": 0,
                        "column": {
                          "utf8_offset": 0,
                          "utf16_offset": 0,
                          "grapheme_offset": 0
                        }
                      },
                      "end": {
                        "line": 0,
                        "column": {
                          "utf8_offset": 0,
                          "utf16_offset": 0,
                          "grapheme_offset": 0
                        }
                      }
                    }
                  },
                  "debug_info": []
                },
                {
                  "type": "jump_to_scope",
                  "id": {
                    "local_id": 2
                  },
                  "source_info": {
                    "span": {
                      "start": {
                        "line": 0,
                        "column": {
                          "utf8_offset": 0,
                          "utf16_offset": 0,
                          "grapheme_offset": 0
                        }
                      },
                      "end": {
                        "line": 0,
                        "column": {
                          "utf8_offset": 0,
                          "utf16_offset": 0,
                          "grapheme_offset": 0
                        }
                      }
                    }
                  },
                  "debug_info": []
                },
                {
                  "type": "scope",
                  "is_exported": false,
                  "id": {
                    "file": "index.ts",
                    "local_id": 0
                  },
                  "source_info": {
                    "span": {
                      "start": {
                        "line": 0,
                        "column": {
                          "utf8_offset": 0,
                          "utf16_offset": 0,
                          "grapheme_offset": 0
                        }
                      },
                      "end": {
                        "line": 0,
                        "column": {
                          "utf8_offset": 0,
                          "utf16_offset": 0,
                          "grapheme_offset": 0
                        }
                      }
                    }
                  },
                  "debug_info": [
                    {
                      "key": "tsg_variable",
                      "value": "@prog.defs"
                    },
                    {
                      "key": "tsg_location",
                      "value": "(225, 14)"
                    }
                  ]
                }
              ],
              "edges": [
                {
                  "source": {
                    "local_id": 1
                  },
                  "sink": {
                    "file": "index.ts",
                    "local_id": 0
                  },
                  "precedence": 0
                }
              ]
            }
        );
        let observed = serde_json::from_value::<super::StackGraph>(json_data).unwrap();
        let mut sg = crate::graph::StackGraph::new();
        observed.load_into(&mut sg).unwrap();

        assert_eq!(sg.iter_nodes().count(), 3);
        assert_eq!(sg.iter_files().count(), 1);

        // the scope node should contain debug and source info
        let handle = sg
            .iter_nodes()
            .find(|handle| matches!(sg[*handle], crate::graph::Node::Scope(..)))
            .unwrap();
        assert!(sg.source_info(handle).is_some());
        assert!(sg.debug_info(handle).is_some());
    }

    #[test]
    fn load_fail_accidental_merge() {
        let source = StackGraph {
            files: Files {
                data: vec!["index.ts".to_owned(), "App.tsx".to_owned()],
            },
            ..Default::default()
        };

        let mut target = crate::graph::StackGraph::new();
        target.add_file("App.tsx").unwrap();

        assert_eq!(
            source.load_into(&mut target).unwrap_err(),
            Error::FileAlreadyPresent("App.tsx".to_owned())
        );

        // ensure that source and target graphs were not partially merged
        assert_eq!(target.iter_files().count(), 1);
    }
}
