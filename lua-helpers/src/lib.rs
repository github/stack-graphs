// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

const TEST_PRELUDE: &str = r#"
  function assert_eq(thing, expected, actual)
    if expected ~= actual then
      error("Expected "..thing.." "..expected..", got "..actual)
    end
  end

  function deepeq(t1, t2, prefix)
    prefix = prefix or ""
    local ty1 = type(t1)
    local ty2 = type(t2)
    if ty1 ~= ty2 then
      local msg = "different types for lhs"..prefix.." ("..ty1..") and rhs"..prefix.." ("..ty2..")"
      return false, {msg}
    end

    -- non-table types can be directly compared
    if ty1 ~= 'table' and ty2 ~= 'table' then
      if t1 ~= t2 then
        local msg = "different values for lhs"..prefix.." ("..t1..") and rhs"..prefix.." ("..t2..")"
        return false, {msg}
      end
      return true, {}
    end

    local equal = true
    local diffs = {}
    for k2, v2 in pairs(t2) do
      local v1 = t1[k2]
      if v1 == nil then
        equal = false
        diffs[#diffs+1] = "missing lhs"..prefix.."."..k2
      else
        local e, d = deepeq(v1, v2, prefix.."."..k2)
        equal = equal and e
        table.move(d, 1, #d, #diffs+1, diffs)
      end
    end
    for k1, v1 in pairs(t1) do
      local v2 = t2[k1]
      if v2 == nil then
        equal = false
        diffs[#diffs+1] = "missing rhs"..prefix.."."..k1
      end
    end
    return equal, diffs
  end

  function assert_deepeq(thing, expected, actual)
    local eq, diffs = deepeq(expected, actual)
    if not eq then
      error("Unexpected "..thing..": "..table.concat(diffs, ", "))
    end
  end

  function values(t)
    local i = 0
    return function() i = i + 1; return t[i] end
  end

  function iter_tostring(...)
    local result = {}
    for element in ... do
      table.insert(result, tostring(element))
    end
    return result
  end
"#;

pub fn new_lua() -> Result<mlua::Lua, mlua::Error> {
    let l = mlua::Lua::new();
    l.load(TEST_PRELUDE).set_name("test prelude").exec()?;
    Ok(l)
}

pub trait CheckLua {
    fn check(&self, chunk: &str) -> Result<(), mlua::Error>;
}

impl CheckLua for mlua::Lua {
    fn check(&self, chunk: &str) -> Result<(), mlua::Error> {
        self.load(chunk).set_name("test chunk").exec()
    }
}

#[test]
fn can_deepeq_from_lua() -> Result<(), mlua::Error> {
    let l = new_lua()?;
    l.check(
        r#"
          function check_deepeq(lhs, rhs, expected, expected_diffs)
            local actual, actual_diffs = deepeq(lhs, rhs)
            actual_diffs = table.concat(actual_diffs, ", ")
            assert_eq("deepeq", expected, actual)
            assert_eq("differences", expected_diffs, actual_diffs)
          end

          check_deepeq(0, 0, true, "")
          check_deepeq(0, 1, false, "different values for lhs (0) and rhs (1)")

          check_deepeq({"a", "b", "c"}, {"a", "b", "c"}, true, "")
          check_deepeq({"a", "b", "c"}, {"a", "b"}, false, "missing rhs.3")
          check_deepeq({"a", "b", "c"}, {"a", "b", "d"}, false, "different values for lhs.3 (c) and rhs.3 (d)")

          check_deepeq({a=1, b=2, c=3}, {a=1, b=2, c=3}, true, "")
          check_deepeq({a=1, b=2, c=3}, {a=1, b=2}, false, "missing rhs.c")
          check_deepeq({a=1, b=2, c=3}, {a=1, b=2, c=4}, false, "different values for lhs.c (3) and rhs.c (4)")
          check_deepeq({a=1, b=2, c=3}, {a=1, b=2, d=3}, false, "missing lhs.d, missing rhs.c")
        "#,
    )?;
    Ok(())
}
