#[cfg(test)]
use ljr::{Error, prelude::*};

#[test]
fn test_str() {
    let mut lua = Lua::new();
    lua.open_libs();

    let value = lua.create_str("hello world");
    lua.with_globals_mut(|g| g.set("global_str", value.clone()));

    assert_eq!(value.as_str(), "hello world");

    let ok = lua
        .do_string::<bool>("return global_str == 'hello world'")
        .unwrap();
    assert!(ok);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_str_ref_as_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn greet(lua: &Lua, name: StrRef) -> StrRef {
            lua.create_str(format!("hello {}", name.as_str()).as_str())
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<String>("return require('test').greet('soreto')");
    assert!(matches!(result, Ok(ref s) if s.as_str() == "hello soreto"));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_str_as_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn greet(lua: &Lua, name: &str) -> StrRef {
            lua.create_str(format!("hello {}", name).as_str())
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<String>("return require('test').greet('soreto')");
    assert!(matches!(result, Ok(ref s) if s.as_str() == "hello soreto"));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_u8_slice_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn len(data: &[u8]) -> i32 {
            data.len() as _
        }

        fn is_magic_header(data: &[u8]) -> bool {
            data == &[0xCA, 0xFE, 0xBA, 0xBE]
        }

        fn sum_bytes(data: &[u8]) -> i32 {
            data.iter().map(|&b| b as i32).sum()
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'

        local txt = "hello"
        if test.len(txt) ~= 5 then return false end
        -- 'h'(104) + 'e'(101) + 'l'(108) + 'l'(108) + 'o'(111) = 532
        if test.sum_bytes(txt) ~= 532 then return false end

        local bin = "a\0b"
        if test.len(bin) ~= 3 then return false end

        local magic = string.char(0xCA, 0xFE, 0xBA, 0xBE)
        if not test.is_magic_header(magic) then return false end
        
        return true
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_u8_slice_arg_wrong_type() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn analyze(_data: &[u8]) -> bool {
            true
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        return test.analyze(true)
        "#,
    );

    assert!(
        matches!(result, Err(Error::LuaError(msg)) if msg.contains("invalid argument") || msg.contains("bad argument"))
    );
    assert_eq!(lua.top(), 0);
}
