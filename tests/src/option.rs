#![allow(unused)]
use ljr::prelude::*;

#[test]
fn test_from_lua_option_some_i32() {
    let mut lua = Lua::new();
    let value = lua.do_string::<Option<i32>>("return 123");
    assert_eq!(value, Ok(Some(123)));
}

#[test]
fn test_from_lua_option_none_i32() {
    let mut lua = Lua::new();
    let value = lua.do_string::<Option<i32>>("return nil");
    assert_eq!(value, Ok(None));
}

#[test]
fn test_from_lua_option_some_string() {
    let mut lua = Lua::new();
    let value = lua.do_string::<Option<String>>("return 'hello option'");
    assert_eq!(value, Ok(Some("hello option".to_string())));
}

#[test]
fn test_from_lua_option_none_string() {
    let mut lua = Lua::new();
    let value = lua.do_string::<Option<String>>("return nil");
    assert_eq!(value, Ok(None));
}

#[test]
fn test_from_lua_option_some_bool() {
    let mut lua = Lua::new();
    let value = lua.do_string::<Option<bool>>("return true");
    assert_eq!(value, Ok(Some(true)));
}

#[test]
fn test_from_lua_option_none_bool() {
    let mut lua = Lua::new();
    let value = lua.do_string::<Option<bool>>("return nil");
    assert_eq!(value, Ok(None));
}

#[test]
fn test_to_lua_option_some_i32() {
    let mut lua = Lua::new();
    let opt_value: Option<i32> = Some(456);
    lua.set_global("my_value", opt_value);

    let result = lua.do_string::<i32>("return my_value");
    assert_eq!(result, Ok(456));
}

#[test]
fn test_to_lua_option_none_i32() {
    let mut lua = Lua::new();
    let opt_value: Option<i32> = None;
    lua.set_global("my_value", opt_value);

    let result = lua.do_string::<bool>("return my_value == nil");
    assert_eq!(result, Ok(true));
}

#[test]
fn test_to_lua_option_some_string() {
    let mut lua = Lua::new();
    let opt_value: Option<String> = Some("option string".to_string());
    lua.set_global("my_string", opt_value);

    let result = lua.do_string::<String>("return my_string");
    assert_eq!(result, Ok("option string".to_string()));
}

#[test]
fn test_to_lua_option_none_string() {
    let mut lua = Lua::new();
    let opt_value: Option<String> = None;
    lua.set_global("my_string", opt_value);

    let result = lua.do_string::<bool>("return my_string == nil");
    assert_eq!(result, Ok(true));
}

struct OptionTest;

#[user_data]
impl OptionTest {
    fn maybe_add(a: Option<i32>, b: i32) -> Option<i32> {
        a.map(|val| val + b)
    }

    fn get_name(name: Option<String>) -> String {
        name.unwrap_or_else(|| "No Name".to_string())
    }

    fn set_flag(lua: &mut Lua, flag: Option<bool>) {
        if let Some(value) = flag {
            lua.set_global("global_flag", value);
        } else {
            lua.set_global("global_flag", false);
        }
    }
}

#[test]
fn test_ud_option_arg_and_return_some() {
    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("option_test", OptionTest);

    let result = lua.do_string::<Option<i32>>(
        r#"
        local ot = require 'option_test'
        return ot.maybe_add(10, 5)
        "#,
    );
    assert_eq!(result, Ok(Some(15)));
}

#[test]
fn test_ud_option_arg_and_return_none() {
    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("option_test", OptionTest);

    let result = lua.do_string::<Option<i32>>(
        r#"
        local ot = require 'option_test'
        return ot.maybe_add(nil, 5)
        "#,
    );
    assert_eq!(result, Ok(None));
}

#[test]
fn test_ud_option_string_get_name_some() {
    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("option_test", OptionTest);

    let result = lua.do_string::<String>(
        r#"
        local ot = require 'option_test'
        return ot.get_name('John Doe')
        "#,
    );
    assert_eq!(result, Ok("John Doe".to_string()));
}

#[test]
fn test_ud_option_string_get_name_none() {
    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("option_test", OptionTest);

    let result = lua.do_string::<String>(
        r#"
        local ot = require 'option_test'
        return ot.get_name(nil)
        "#,
    );
    assert_eq!(result, Ok("No Name".to_string()));
}

#[test]
fn test_ud_option_set_flag_some() {
    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("option_test", OptionTest);

    lua.exec(
        r#"
        local ot = require 'option_test'
        ot.set_flag(true)
        "#,
    )
    .ok();

    let flag_value = lua.do_string::<bool>("return global_flag");
    assert_eq!(flag_value, Ok(true));
}

#[test]
fn test_ud_option_set_flag_none() {
    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("option_test", OptionTest);

    lua.exec(
        r#"
        local ot = require 'option_test'
        ot.set_flag(nil)
        "#,
    )
    .ok();

    let flag_value = lua.do_string::<bool>("return global_flag");
    assert_eq!(flag_value, Ok(false));
}

#[test]
fn test_return_opt_owned_ud() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn new(create: bool) -> Option<Test> {
            if create {
                Some(Test { value: 20 })
            } else {
                None
            }
        }

        fn get(&self) -> i32 {
            self.value
        }
    }

    lua.register("test", Test { value: 10 });

    let result = lua.do_string::<bool>(
        r#"
        local Test = require 'test'
        local test = Test.new(true)
        return test:get() == 20
        "#,
    );
    assert!(matches!(result, Ok(true)));
    assert_eq!(lua.top(), 0);

    let result = lua.do_string::<bool>(
        r#"
        local Test = require 'test'
        local test = Test.new(false)
        return test == nil
        "#,
    );
    assert!(matches!(result, Ok(true)));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_opt_ud_ref_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn matches(&self, other: Option<&Test>) -> bool {
            if let Some(other) = other {
                self.value == other.value
            } else {
                false
            }
        }
    }

    struct TestFactory;

    #[user_data]
    impl TestFactory {
        fn new(value: i32) -> Test {
            Test { value }
        }
    }

    lua.register("test", TestFactory);

    let result = lua.do_string::<bool>(
        r#"
        local Test = require 'test'
        local a = Test.new(123)
        local b = Test.new(123)
        return a:matches(b)
        "#,
    );
    assert!(matches!(result, Ok(true)));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_opt_str_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn say_hello(&self, other: Option<&str>) -> String {
            if let Some(other) = other {
                format!("hello {}", other)
            } else {
                String::new()
            }
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local msg = test:say_hello('soreto')
        return msg == 'hello soreto'
        "#,
    );
    assert!(matches!(result, Ok(true)));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_opt_slice_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn len(&self, value: Option<&[u8]>) -> i32 {
            value.map(|v| v.len()).unwrap_or(0) as _
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local msg = 'soreto'
        local len = test:len(msg)
        return len == #msg
        "#,
    );
    assert!(matches!(result, Ok(true)));
    assert_eq!(lua.top(), 0);
}
