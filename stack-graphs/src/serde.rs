use crate::{arena::Handle, json::Filter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
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
}

impl StackGraph {
    pub fn from_graph<'a>(graph: &crate::graph::StackGraph, filter: &'a dyn Filter) -> Self {
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
        let not_found = |f: &str| Error::FileNotFound(f.to_owned());
        let no_file_data = |id| Error::NoFileData(id);

        for n in self.nodes.data.as_slice() {
            match n {
                Node::DropScopes { id, .. } => {
                    let file = id.file().ok_or(no_file_data(id.local_id))?;
                    let file_handle = graph.get_file(file).ok_or(not_found(file))?;
                    let node_id = crate::graph::NodeID::new_in_file(file_handle, id.local_id);
                    graph.add_drop_scopes_node(node_id);
                }
                Node::PopScopedSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let file = id.file().ok_or(no_file_data(id.local_id))?;
                    let file_handle = graph.get_file(file).ok_or(not_found(file))?;
                    let node_id = crate::graph::NodeID::new_in_file(file_handle, id.local_id);
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_pop_scoped_symbol_node(node_id, symbol_handle, *is_definition);
                }
                Node::PopSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let file = id.file().ok_or(no_file_data(id.local_id))?;
                    let file_handle = graph.get_file(file).ok_or(not_found(file))?;
                    let node_id = crate::graph::NodeID::new_in_file(file_handle, id.local_id);
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_pop_symbol_node(node_id, symbol_handle, *is_definition);
                }
                Node::PushScopedSymbol {
                    id,
                    symbol,
                    scope,
                    is_reference,
                    ..
                } => {
                    let file = id.file().ok_or(no_file_data(id.local_id))?;
                    let file_handle = graph.get_file(file).ok_or(not_found(file))?;
                    let node_id = crate::graph::NodeID::new_in_file(file_handle, id.local_id);

                    let scope_file = id.file().ok_or(no_file_data(scope.local_id))?;
                    let scope_file_handle =
                        graph.get_file(scope_file).ok_or(not_found(scope_file))?;
                    let scope_id =
                        crate::graph::NodeID::new_in_file(scope_file_handle, scope.local_id);

                    let symbol_handle = graph.add_symbol(symbol.as_str());

                    graph.add_push_scoped_symbol_node(
                        node_id,
                        symbol_handle,
                        scope_id,
                        *is_reference,
                    );
                }
                Node::PushSymbol {
                    id,
                    symbol,
                    is_reference,
                    ..
                } => {
                    let file = id.file().ok_or(no_file_data(id.local_id))?;
                    let file_handle = graph.get_file(file).ok_or(not_found(file))?;
                    let node_id = crate::graph::NodeID::new_in_file(file_handle, id.local_id);
                    let symbol_handle = graph.add_symbol(symbol.as_str());
                    graph.add_push_symbol_node(node_id, symbol_handle, *is_reference);
                }
                Node::Scope {
                    id, is_exported, ..
                } => {
                    let file = id.file().ok_or(no_file_data(id.local_id))?;
                    let file_handle = graph.get_file(file).ok_or(not_found(file))?;
                    let node_id = crate::graph::NodeID::new_in_file(file_handle, id.local_id);
                    graph.add_scope_node(node_id, *is_exported);
                }
                Node::JumpToScope { .. } | Node::Root { .. } => {}
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
            let source_file_handle = source.file().and_then(|f| graph.get_file(f));
            let sink_file_handle = sink.file().and_then(|f| graph.get_file(f));

            let convert = |n: &NodeID| {
                if n.is_root() {
                    crate::graph::NodeID::root()
                } else if n.is_jump_to() {
                    crate::graph::NodeID::jump_to()
                } else {
                    panic!()
                }
            };
            let (source_id, sink_id) = match (source_file_handle, sink_file_handle) {
                (Some(a), Some(b)) => {
                    let source_node_id = crate::graph::NodeID::new_in_file(a, source.local_id);
                    let sink_node_id = crate::graph::NodeID::new_in_file(b, source.local_id);
                    (source_node_id, sink_node_id)
                }
                (Some(a), None) => {
                    let source_node_id = crate::graph::NodeID::new_in_file(a, source.local_id);
                    let sink_node_id = convert(&sink);
                    (source_node_id, sink_node_id)
                }
                (None, Some(b)) => {
                    let source_node_id = convert(&source);
                    let sink_node_id = crate::graph::NodeID::new_in_file(b, source.local_id);
                    (source_node_id, sink_node_id)
                }
                (None, None) => {
                    let source_node_id = convert(&source);
                    let sink_node_id = convert(&sink);
                    (source_node_id, sink_node_id)
                }
            };

            if let (Some(source_node), Some(sink_node)) =
                (graph.node_for_id(source_id), graph.node_for_id(sink_id))
            {
                graph.add_edge(source_node, sink_node, *precedence);
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
#[serde(transparent)]
pub struct Files {
    data: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
#[serde(transparent)]
pub struct Nodes {
    data: Vec<Node>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct SourceInfo {
    span: lsp_positions::Span,
    syntax_type: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(transparent)]
pub struct DebugInfo {
    data: Vec<DebugEntry>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DebugEntry {
    key: String,
    value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct NodeID {
    file: Option<String>,
    local_id: u32,
}

impl NodeID {
    fn is_root(&self) -> bool {
        self.local_id == crate::graph::NodeID::root().local_id()
    }

    fn is_jump_to(&self) -> bool {
        self.local_id == crate::graph::NodeID::jump_to().local_id()
    }

    fn file(&self) -> Option<&str> {
        self.file.as_ref().map(|f| f.as_str())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
#[serde(transparent)]
pub struct Edges {
    data: Vec<Edge>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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
    fn reconstruct() {
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
        }]});
        let observed = serde_json::from_value::<super::StackGraph>(json_data).unwrap();
        let mut sg = crate::graph::StackGraph::new();
        observed.load_into(&mut sg).unwrap();

        // always 2 nodes: root and jump
        assert_eq!(sg.iter_nodes().count(), 2);
        assert_eq!(sg.iter_files().count(), 1);
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
