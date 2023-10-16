// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use assert_json_diff::assert_json_eq;
use serde_json;
use serde_json::json;
use stack_graphs::graph;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::serde;
use stack_graphs::stitching::{Database, ForwardPartialPathStitcher, StitcherConfig};
use stack_graphs::NoCancellation;

use crate::test_graphs;

#[test]
fn serde_json_stack_graph() {
    let expected = serde::StackGraph {
        files: serde::Files {
            data: vec!["index.ts".to_owned()],
        },
        nodes: serde::Nodes {
            data: vec![serde::Node::Root {
                id: serde::NodeID {
                    local_id: 1,
                    file: None,
                },
                source_info: Some(serde::SourceInfo {
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
                debug_info: Some(serde::DebugInfo { data: vec![] }),
            }],
        },
        edges: serde::Edges {
            data: vec![serde::Edge {
                source: serde::NodeID {
                    file: None,
                    local_id: 1,
                },
                sink: serde::NodeID {
                    file: Some("index.ts".to_owned()),
                    local_id: 0,
                },
                precedence: 0,
                debug_info: Some(serde::DebugInfo { data: vec![] }),
            }],
        },
    };

    // formatted using: json_pp -json_opt utf8,canonical,pretty,indent_length=4
    let json_data = serde_json::json!(
        {
            "edges" : [
                {
                    "debug_info" : [],
                    "precedence" : 0,
                    "sink" : {
                        "file" : "index.ts",
                        "local_id" : 0
                    },
                    "source" : {
                        "local_id" : 1
                    }
                }
            ],
            "files" : [
                "index.ts"
            ],
            "nodes" : [
                {
                    "debug_info" : [],
                    "id" : {
                        "local_id" : 1
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            }
                        }
                    },
                    "type" : "root"
                }
            ]
        }
    );

    let observed = serde_json::from_value::<serde::StackGraph>(json_data).unwrap();

    assert_eq!(observed, expected);
}

#[test]
fn can_load_serialized_stack_graph() {
    // formatted using: json_pp -json_opt utf8,canonical,pretty,indent_length=4
    let json_data = serde_json::json!(
        {
            "edges" : [
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "index.ts",
                        "local_id" : 0
                    },
                    "source" : {
                        "local_id" : 1
                    }
                }
            ],
            "files" : [
                "index.ts"
            ],
            "nodes" : [
                {
                    "debug_info" : [],
                    "id" : {
                        "local_id" : 1
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            }
                        }
                    },
                    "type" : "root"
                },
                {
                    "debug_info" : [],
                    "id" : {
                        "local_id" : 2
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            }
                        }
                    },
                    "type" : "jump_to_scope"
                },
                {
                    "debug_info" : [
                        {
                            "key" : "tsg_variable",
                            "value" : "@prog.defs"
                        },
                        {
                            "key" : "tsg_location",
                            "value" : "(225, 14)"
                        }
                    ],
                    "id" : {
                        "file" : "index.ts",
                        "local_id" : 0
                    },
                    "is_exported" : false,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "containing_line" : {
                                    "end" : 0,
                                    "start" : 0
                                },
                                "line" : 0,
                                "trimmed_line" : {
                                    "end" : 0,
                                    "start" : 0
                                }
                            }
                        }
                    },
                    "type" : "scope"
                }
            ]
        }
    );
    let observed = serde_json::from_value::<serde::StackGraph>(json_data).unwrap();
    let mut sg = graph::StackGraph::new();
    observed.load_into(&mut sg).unwrap();

    assert_eq!(sg.iter_nodes().count(), 3);
    assert_eq!(sg.iter_files().count(), 1);

    // the scope node should contain debug and source info
    let handle = sg
        .iter_nodes()
        .find(|handle| matches!(sg[*handle], graph::Node::Scope(..)))
        .unwrap();
    assert!(sg.source_info(handle).is_some());
    assert!(sg.node_debug_info(handle).is_some());
}

#[test]
fn can_serialize_graph() {
    let graph: StackGraph = test_graphs::simple::new();
    let actual = serde_json::to_value(graph.to_serializable()).expect("Cannot serialize graph");
    // formatted using: json_pp -json_opt utf8,canonical,pretty,indent_length=4
    let expected = json!(
        {
            "edges" : [
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 2
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 1
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 4
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 2
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 8
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 5
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 4
                    }
                },
                {
                    "debug_info" : [
                        {
                            "key" : "dsl_position",
                            "value" : "line 23 column 4"
                        }
                    ],
                    "precedence" : 0,
                    "sink" : {
                        "local_id" : 1
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 5
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 6
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 5
                    }
                },
                {
                    "precedence" : 1,
                    "sink" : {
                        "local_id" : 2
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 6
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 7
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 6
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 8
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 7
                    }
                },
                {
                    "precedence" : 0,
                    "sink" : {
                        "file" : "test.py",
                        "local_id" : 9
                    },
                    "source" : {
                        "file" : "test.py",
                        "local_id" : 8
                    }
                }
            ],
            "files" : [
                "test.py"
            ],
            "nodes" : [
                {
                    "debug_info" : [],
                    "id" : {
                        "local_id" : 1
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "type" : "root"
                },
                {
                    "debug_info" : [],
                    "id" : {
                        "local_id" : 2
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "type" : "jump_to_scope"
                },
                {
                    "debug_info" : [],
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 1
                    },
                    "is_reference" : true,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 14,
                                    "utf16_offset" : 14,
                                    "utf8_offset" : 14
                                },
                                "line" : 1,
                                "containing_line" : {
                                    "start" : 7,
                                    "end" : 15
                                },
                                "trimmed_line" : {
                                    "start" : 7,
                                    "end" : 15
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 13,
                                    "utf16_offset" : 13,
                                    "utf8_offset" : 13
                                },
                                "line" : 1,
                                "containing_line" : {
                                    "start" : 7,
                                    "end" : 15
                                },
                                "trimmed_line" : {
                                    "start" : 7,
                                    "end" : 15
                                }
                            }
                        },
                        "syntax_type" : "variable"
                    },
                    "symbol" : "x",
                    "type" : "push_symbol"
                },
                {
                    "debug_info" : [],
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 2
                    },
                    "is_reference" : false,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "symbol" : ".",
                    "type" : "push_symbol"
                },
                {
                    "debug_info" : [
                        {
                            "key" : "dsl_var",
                            "value" : "arg_scope"
                        },
                        {
                            "key" : "dsl_position",
                            "value" : "line 31 column 20"
                        }
                    ],
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 3
                    },
                    "is_exported" : true,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "type" : "scope"
                },
                {
                    "debug_info" : [],
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 4
                    },
                    "is_reference" : false,
                    "scope" : {
                        "file" : "test.py",
                        "local_id" : 3
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "symbol" : "()",
                    "type" : "push_scoped_symbol"
                },
                {
                    "debug_info" : [
                        {
                            "key" : "dsl_var",
                            "value" : "lexical_scope"
                        },
                        {
                            "key" : "dsl_position",
                            "value" : "line 8 column 11"
                        }
                    ],
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 5
                    },
                    "is_exported" : false,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "type" : "scope"
                },
                {
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 6
                    },
                    "is_definition" : false,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "symbol" : "()",
                    "type" : "pop_scoped_symbol"
                },
                {
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 7
                    },
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "type" : "drop_scopes"
                },
                {
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 8
                    },
                    "is_definition" : false,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 0
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 0
                                }
                            }
                        }
                    },
                    "symbol" : ".",
                    "type" : "pop_symbol"
                },
                {
                    "id" : {
                        "file" : "test.py",
                        "local_id" : 9
                    },
                    "is_definition" : true,
                    "source_info" : {
                        "span" : {
                            "end" : {
                                "column" : {
                                    "grapheme_offset" : 1,
                                    "utf16_offset" : 1,
                                    "utf8_offset" : 1
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 6
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 6
                                }
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0,
                                "containing_line" : {
                                    "start" : 0,
                                    "end" : 6
                                },
                                "trimmed_line" : {
                                    "start" : 0,
                                    "end" : 6
                                }
                            }
                        },
                        "syntax_type" : "variable"
                    },
                    "symbol" : "x",
                    "type" : "pop_symbol"
                }
            ]
        }
    );
    assert_json_eq!(expected, actual);
}

#[test]
fn can_serialize_partial_paths() {
    let graph: StackGraph = test_graphs::simple::new();
    let mut partials = PartialPaths::new();
    let mut db = Database::new();
    for file in graph.iter_files() {
        ForwardPartialPathStitcher::find_minimal_partial_path_set_in_file(
            &graph,
            &mut partials,
            file,
            &StitcherConfig::default(),
            &NoCancellation,
            |g, ps, p| {
                db.add_partial_path(g, ps, p.clone());
            },
        )
        .expect("Expect path finding to work");
    }
    let actual = serde_json::to_value(&db.to_serializable(&graph, &mut partials))
        .expect("Cannot serialize paths");
    // formatted using: json_pp -json_opt utf8,canonical,pretty,indent_length=4
    let expected = json!(
        [
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 3
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 8
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 9
                },
                "scope_stack_postcondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "scope_stack_precondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 3
                },
                "symbol_stack_postcondition" : {
                    "symbols" : [],
                    "variable" : 1
                },
                "symbol_stack_precondition" : {
                    "symbols" : [
                        {
                            "symbol" : "."
                        },
                        {
                            "symbol" : "x"
                        }
                    ],
                    "variable" : 1
                }
            },
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 1
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 2
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 4
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 5
                        }
                    }
                ],
                "end_node" : {
                    "local_id" : 1
                },
                "scope_stack_postcondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "scope_stack_precondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack_postcondition" : {
                    "symbols" : [
                        {
                            "scopes" : {
                                "scopes" : [
                                    {
                                        "file" : "test.py",
                                        "local_id" : 3
                                    }
                                ],
                                "variable" : 1
                            },
                            "symbol" : "()"
                        },
                        {
                            "symbol" : "."
                        },
                        {
                            "symbol" : "x"
                        }
                    ],
                    "variable" : 1
                },
                "symbol_stack_precondition" : {
                    "symbols" : [],
                    "variable" : 1
                }
            },
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 1
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 2
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 4
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 5
                        }
                    },
                    {
                        "precedence" : 1,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 6
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "local_id" : 2
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 3
                },
                "scope_stack_postcondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "scope_stack_precondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack_postcondition" : {
                    "symbols" : [
                        {
                            "symbol" : "."
                        },
                        {
                            "symbol" : "x"
                        }
                    ],
                    "variable" : 1
                },
                "symbol_stack_precondition" : {
                    "symbols" : [],
                    "variable" : 1
                }
            },
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 1
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 2
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 4
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 5
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 6
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 7
                        }
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 8
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 9
                },
                "scope_stack_postcondition" : {
                    "scopes" : []
                },
                "scope_stack_precondition" : {
                    "scopes" : [],
                    "variable" : 1
                },
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack_postcondition" : {
                    "symbols" : [],
                    "variable" : 1
                },
                "symbol_stack_precondition" : {
                    "symbols" : [],
                    "variable" : 1
                }
            }
        ]
    );
    assert_json_eq!(expected, actual);
}
