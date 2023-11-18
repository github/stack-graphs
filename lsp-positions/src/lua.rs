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
use mlua::Value;

use crate::Offset;
use crate::Position;
use crate::Span;

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
