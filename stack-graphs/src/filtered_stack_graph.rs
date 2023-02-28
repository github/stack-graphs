use super::Filter;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct StackGraph {
    files: Files,
    nodes: Nodes,
    edges: Edges,
}

impl StackGraph {
    pub fn reconstruct(&self) -> crate::graph::StackGraph {
        todo!()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(transparent)]
pub struct Files {
    data: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(transparent)]
pub struct Nodes {
    data: Vec<Node>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(tag = "type")]
pub enum Node {
    #[serde(rename = "drop_scopes")]
    DropScopes {
        id: NodeID,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "jump_to")]
    JumpTo {
        id: NodeID,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "pop_scoped_symbol")]
    PopScopedSymbol {
        id: NodeID,
        symbol: String,
        is_definition: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "pop_symbol")]
    PopSymbol {
        id: NodeID,
        symbol: String,
        is_definition: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "push_scoped_symbol")]
    PushScopedSymbol {
        id: NodeID,
        symbol: String,
        scope: NodeID,
        is_reference: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "push_symbol")]
    PushSymbol {
        id: NodeID,
        symbol: String,
        is_reference: bool,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "root")]
    Root {
        id: NodeID,
        source_info: Option<SourceInfo>,
        debug_info: Option<DebugInfo>,
    },

    #[serde(rename = "scope")]
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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
    pub fn apply_filter<'a>(&self, filter: &'a dyn Filter) -> StackGraph {
        let files = self.filter_files(filter);
        let nodes = self.filter_nodes(filter);
        let edges = self.filter_edges(filter);

        StackGraph {
            files,
            nodes,
            edges,
        }
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

    fn filter_node<'a>(&self, _filter: &'a dyn Filter, id: super::NodeID) -> NodeID {
        let file = id.file().map(|idx| self[idx].name().to_owned());
        let local_id = id.local_id();
        NodeID { file, local_id }
    }

    fn filter_source_info<'a>(
        &self,
        _filter: &'a dyn Filter,
        handle: super::Handle<super::Node>,
    ) -> Option<SourceInfo> {
        self.source_info(handle).map(|info| SourceInfo {
            span: info.span.clone(),
            syntax_type: info.syntax_type.map(|ty| self[ty].to_owned()),
        })
    }

    fn filter_debug_info<'a>(
        &self,
        _filter: &'a dyn Filter,
        handle: super::Handle<super::Node>,
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
                        super::Node::DropScopes(_node) => Node::DropScopes {
                            id,
                            source_info,
                            debug_info,
                        },
                        super::Node::JumpTo(_node) => Node::JumpTo {
                            id,
                            source_info,
                            debug_info,
                        },
                        super::Node::PopScopedSymbol(node) => Node::PopScopedSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            is_definition: node.is_definition,
                            source_info,
                            debug_info,
                        },
                        super::Node::PopSymbol(node) => Node::PopSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            is_definition: node.is_definition,
                            source_info,
                            debug_info,
                        },
                        super::Node::PushScopedSymbol(node) => Node::PushScopedSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            scope: self.filter_node(filter, node.scope),
                            is_reference: node.is_reference,
                            source_info,
                            debug_info,
                        },
                        super::Node::PushSymbol(node) => Node::PushSymbol {
                            id,
                            symbol: self[node.symbol].to_owned(),
                            is_reference: node.is_reference,
                            source_info,
                            debug_info,
                        },
                        super::Node::Root(_node) => Node::Root {
                            id,
                            source_info,
                            debug_info,
                        },
                        super::Node::Scope(node) => Node::Scope {
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
}
