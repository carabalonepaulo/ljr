mod option;
mod str;

#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_do_string_return_num() {
    let mut lua = Lua::new();
    let value = lua.do_string::<i32>("return 1");
    assert!(matches!(value, Ok(1)));
}

#[test]
fn test_do_string_return_bool() {
    let mut lua = Lua::new();
    let value = lua.do_string::<bool>("return true");
    assert_eq!(value, Ok(true));

    let value = lua.do_string::<bool>("return false");
    assert_eq!(value, Ok(false));
}

#[test]
fn test_do_string_return_f32() {
    let mut lua = Lua::new();
    let value = lua.do_string::<f32>("return 3.14");
    assert!(value.is_ok());
    assert!((value.unwrap() - 3.14).abs() < f32::EPSILON);
}

#[test]
fn test_do_string_return_f64() {
    let mut lua = Lua::new();
    let value = lua.do_string::<f64>("return 1.23456789");
    assert!(value.is_ok());
    assert!((value.unwrap() - 1.23456789).abs() < f64::EPSILON);
}

#[test]
fn test_do_string_return_string() {
    let mut lua = Lua::new();
    let value = lua.do_string::<String>("return 'hello world'");
    assert!(matches!(value, Ok(ref s) if s == "hello world"));
}

#[test]
fn test_do_string_error() {
    let mut lua = Lua::new();
    lua.open_libs();

    let value = lua.do_string::<String>("error('error')");
    let expected_err_msg = r#"[string "error('error')"]:1: error"#.to_string();
    assert_eq!(value, Err(Error::LuaError(expected_err_msg)));
}

#[test]
fn test_simple_user_data() {
    let lua = Lua::new();
    lua.open_libs();

    struct Person;

    #[user_data]
    impl Person {}

    lua.register("person", Person);
}

#[test]
fn test_ud_simple_func() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn sum(a: i32, b: i32) -> i32 {
            a + b
        }
    }

    lua.register("test", Test);

    let value = lua.do_string::<i32>(
        r#"
        local test = require 'test'
        return test.sum(10, 2)
    "#,
    );
    assert!(matches!(value, Ok(12)));
}

#[test]
fn test_ud_fn_tuple() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn sum(v: (i32, i32)) -> (i32, bool) {
            ((v.0 + v.1), false)
        }
    }

    lua.register("test", Test);

    let value = lua.do_string::<(i32, bool)>(
        r#"
        local test = require 'test'
        return test.sum(10, 2)
    "#,
    );
    assert!(matches!(value, Ok((12, false))));
}

#[test]
fn test_ud_mut_self() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn get_value(&self) -> i32 {
            self.value
        }

        fn change(&mut self) {
            self.value = 2190;
        }
    }

    lua.register("test", Test { value: 0 });

    let value = lua.do_string::<i32>(
        r#"
        local test = require 'test'
        test:change()
        return test:get_value()
    "#,
    );
    assert!(matches!(value, Ok(2190)));
}

#[test]
fn test_ud_ctor() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn get_value(&self) -> i32 {
            self.value
        }
    }

    struct Factory;
    #[user_data]
    impl Factory {
        fn new(value: i32) -> Test {
            Test { value }
        }
    }

    lua.register("test", Factory);

    let value = lua.do_string::<bool>(
        r#"
        local Test = require 'test'
        local a = Test.new(123)
        return a:get_value() == 123
    "#,
    );
    assert!(matches!(value, Ok(true)));
}

#[test]
fn test_ud_mut_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn get_value(&self) -> i32 {
            self.value
        }

        fn change(&self, other: &mut Test) {
            other.value = 2190;
        }
    }

    struct Factory;
    #[user_data]
    impl Factory {
        fn new(value: i32) -> Test {
            Test { value }
        }
    }

    lua.register("test", Factory);

    let value = lua.do_string::<i32>(
        r#"
        local Test = require 'test'
        local a = Test.new(1)
        local b = Test.new(23)

        a:change(b)
        return b:get_value()
    "#,
    );
    assert!(matches!(value, Ok(2190)));
}

#[test]
fn test_ud_borrow_checker() {
    use gag::BufferRedirect;

    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn test(&self, _other: &mut Test) {}
    }

    lua.register("test", Test);

    let redirect = BufferRedirect::stderr().unwrap();

    let value = lua.do_string::<i32>(
        r#"
        local test = require 'test'
        test:test(test)
        "#,
    );

    let _ = redirect.into_inner();

    let expected_msg = "RefCell already borrowed";
    assert!(matches!(value, Err(Error::LuaError(ref msg)) if msg.contains(expected_msg)));
}

#[test]
fn test_table() {
    let mut lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with(|t| {
        t.push(10i32);
        t.push(false);
        t.push("hello world");

        t.set("name", "soreto");
    });
    lua.register("value", table);

    let value = lua.do_string::<bool>(
        r#"
        local value = require 'value'
        return value[1] == 10 and value[2] == false and value[3] == 'hello world' and value.name == 'soreto'
        "#,
    );
    assert_eq!(value, Ok(true));
}

#[test]
fn test_ud_table_arg() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn use_table(mut table: Table) {
            table.with(|t| {
                t.set(false, 123);
            })
        }
    }

    lua.register("test", Test);

    let value = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local value = {}
        test.use_table(value)
        return value[false] == 123
        "#,
    );
    assert_eq!(value, Ok(true));
}

#[test]
fn test_ud_inject_lua() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test2;

    #[user_data]
    impl Test2 {
        fn get_value() -> i32 {
            123
        }
    }

    struct Test;

    #[user_data]
    impl Test {
        fn test(lua: &Lua) {
            lua.register("test2", Test2);
        }
    }

    lua.register("xxx", Test);

    let value = lua.do_string::<bool>(
        r#"
        local test = require 'xxx'
        test.test()

        local test2 = require 'test2'
        return test2.get_value() == 123
        "#,
    );
    assert_eq!(value, Ok(true));
}

#[test]
fn test_create_ref() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn get_value(&self) -> i32 {
            self.value
        }
    }

    let mut test_ref = lua.create_ref(Test { value: 0 });
    test_ref.with_mut(|t| t.value = 123);
    lua.set_global("test_value", test_ref);

    let value = lua.do_string::<bool>("return test_value:get_value() == 123");
    assert_eq!(value, Ok(true));
}

#[test]
fn test_ud_fn_wrong_arg_count_error() {
    let mut lua = Lua::new();
    lua.open_libs();
    struct Test;
    #[user_data]
    impl Test {
        fn sum(a: i32, b: i32) -> i32 {
            a + b
        }
    }
    lua.register("test", Test);
    let value = lua.do_string::<i32>("local test = require 'test'; return test.sum(10)"); // Apenas 1 argumento
    let err_msg = "wrong number of arguments";
    assert!(matches!(value, Err(Error::LuaError(msg)) if msg.contains(err_msg)));
}

#[test]
fn test_ud_fn_wrong_arg_type_error() {
    let mut lua = Lua::new();
    lua.open_libs();
    struct Test;
    #[user_data]
    impl Test {
        fn sum(a: i32, b: i32) -> i32 {
            a + b
        }
    }
    lua.register("test", Test);
    let value = lua.do_string::<i32>("local test = require 'test'; return test.sum(10, 'hello')"); // String em vez de i32
    assert!(matches!(value, Err(Error::LuaError(msg)) if msg.contains("invalid argument")));
}

#[test]
fn test_table_iter_ipairs() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with(|t| {
        t.push(10i32);
        t.push(20i32);
        t.push(30i32);
    });

    let values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());

    assert_eq!(values, vec![10, 20, 30]);
}

#[test]
fn test_table_iter_pairs() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with(|t| {
        t.push(10i32);
        t.set("name", "Alice");
        t.push(20i32);
        t.set(false, 123i32);
        t.push(30i32);
    });

    let values: Vec<(String, String)> = table.with(|t| t.pairs::<String, String>().collect());
    let mut expected = vec![("name".to_string(), "Alice".to_string())];
    expected.sort_unstable();

    assert_eq!(values, expected);
}

#[test]
fn test_create_table_with_macros() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, {
        "hello",
        "world",
        10,
        20,
        30,
        true,
        false,

        "name" => "Alice",
        12 => false,
        true => "ulala",
    });

    assert_eq!(table.len(), 7);

    {
        let values: Vec<(String, String)> = table.with(|t| t.pairs::<String, String>().collect());
        assert_eq!(&values, &[("name".to_string(), "Alice".to_string())]);
    }

    {
        let mut values: Vec<(i32, bool)> = table.with(|t| t.pairs::<i32, bool>().collect());
        values.sort_unstable();
        assert_eq!(&values, &[(6, true), (7, false), (12, false)]);
    }

    {
        let values: Vec<(bool, String)> = table.with(|t| t.pairs::<bool, String>().collect());
        assert_eq!(&values, &[(true, "ulala".to_string())]);
    }

    {
        let mut values: Vec<String> =
            table.with(|t| t.ipairs::<String>().map(|(_, v)| v).collect());
        values.sort_unstable();
        assert_eq!(&values, &["hello", "world"]);
    }

    {
        let mut values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());
        values.sort_unstable();
        assert_eq!(&values, &[10, 20, 30]);
    }
}

#[test]
fn test_table_clear() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, {
        "hello",
        "world",
        10,
        20,
        30,
        true,
        false
    });
    assert_eq!(table.len(), 7);
    assert_eq!(table.with(|t| t.len()), 7);
    table.with(|t| t.clear());
    assert_eq!(table.len(), 0);
}

#[test]
fn test_stack_fn() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn test_fn(stack_fn: &StackFn<(i32, i32), (i32, bool)>) -> (i32, bool) {
            stack_fn.call((12, 4)).unwrap_or((0, false))
        }
    }
    lua.register("test", Test);

    let result = lua.do_string::<(i32, bool)>(
        r#"
        local fn = function(a, b)
            return a + b, true
        end

        local test = require 'test'
        return test.test_fn(fn)
        "#,
    );
    assert!(matches!(result, Ok((16, true))));
}

#[test]
fn test_fn_ref() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        callback: Option<FnRef<(i32, i32), (i32, bool)>>,
    }

    #[user_data]
    impl Test {
        fn store(&mut self, fn_ref: FnRef<(i32, i32), (i32, bool)>) {
            self.callback = Some(fn_ref);
        }

        fn call(&self, args: (i32, i32)) -> (i32, bool) {
            self.callback
                .as_ref()
                .map(|cb| cb.call(args).unwrap_or((0, false)))
                .unwrap_or((0, false))
        }
    }

    lua.register("test", Test { callback: None });

    let result = lua.do_string::<(i32, bool)>(
        r#"
        local fn = function(a, b)
            return a + b, true
        end

        local test = require 'test'
        test:store(fn)
        return test:call(14, 2)
        "#,
    );
    assert!(matches!(result, Ok((16, true))));
}

#[test]
fn test_table_extend_from_slice() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, {});
    assert_eq!(table.len(), 0);

    table.extend_from_slice(&[10, 20, 30]);
    assert_eq!(table.len(), 3);

    table.clear();
    assert_eq!(table.len(), 0);
}

#[test]
fn test_table_extend_from_map() {
    use std::collections::HashMap;

    let lua = Lua::new();
    lua.open_libs();

    let mut map: HashMap<String, bool> = HashMap::new();
    map.insert("hello".to_string(), false);
    map.insert("world".to_string(), true);

    let mut table = create_table!(lua, {});
    table.extend_from_map(&map);

    let mut values: Vec<(String, bool)> = table.with(|t| t.pairs::<String, bool>().collect());
    values.sort_unstable();
    assert_eq!(&values, &[("hello".into(), false), ("world".into(), true)]);
}
