//! Bounded JSON data parsing under the pure Lua 5.1 harness.
mod common;

#[test]
fn null_false_zero_and_integer_lexemes_remain_distinct() {
    common::strict_core()
        .run::<()>(
            "values",
            r#"
      local d = ns.Decode
      local x = assert(d.decode('{"null":null,"false":false,"zero":0,"float":0.0,"exponent":1e0}'))
      assert(x.null == d.null and x.missing == nil and x['false'] == false and x.zero == 0)
      assert(d.integer(x, 'zero') and not d.integer(x, 'float') and not d.integer(x, 'exponent'))
      assert(d.kind(d.decode('{}')) == 'object' and d.kind(d.decode('[]')) == 'array')
      local unicode = assert(d.decode('"\\ud83d\\ude00"'))
      assert(unicode == string.char(240, 159, 152, 128))
      local failures = {'{"a":1,"\\u0061":2}', '01', '1.', '1e', '[1,]', '{"a":1,}',
        'true false', '"\\ud800"', '"\\udc00"', '"\\ud800\\u0041"', '1e400',
        '9007199254740992', '"' .. string.char(192, 128) .. '"', '"raw\nline"'}
      for _, text in ipairs(failures) do
        local value, code = d.decode(text)
        assert(value == nil and type(code) == 'string', text)
      end
    "#,
        )
        .unwrap();
}

#[test]
fn production_byte_depth_string_collection_and_node_budgets_are_enforced() {
    common::strict_core()
        .run::<()>(
            "limits",
            r#"
      local d = ns.Decode
      assert(d.decode('{}' .. string.rep(' ', 32766)))
      assert(d.decode('{}' .. string.rep(' ', 32767)) == nil)
      local deep = string.rep('[', 32) .. '0' .. string.rep(']', 32)
      assert(d.decode(deep))
      assert(d.decode('[' .. deep .. ']') == nil)
      assert(d.decode('"' .. string.rep('x', 4096) .. '"'))
      assert(d.decode('"' .. string.rep('x', 4097) .. '"') == nil)
      local list = '[' .. string.rep('0,', 1023) .. '0]'
      assert(d.decode(list))
      assert(d.decode('[' .. string.rep('0,', 1024) .. '0]') == nil)
      local nested = '[' .. string.rep(list .. ',', 7) .. list .. ']'
      -- The overall byte cap allows this document; visited nodes exceed 8192.
      local value, code = d.decode(nested)
      assert(value == nil and code == 'json_nodes')
      assert(d.decode(42) == nil)
    "#,
        )
        .unwrap();
}
