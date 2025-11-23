#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_str() {
    let mut lua = Lua::new();
    lua.open_libs();

    let value = lua.create_str("hello world");
    lua.set_global("global_str", value.clone());

    assert_eq!(value.as_str(), Some("hello world"));

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
            lua.create_str(format!("hello {}", name.as_str().unwrap()).as_str())
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
