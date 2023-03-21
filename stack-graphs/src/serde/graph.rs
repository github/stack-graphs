// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::arena::Handle;

use super::Filter;
use super::NoFilter;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct StackGraph {
    pub files: Files,
    pub nodes: Nodes,
    pub edges: Edges,
}

#[derive(Debug, Error, Eq, PartialEq)]
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
    InvalidGlobalNodeID(u32),
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
        for file in self.files.data.iter() {
            graph
                .add_file(&file)
                .map_err(|_| Error::FileAlreadyPresent(file.to_owned()))?;
        }

        Ok(())
    }

    fn load_nodes(&self, graph: &mut crate::graph::StackGraph) -> Result<(), Error> {
        for node in &self.nodes.data {
            let handle = match node {
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
                    let symbol_handle = graph.add_symbol(&symbol);
                    graph.add_pop_scoped_symbol_node(node_id, symbol_handle, *is_definition)
                }
                Node::PopSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let node_id = id.into_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(&symbol);
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
                    let symbol_handle = graph.add_symbol(&symbol);
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
                    let symbol_handle = graph.add_symbol(&symbol);
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
                if let Some(source_info) = node.source_info() {
                    *graph.source_info_mut(handle) = crate::graph::SourceInfo {
                        span: source_info.span.clone(),
                        syntax_type: source_info
                            .syntax_type
                            .as_ref()
                            .map(|st| graph.add_string(&st)),
                        ..Default::default()
                    };
                }

                // load debug-info of each node
                if let Some(debug_info) = node.debug_info() {
                    *graph.debug_info_mut(handle) = debug_info.data.iter().fold(
                        crate::graph::DebugInfo::default(),
                        |mut info, entry| {
                            let key = graph.add_string(&entry.key);
                            let value = graph.add_string(&entry.value);
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
        } in &self.edges.data
        {
            let source_id = source.into_node_id(graph)?;
            let sink_id = sink.into_node_id(graph)?;

            let source_handle = graph
                .node_for_id(source_id)
                .ok_or(Error::InvalidGlobalNodeID(source.local_id))?;
            let sink_handle = graph
                .node_for_id(sink_id)
                .ok_or(Error::InvalidGlobalNodeID(sink.local_id))?;

            graph.add_edge(source_handle, sink_handle, *precedence);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Files {
    pub data: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Nodes {
    pub data: Vec<Node>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceInfo {
    pub span: lsp_positions::Span,
    pub syntax_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DebugInfo {
    pub data: Vec<DebugEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DebugEntry {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodeID {
    pub file: Option<String>,
    pub local_id: u32,
}

impl NodeID {
    fn into_node_id(
        &self,
        graph: &crate::graph::StackGraph,
    ) -> Result<crate::graph::NodeID, Error> {
        if let Some(file) = self.file.as_ref() {
            let handle = graph
                .get_file(&file)
                .ok_or(Error::FileNotFound(file.to_owned()))?;
            Ok(crate::graph::NodeID::new_in_file(handle, self.local_id))
        } else if self.is_root() {
            Ok(crate::graph::NodeID::root())
        } else if self.is_jump_to() {
            Ok(crate::graph::NodeID::jump_to())
        } else {
            Err(Error::InvalidGlobalNodeID(self.local_id))
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Edges {
    pub data: Vec<Edge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Edge {
    pub source: NodeID,
    pub sink: NodeID,
    pub precedence: i32,
}

impl crate::graph::StackGraph {
    pub fn to_serializable(&self) -> StackGraph {
        self.to_serializable_filter(&NoFilter)
    }

    pub fn to_serializable_filter<'a>(&self, f: &'a dyn Filter) -> StackGraph {
        crate::serde::StackGraph::from_graph_filter(self, f)
    }

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
