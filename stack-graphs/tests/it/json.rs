// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use serde_json::json;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;

use crate::test_graphs;

#[test]
fn can_serialize_graph() {
    let graph: StackGraph = test_graphs::simple::new();
    let actual = graph.to_json().to_value().expect("Cannot serialize graph");
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
               "id" : {
                  "file" : "test.py",
                  "local_id" : 2
               },
               "is_reference" : false,
               "symbol" : ".",
               "type" : "push_symbol"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 3
               },
               "is_exported" : true,
               "type" : "scope"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 4
               },
               "is_reference" : false,
               "scope" : {
                  "file" : "test.py",
                  "local_id" : 3
               },
               "symbol" : "()",
               "type" : "push_scoped_symbol"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 5
               },
               "is_exported" : false,
               "type" : "scope"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 6
               },
               "is_definition" : false,
               "symbol" : "()",
               "type" : "pop_scoped_symbol"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 7
               },
               "type" : "drop_scopes"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 8
               },
               "is_definition" : false,
               "symbol" : ".",
               "type" : "pop_symbol"
            },
            {
               "id" : {
                  "file" : "test.py",
                  "local_id" : 9
               },
               "is_definition" : true,
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
        .to_json(&graph, |_, _, _| true)
        .to_value()
        .expect("Cannot serialize paths");
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
                     "local_id" : 2
                  }
               },
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
                     "local_id" : 4
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
                     "local_id" : 5
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
                     "local_id" : 4
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
                     "local_id" : 1
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
                     "local_id" : 5
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
                     "local_id" : 2
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
                     "local_id" : 2
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
                     "file" : "test.py",
                     "local_id" : 5
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
                     "local_id" : 6
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
                     "local_id" : 4
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
                     "local_id" : 5
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
                     "local_id" : 2
                  }
               },
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
                     "local_id" : 2
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
                     "file" : "test.py",
                     "local_id" : 5
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
                     "local_id" : 6
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
                     "local_id" : 4
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
                     "local_id" : 2
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
                     "file" : "test.py",
                     "local_id" : 5
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
                     "local_id" : 2
                  }
               },
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
                     "local_id" : 6
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
                     "local_id" : 4
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
                     "local_id" : 1
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
                     "local_id" : 3
                  }
               },
               {
                  "precedence" : 0,
                  "source" : {
                     "local_id" : 2
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
                     "file" : "test.py",
                     "local_id" : 5
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
                     "local_id" : 2
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
                     "local_id" : 7
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
                     "local_id" : 5
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
                     "local_id" : 2
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
                     "local_id" : 3
                  }
               },
               {
                  "precedence" : 0,
                  "source" : {
                     "local_id" : 2
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
                     "file" : "test.py",
                     "local_id" : 5
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
                     "local_id" : 2
                  }
               },
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
                     "local_id" : 7
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
                     "local_id" : 5
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
                     "local_id" : 2
                  }
               },
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
                     "local_id" : 8
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
                     "local_id" : 2
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
                     "file" : "test.py",
                     "local_id" : 5
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
                     "local_id" : 2
                  }
               },
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
                     "local_id" : 8
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
                     "local_id" : 6
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
                     "local_id" : 4
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
                     "local_id" : 1
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
