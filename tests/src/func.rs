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
    assert!(matches!(result, Err(Error::UnexpectedType)));
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
    assert!(matches!(result, Err(Error::UnexpectedType)));
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
                _ => (false, false),
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

#[test]
fn test_func_call_stack_leak() {
    let mut lua = Lua::new();
    lua.open_libs();

    let func = lua
        .do_string::<FnRef<(), i32>>("return function() return 42 end")
        .unwrap();

    assert_eq!(lua.top(), 0);

    for _ in 0..100 {
        let result = func.call(());
        assert_eq!(result, Ok(42));
    }

    assert_eq!(lua.top(), 0);
}
#[test]
fn test_callback_with_stack_ud_return() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Item {
        val: i32,
    }
    #[user_data]
    impl Item {
        fn get(&self) -> i32 {
            self.val
        }
    }

    struct ItemFactory;

    #[user_data]
    impl ItemFactory {
        fn new(val: i32) -> Item {
            Item { val }
        }
    }

    struct Runner;
    #[user_data]
    impl Runner {
        fn execute(callback: &StackFn<(), StackUd<Item>>) -> i32 {
            callback
                .call_then((), |item| item.as_ref().get())
                .unwrap_or(-1)
        }
    }

    lua.register("item", ItemFactory);
    lua.register("runner", Runner);

    let result = lua.do_string::<i32>(
        r#"
        local Item = require 'item'
        local runner = require 'runner'
        local my_item = Item.new(123)
        return runner.execute(function() return my_item end)
        "#,
    );

    assert!(matches!(result, Ok(123)));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_callback_with_str_ref_return() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Runner;

    #[user_data]
    impl Runner {
        fn len(cb: &StackFn<(), StackStr>) -> i32 {
            cb.call_then((), |s| s.as_str().len() as i32).unwrap_or(-1)
        }
    }

    lua.register("runner", Runner);

    let result = lua.do_string::<i32>(
        r#"
        local runner = require 'runner'
        local text = "hello world via callback"
        return runner.len(function()
            return text
        end)
        "#,
    );

    assert_eq!(result, Ok(24));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_callback_nested_calls() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Processor;
    #[user_data]
    impl Processor {
        fn process(data: &StackFn<i32, i32>) -> i32 {
            data.call(10).unwrap_or(0) + 5
        }

        fn process_borrowed(data: &StackFn<i32, i32>) -> i32 {
            data.call_then(20, |ret| *ret + 5).unwrap_or(0)
        }
    }

    lua.register("proc", Processor);

    let result = lua.do_string::<bool>(
        r#"
        local proc = require 'proc'
        local res1 = proc.process(function(val) return val * 2 end)
        local res2 = proc.process_borrowed(function(val) return val * 2 end)
        return res1 == 25 and res2 == 45
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_integration_table_iter_func_call_with_stack_ud() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Item {
        val: i32,
    }

    #[user_data]
    impl Item {
        fn get(&self) -> i32 {
            self.val
        }
    }

    struct Factory;

    #[user_data]
    impl Factory {
        fn new(v: i32) -> Item {
            Item { val: v }
        }
    }

    lua.register("factory", Factory);

    let table = lua
        .do_string::<TableRef>(
            r#"
        local Factory = require 'factory'
        return {
            function() return Factory.new(10) end,
            function() return Factory.new(20) end,
            function() return Factory.new(30) end
        }
        "#,
        )
        .unwrap();

    let mut total = 0;

    table.with(|t| {
        t.for_each(|_k: &i32, func: &StackFn<(), StackUd<Item>>| {
            let val = func
                .call_then((), |item_ud: &StackUd<Item>| item_ud.as_ref().get())
                .unwrap_or(0);
            total += val;
            true
        });
    });

    assert_eq!(total, 60);
    assert_eq!(lua.top(), 0);
}
