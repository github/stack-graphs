// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use stack_graphs::graph;
use stack_graphs::serde;

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
                        },
                        "containing_line": {
                            "start": 0,
                            "end": 0,
                        },
                        "trimmed_line": {
                            "start": 0,
                            "end": 0,
                        }
                    },
                    "end": {
                        "line": 0,
                        "column": {
                            "utf8_offset": 0,
                            "utf16_offset": 0,
                            "grapheme_offset": 0
                        },
                        "containing_line": {
                            "start": 0,
                            "end": 0,
                        },
                        "trimmed_line": {
                            "start": 0,
                            "end": 0,
                        }
                    },
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

    let observed = serde_json::from_value::<serde::StackGraph>(json_data).unwrap();

    assert_eq!(observed, expected);
}

#[test]
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
                    },
                    "containing_line": {
                        "start": 0,
                        "end": 0,
                    },
                    "trimmed_line": {
                        "start": 0,
                        "end": 0,
                    }
                  },
                  "end": {
                    "line": 0,
                    "column": {
                      "utf8_offset": 0,
                      "utf16_offset": 0,
                      "grapheme_offset": 0
                    },
                    "containing_line": {
                        "start": 0,
                        "end": 0,
                    },
                    "trimmed_line": {
                        "start": 0,
                        "end": 0,
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
                    },
                    "containing_line": {
                        "start": 0,
                        "end": 0,
                    },
                    "trimmed_line": {
                        "start": 0,
                        "end": 0,
                    }
                  },
                  "end": {
                    "line": 0,
                    "column": {
                      "utf8_offset": 0,
                      "utf16_offset": 0,
                      "grapheme_offset": 0
                    },
                    "containing_line": {
                        "start": 0,
                        "end": 0,
                    },
                    "trimmed_line": {
                        "start": 0,
                        "end": 0,
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
                    },
                    "containing_line": {
                        "start": 0,
                        "end": 0,
                    },
                    "trimmed_line": {
                        "start": 0,
                        "end": 0,
                    }
                  },
                  "end": {
                    "line": 0,
                    "column": {
                      "utf8_offset": 0,
                      "utf16_offset": 0,
                      "grapheme_offset": 0
                    },
                    "containing_line": {
                        "start": 0,
                        "end": 0,
                    },
                    "trimmed_line": {
                        "start": 0,
                        "end": 0,
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
    assert!(sg.debug_info(handle).is_some());
}
