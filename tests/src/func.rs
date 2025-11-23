#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_fn_ref_unit_return() {
    let mut lua = Lua::new();
    lua.open_libs();

    let lua_fn = lua
        .do_string::<FnRef<String, ()>>("return function(str) return 'hello world' end")
        .unwrap();

    let result = lua_fn.call("hello".into());
    assert!(matches!(result, Ok(())));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_fn_ref_wrong_return() {
    let mut lua = Lua::new();
    lua.open_libs();

    let lua_fn = lua
        .do_string::<FnRef<String, (bool, i32)>>("return function(str) return false end")
        .unwrap();

    let result = lua_fn.call("hello".into());
    assert!(matches!(result, Err(Error::WrongReturnType)));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_fn_ref_no_return() {
    let mut lua = Lua::new();
    lua.open_libs();

    let lua_fn = lua
        .do_string::<FnRef<String, (bool, i32)>>("return function(str) end")
        .unwrap();

    let result = lua_fn.call("hello".into());
    assert!(matches!(result, Err(Error::WrongReturnType)));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_fn_ref_no_arg_no_return() {
    let mut lua = Lua::new();
    lua.open_libs();

    let lua_fn = lua
        .do_string::<FnRef<(), ()>>("return function() end")
        .unwrap();

    let result = lua_fn.call(());
    assert!(matches!(result, Ok(())));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_fn_ref_unit_return_on_user_data() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn call(fn_ref: FnRef<String, ()>) -> (bool, bool) {
            let result = fn_ref.call("hello".into());
            match result {
                Ok(v) => (true, v == ()),
                Err(_) => (false, false),
            }
        }
    }

    lua.register("test", Test);

    lua.do_string::<bool>(
        r#"
        function valid(str)
        end

        function invalid(str)
            return 0
        end
        return true
        "#,
    )
    .unwrap();

    let valid_result = lua.do_string::<(bool, bool)>(
        r#"
        local test = require 'test'
        return test.call(valid)
    "#,
    );

    let invalid_result = lua.do_string::<(bool, bool)>(
        r#"
        local test = require 'test'
        return test.call(invalid)
        "#,
    );

    assert!(matches!(valid_result, Ok((true, true))));
    assert!(matches!(invalid_result, Ok((true, true))));
    assert_eq!(0, lua.top());
}

#[test]
fn test_return_fn_ref() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        fn_ref: Option<FnRef<String, i32>>,
    }

    #[user_data]
    impl Test {
        fn store(&mut self, fn_ref: FnRef<String, i32>) {
            self.fn_ref = Some(fn_ref);
        }

        fn get(&self) -> Option<FnRef<String, i32>> {
            self.fn_ref.as_ref().cloned()
        }
    }

    lua.register("test", Test { fn_ref: None });

    let result = lua.do_string::<i32>(
        r#"
        local test = require 'test'
        assert(test)
        assert(test:get() == nil)
        test:store(function(str) return 123 end)
        assert(test:get())
        return test:get()()
        "#,
    );

    assert!(matches!(result, Ok(123)));
    assert_eq!(0, lua.top());
}
