// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lsp_positions::lua::Module;
use lua_helpers::new_lua;
use lua_helpers::CheckLua;

#[test]
fn can_calculate_positions_from_lua() -> Result<(), mlua::Error> {
    let l = new_lua()?;
    l.open_lsp_positions()?;
    l.check(
        r#"
          local source = "   from a import *   "
          local sc = lsp_positions.SpanCalculator.new(source)
          local position = sc:for_line_and_column(0, 0, 9)
          local expected = {
            line=0,
            column={
              utf8_offset=9,
              utf16_offset=9,
              grapheme_offset=9,
            },
            containing_line={start=0, ["end"]=21},
            trimmed_line={start=3, ["end"]=18},
          }
          assert_deepeq("position", expected, position)
        "#,
    )?;
    Ok(())
}

#[cfg(feature = "tree-sitter")]
#[test]
fn can_calculate_tree_sitter_spans_from_lua() -> Result<(), anyhow::Error> {
    let code = br#"
      def double(x):
          return x * 2
    "#;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree_sitter_python::language()).unwrap();
    let parsed = parser.parse(code, None).unwrap();

    use mlua_tree_sitter::Module;
    use mlua_tree_sitter::WithSource;
    let l = new_lua()?;
    l.open_lsp_positions()?;
    l.open_ltreesitter()?;
    l.globals().set("parsed", parsed.with_source(code))?;

    l.check(
        r#"
          local module = parsed:root()
          local double = module:child(0)
          local name = double:child_by_field_name("name")
          local sc = lsp_positions.SpanCalculator.new_from_tree(parsed)
          local position = sc:for_node(name)
          local expected = {
            start={
              line=1,
              column={
                utf8_offset=10,
                utf16_offset=10,
                grapheme_offset=10,
              },
              containing_line={start=1, ["end"]=21},
              trimmed_line={start=7, ["end"]=21},
            },
            ["end"]={
              line=1,
              column={
                utf8_offset=16,
                utf16_offset=16,
                grapheme_offset=16,
              },
              containing_line={start=1, ["end"]=21},
              trimmed_line={start=7, ["end"]=21},
            },
          }
          assert_deepeq("position", expected, position)
        "#,
    )?;
    Ok(())
}
