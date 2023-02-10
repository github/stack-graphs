// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde_json::json;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::json::NoFilter;
use stack_graphs::partial::PartialPaths;
use stack_graphs::paths::Paths;
use stack_graphs::stitching::Database;
use stack_graphs::NoCancellation;

use crate::test_graphs;

#[test]
fn can_serialize_graph() {
    let graph: StackGraph = test_graphs::simple::new();
    let actual = graph
        .to_json(&|_: &StackGraph, _: &Handle<File>| true)
        .to_value()
        .expect("Cannot serialize graph");
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 1
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 13,
                                    "utf16_offset" : 13,
                                    "utf8_offset" : 13
                                },
                                "line" : 1
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
                                "line" : 0
                            },
                            "start" : {
                                "column" : {
                                    "grapheme_offset" : 0,
                                    "utf16_offset" : 0,
                                    "utf8_offset" : 0
                                },
                                "line" : 0
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
    assert_eq!(actual, expected);
}

#[test]
fn can_serialize_paths() {
    let graph: StackGraph = test_graphs::simple::new();
    let mut paths = Paths::new();
    let actual = paths
        .to_json(&graph, &|_: &StackGraph, _: &Handle<File>| true)
        .to_value()
        .expect("Cannot serialize paths");
    // formatted using: json_pp -json_opt utf8,canonical,pretty,indent_length=4
    let expected = json!(
        [
            {
                "edges" : [],
                "end_node" : {
                    "local_id" : 1
                },
                "scope_stack" : [],
                "start_node" : {
                    "local_id" : 1
                },
                "symbol_stack" : []
            },
            {
                "edges" : [],
                "end_node" : {
                    "local_id" : 2
                },
                "scope_stack" : [],
                "start_node" : {
                    "local_id" : 2
                },
                "symbol_stack" : []
            },
            {
                "edges" : [],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "x"
                    }
                ]
            },
            {
                "edges" : [],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    }
                ]
            },
            {
                "edges" : [],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 3
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 3
                },
                "symbol_stack" : []
            },
            {
                "edges" : [],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "scope_stack" : [
                    {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                ],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    }
                ]
            },
            {
                "edges" : [],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 5
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 5
                },
                "symbol_stack" : []
            },
            {
                "edges" : [],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 7
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 7
                },
                "symbol_stack" : []
            },
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 1
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
            },
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 2
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    },
                    {
                        "symbol" : "."
                    }
                ]
            },
            {
                "edges" : [
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 4
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 5
                },
                "scope_stack" : [
                    {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                ],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    }
                ]
            },
            {
                "edges" : [
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 5
                },
                "symbol_stack" : []
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    },
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
            },
            {
                "edges" : [
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 5
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    },
                    {
                        "symbol" : "."
                    }
                ]
            },
            {
                "edges" : [
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
                "scope_stack" : [
                    {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                ],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    }
                ]
            },
            {
                "edges" : [
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
                    "file" : "test.py",
                    "local_id" : 6
                },
                "scope_stack" : [
                    {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                ],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "symbol_stack" : []
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 5
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    },
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
            },
            {
                "edges" : [
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    },
                    {
                        "symbol" : "."
                    }
                ]
            },
            {
                "edges" : [
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
                    "file" : "test.py",
                    "local_id" : 6
                },
                "scope_stack" : [
                    {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                ],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    }
                ]
            },
            {
                "edges" : [
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "symbol_stack" : []
            },
            {
                "edges" : [
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 7
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 4
                },
                "symbol_stack" : []
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "scopes" : [
                            {
                                "file" : "test.py",
                                "local_id" : 3
                            }
                        ],
                        "symbol" : "()"
                    },
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
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
                    "file" : "test.py",
                    "local_id" : 6
                },
                "scope_stack" : [
                    {
                        "file" : "test.py",
                        "local_id" : 3
                    }
                ],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
            },
            {
                "edges" : [
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    }
                ]
            },
            {
                "edges" : [
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 7
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    }
                ]
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 7
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "."
                    },
                    {
                        "symbol" : "x"
                    }
                ]
            },
            {
                "edges" : [
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
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 3
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 8
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : []
            },
            {
                "edges" : [
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 8
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 2
                },
                "symbol_stack" : []
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
                    },
                    {
                        "precedence" : 0,
                        "source" : {
                            "file" : "test.py",
                            "local_id" : 3
                        }
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 8
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "x"
                    }
                ]
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
                    }
                ],
                "end_node" : {
                    "file" : "test.py",
                    "local_id" : 8
                },
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : [
                    {
                        "symbol" : "x"
                    }
                ]
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
                    },
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : []
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
                "scope_stack" : [],
                "start_node" : {
                    "file" : "test.py",
                    "local_id" : 1
                },
                "symbol_stack" : []
            }
        ]
    );
    assert_eq!(actual, expected);
}

#[test]
fn can_serialize_partial_paths() {
    let graph: StackGraph = test_graphs::simple::new();
    let mut partials = PartialPaths::new();
    let mut db = Database::new();
    for file in graph.iter_files() {
        partials
            .find_minimal_partial_path_set_in_file(&graph, file, &NoCancellation, |g, ps, p| {
                db.add_partial_path(g, ps, p);
            })
            .expect("Expect path finding to work");
    }
    let actual = db
        .to_json(&graph, &mut partials, &NoFilter)
        .to_value()
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
    assert_eq!(actual, expected);
}
