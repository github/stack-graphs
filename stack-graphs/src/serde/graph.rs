// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use thiserror::Error;

use crate::arena::Handle;

use super::Filter;
use super::ImplicationFilter;
use super::NoFilter;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
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
    #[error("node `{0}` is an invalid node")]
    InvalidGlobalNodeID(u32),
    #[error("variable `{0}` is an invalid stack variable")]
    InvalidStackVariable(u32),
    #[error("failed to locate node `{0}` in graph")]
    NodeNotFound(NodeID),
}

impl StackGraph {
    pub fn from_graph<'a>(graph: &crate::graph::StackGraph) -> Self {
        Self::from_graph_filter(graph, &NoFilter)
    }

    pub fn from_graph_filter<'a>(graph: &crate::graph::StackGraph, filter: &'a dyn Filter) -> Self {
        let filter = ImplicationFilter(filter);
        let files = graph.filter_files(&filter);
        let nodes = graph.filter_nodes(&filter);
        let edges = graph.filter_edges(&filter);
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
                    let node_id = id.to_node_id(graph)?;
                    graph.add_drop_scopes_node(node_id)
                }
                Node::PopScopedSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let node_id = id.to_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(&symbol);
                    graph.add_pop_scoped_symbol_node(node_id, symbol_handle, *is_definition)
                }
                Node::PopSymbol {
                    id,
                    symbol,
                    is_definition,
                    ..
                } => {
                    let node_id = id.to_node_id(graph)?;
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
                    let node_id = id.to_node_id(graph)?;
                    let scope_id = scope.to_node_id(graph)?;
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
                    let node_id = id.to_node_id(graph)?;
                    let symbol_handle = graph.add_symbol(&symbol);
                    graph.add_push_symbol_node(node_id, symbol_handle, *is_reference)
                }
                Node::Scope {
                    id, is_exported, ..
                } => {
                    let node_id = id.to_node_id(graph)?;
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
                            .map(|st| graph.add_string(&st))
                            .into(),
                        ..Default::default()
                    };
                }

                // load debug-info of each node
                if let Some(debug_info) = node.debug_info() {
                    *graph.node_debug_info_mut(handle) = debug_info.data.iter().fold(
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
            debug_info,
        } in &self.edges.data
        {
            let source_id = source.to_node_id(graph)?;
            let sink_id = sink.to_node_id(graph)?;

            let source_handle = graph
                .node_for_id(source_id)
                .ok_or(Error::InvalidGlobalNodeID(source.local_id))?;
            let sink_handle = graph
                .node_for_id(sink_id)
                .ok_or(Error::InvalidGlobalNodeID(sink.local_id))?;

            graph.add_edge(source_handle, sink_handle, *precedence);

            // load debug-info of each node
            if let Some(debug_info) = debug_info {
                *graph.edge_debug_info_mut(source_handle, sink_handle) = debug_info
                    .data
                    .iter()
                    .fold(crate::graph::DebugInfo::default(), |mut info, entry| {
                        let key = graph.add_string(&entry.key);
                        let value = graph.add_string(&entry.value);
                        info.add(key, value);
                        info
                    });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Files {
    pub data: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Nodes {
    pub data: Vec<Node>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
    serde(tag = "type", rename_all = "snake_case"),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
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

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct SourceInfo {
    pub span: lsp_positions::Span,
    pub syntax_type: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct DebugInfo {
    pub data: Vec<DebugEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct DebugEntry {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct NodeID {
    pub file: Option<String>,
    pub local_id: u32,
}

impl NodeID {
    pub fn from_node_id(graph: &crate::graph::StackGraph, value: crate::graph::NodeID) -> Self {
        Self {
            file: value.file().map(|f| graph[f].to_string()),
            local_id: value.local_id(),
        }
    }

    pub fn to_node_id(
        &self,
        graph: &crate::graph::StackGraph,
    ) -> Result<crate::graph::NodeID, Error> {
        if let Some(file) = &self.file {
            let file = graph
                .get_file(file)
                .ok_or_else(|| Error::FileNotFound(file.clone()))?;
            Ok(crate::graph::NodeID::new_in_file(file, self.local_id))
        } else if self.local_id == crate::graph::JUMP_TO_NODE_ID {
            Ok(crate::graph::NodeID::jump_to())
        } else if self.local_id == crate::graph::ROOT_NODE_ID {
            Ok(crate::graph::NodeID::root())
        } else {
            Err(Error::InvalidGlobalNodeID(self.local_id))
        }
    }

    pub fn from_node(graph: &crate::graph::StackGraph, handle: Handle<crate::graph::Node>) -> Self {
        Self::from_node_id(graph, graph[handle].id())
    }

    pub fn to_node(
        &self,
        graph: &mut crate::graph::StackGraph,
    ) -> Result<Handle<crate::graph::Node>, Error> {
        let value = self.to_node_id(graph)?;
        Ok(graph
            .node_for_id(value)
            .ok_or_else(|| Error::NodeNotFound(self.clone()))?)
    }
}

impl std::fmt::Display for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}:", file)?;
        }
        write!(f, "{}", self.local_id)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Edges {
    pub data: Vec<Edge>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    serde_with::skip_serializing_none, // must come before derive
    derive(serde::Deserialize, serde::Serialize),
)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Edge {
    pub source: NodeID,
    pub sink: NodeID,
    pub precedence: i32,
    pub debug_info: Option<DebugInfo>,
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
            syntax_type: info.syntax_type.into_option().map(|ty| self[ty].to_owned()),
        })
    }

    fn filter_node_debug_info<'a>(
        &self,
        _filter: &'a dyn Filter,
        handle: Handle<crate::graph::Node>,
    ) -> Option<DebugInfo> {
        self.node_debug_info(handle).map(|info| DebugInfo {
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
                    let debug_info = self.filter_node_debug_info(filter, handle);

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
                            debug_info: self.filter_edge_debug_info(filter, e.source, e.sink),
                        })
                })
                .flatten()
                .collect::<Vec<_>>(),
        }
    }

    fn filter_edge_debug_info<'a>(
        &self,
        _filter: &'a dyn Filter,
        source_handle: Handle<crate::graph::Node>,
        sink_handle: Handle<crate::graph::Node>,
    ) -> Option<DebugInfo> {
        self.edge_debug_info(source_handle, sink_handle)
            .map(|info| DebugInfo {
                data: info
                    .iter()
                    .map(|entry| DebugEntry {
                        key: self[entry.key].to_owned(),
                        value: self[entry.value].to_owned(),
                    })
                    .collect(),
            })
    }
}
