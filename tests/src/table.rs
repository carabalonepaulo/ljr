#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_stack_table_mutation() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn modify(table: &mut StackTable) {
            table.with_mut(|t| {
                let current = t.get::<i32>("val").unwrap_or(0);
                t.set("result", current * 2);
                t.set("val", 0);
            });
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local t = { val = 21 }
        test.modify(t)

        -- O Rust deve ter criado 'result' (42) e zerado 'val'
        return t.result == 42 and t.val == 0
        "#,
    );
    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_stack_table_array_push() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn append_numbers(table: &mut StackTable) -> i32 {
            table.with_mut(|t| {
                t.push(30);
                t.push(40);
                t.len() as i32
            })
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local list = {10, 20}
        local new_len = test.append_numbers(list)

        return new_len == 4
            and list[3] == 30
            and list[4] == 40
        "#,
    );
    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_stack_table_pairs_iter() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn sum_values(table: &StackTable) -> i32 {
            table.with(|t| {
                let mut sum = 0;
                for (_k, v) in t.pairs::<String, i32>() {
                    sum += v;
                }
                sum
            })
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<i32>(
        r#"
        local test = require 'test'
        local data = { a = 10, b = 20, c = 5 }
        return test.sum_values(data)
        "#,
    );

    assert_eq!(result, Ok(35));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_stack_table_ipairs_iter() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn concat_strings(table: &StackTable) -> String {
            table.with(|t| {
                let mut result = String::new();
                for (_, s) in t.ipairs::<String>() {
                    result.push_str(&s);
                }
                result
            })
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<String>(
        r#"
        local test = require 'test'
        local words = {'hello', ' ', 'world'}
        return test.concat_strings(words)
        "#,
    );

    assert!(matches!(result, Ok(ref s) if s == "hello world"));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_stack_table_clear() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn wipe(table: &mut StackTable) {
            table.clear();
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local t = { a = 1, b = 2, [1] = 10 }

        if t.a ~= 1 then return false end

        test.wipe(t)

        local count = 0
        for _ in pairs(t) do count = count + 1 end
        return count == 0
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_ref_passthrough() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn pass(table: TableRef) -> TableRef {
            table
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local original = { id = 123 }
        local returned = test.pass(original)

        return original == returned and returned.id == 123
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_stack_table_type_check_error() {
    use ljr::Borrowed;
    use ljr::table::Table;

    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn expect_table(_t: &Table<Borrowed>) -> bool {
            true
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        return test.expect_table("not a table")
        "#,
    );

    assert!(result.is_err());
    if let Err(Error::LuaError(msg)) = result {
        assert!(msg.contains("invalid argument") || msg.contains("bad argument"));
    }

    assert_eq!(lua.top(), 0);
}
