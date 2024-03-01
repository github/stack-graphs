// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::ops::Range;

use mlua::Error;
use mlua::FromLua;
use mlua::IntoLua;
use mlua::Lua;
use mlua::UserData;
use mlua::UserDataMethods;
use mlua::Value;
use mlua_tree_sitter::TSNode;
use mlua_tree_sitter::TreeWithSource;

use crate::Offset;
use crate::Position;
use crate::Span;
use crate::SpanCalculator;

/// An extension trait that lets you load the `lsp_positions` module into a Lua environment.
pub trait Module {
    /// Loads the `lsp_positions` module into a Lua environment.
    fn open_lsp_positions(&self) -> Result<(), mlua::Error>;
}

impl Module for Lua {
    fn open_lsp_positions(&self) -> Result<(), mlua::Error> {
        let exports = self.create_table()?;
        let sc_type = self.create_table()?;

        let function = self.create_function(|lua, source_value: mlua::String| {
            // We are going to add the Lua string as a user value of the SpanCalculator's Lua
            // wrapper.  That will ensure that the string is not garbage collected before the
            // SpanCalculator, which makes it safe to transmute into a 'static reference.
            let source = source_value.to_str()?;
            let source: &'static str = unsafe { std::mem::transmute(source) };
            let sc = SpanCalculator::new(source);
            let sc = lua.create_userdata(sc)?;
            sc.set_user_value(source_value)?;
            Ok(sc)
        })?;
        sc_type.set("new", function)?;

        #[cfg(feature = "tree-sitter")]
        {
            let function = self.create_function(|lua, tws_value: Value| {
                // We are going to add the tree-sitter treee as a user value of the
                // SpanCalculator's Lua wrapper.  That will ensure that the tree is not garbage
                // collected before the SpanCalculator, which makes it safe to transmute into a
                // 'static reference.
                let tws = TreeWithSource::from_lua(tws_value.clone(), lua)?;
                let source: &'static str = unsafe { std::mem::transmute(tws.src) };
                let sc = SpanCalculator::new(source);
                let sc = lua.create_userdata(sc)?;
                sc.set_user_value(tws_value)?;
                Ok(sc)
            })?;
            sc_type.set("new_from_tree", function)?;
        }

        exports.set("SpanCalculator", sc_type)?;
        self.globals().set("lsp_positions", exports)?;
        Ok(())
    }
}

impl<'lua> FromLua<'lua> for Offset {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> Result<Self, Error> {
        let table = match value {
            Value::Table(table) => table,
            Value::Nil => return Ok(Offset::default()),
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Offset",
                    message: None,
                })
            }
        };
        let utf8_offset = table.get::<_, Option<_>>("utf8_offset")?.unwrap_or(0);
        let utf16_offset = table.get::<_, Option<_>>("utf16_offset")?.unwrap_or(0);
        let grapheme_offset = table.get::<_, Option<_>>("grapheme_offset")?.unwrap_or(0);
        Ok(Offset {
            utf8_offset,
            utf16_offset,
            grapheme_offset,
        })
    }
}

impl<'lua> IntoLua<'lua> for Offset {
    fn into_lua(self, l: &'lua Lua) -> Result<Value<'lua>, Error> {
        let result = l.create_table()?;
        result.set("utf8_offset", self.utf8_offset)?;
        result.set("utf16_offset", self.utf16_offset)?;
        result.set("grapheme_offset", self.grapheme_offset)?;
        Ok(Value::Table(result))
    }
}

fn range_from_lua<'lua>(value: Value<'lua>) -> Result<Range<usize>, Error> {
    let table = match value {
        Value::Table(table) => table,
        Value::Nil => return Ok(0..0),
        _ => {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Range",
                message: None,
            })
        }
    };
    let start = table.get("start")?;
    let end = table.get("end")?;
    Ok(start..end)
}

fn range_into_lua<'lua>(range: Range<usize>, l: &'lua Lua) -> Result<Value<'lua>, Error> {
    let result = l.create_table()?;
    result.set("start", range.start)?;
    result.set("end", range.end)?;
    Ok(Value::Table(result))
}

impl<'lua> FromLua<'lua> for Position {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> Result<Self, Error> {
        let table = match value {
            Value::Table(table) => table,
            Value::Nil => return Ok(Position::default()),
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Position",
                    message: None,
                })
            }
        };
        let line = table.get("line")?;
        let column = table.get("column")?;
        let containing_line = range_from_lua(table.get("containing_line")?)?;
        let trimmed_line = range_from_lua(table.get("trimmed_line")?)?;
        Ok(Position {
            line,
            column,
            containing_line,
            trimmed_line,
        })
    }
}

impl<'lua> IntoLua<'lua> for Position {
    fn into_lua(self, l: &'lua Lua) -> Result<Value<'lua>, Error> {
        let result = l.create_table()?;
        result.set("line", self.line)?;
        result.set("column", self.column)?;
        result.set("containing_line", range_into_lua(self.containing_line, l)?)?;
        result.set("trimmed_line", range_into_lua(self.trimmed_line, l)?)?;
        Ok(Value::Table(result))
    }
}

impl<'lua> FromLua<'lua> for Span {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> Result<Self, Error> {
        let table = match value {
            Value::Table(table) => table,
            Value::Nil => return Ok(Span::default()),
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Span",
                    message: None,
                })
            }
        };
        let start = table.get("start")?;
        let end = table.get("end")?;
        Ok(Span { start, end })
    }
}

impl<'lua> IntoLua<'lua> for Span {
    fn into_lua(self, l: &'lua Lua) -> Result<Value<'lua>, Error> {
        if self == Span::default() {
            return Ok(Value::Nil);
        }
        let result = l.create_table()?;
        result.set("start", self.start)?;
        result.set("end", self.end)?;
        Ok(Value::Table(result))
    }
}

impl UserData for SpanCalculator<'static> {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut(
            "for_line_and_column",
            |_, sc, (line, line_utf8_offset, column_utf8_offset)| {
                Ok(sc.for_line_and_column(line, line_utf8_offset, column_utf8_offset))
            },
        );

        methods.add_method_mut(
            "for_line_and_grapheme",
            |_, sc, (line, line_utf8_offset, column_grapheme_offset)| {
                Ok(sc.for_line_and_grapheme(line, line_utf8_offset, column_grapheme_offset))
            },
        );

        #[cfg(feature = "tree-sitter")]
        methods.add_method_mut("for_node", |_, sc, node: TSNode| Ok(sc.for_node(&node)));
    }
}
