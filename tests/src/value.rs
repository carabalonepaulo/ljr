#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_primitives() {
    let mut lua = Lua::new();
    lua.with_globals_mut(|g| {
        g.set("nil_v", Nil);
        g.set("bool_v", true);
        g.set("num_v", 123.123);
    });

    lua.with_globals(|g| {
        g.view("nil_v", |v: &StackValue| {
            assert!(matches!(v.try_as_nil(), Ok(Nil)))
        });
        g.view("bool_v", |v: &StackValue| {
            assert!(matches!(v.try_as_bool(), Ok(true)))
        });
        g.view("num_v", |v: &StackValue| {
            assert!(matches!(v.try_as_number(), Ok(123.123)))
        });

        assert_eq!(g.get("nil_v"), Some(Nil));
        assert_eq!(g.get("bool_v"), Some(true));
        assert_eq!(g.get("num_v"), Some(123.123));
    });
}

#[test]
fn test_fn() {
    let mut lua = Lua::new();

    lua.exec("function sum(a, b) return a + b end")
        .unwrap_display();

    lua.with_globals(|g| {
        g.view("sum", |v: &StackValue| {
            let result = v
                .try_with_func(|f: &StackFn<(i32, i32), i32>| f.call((2, 3)).unwrap())
                .unwrap();
            assert_eq!(result, 5);
        })
    });
}

#[test]
fn test_stack_diff_ctx() {
    let mut lua_a = Lua::new();
    lua_a.with_globals_mut(|g| g.set("str_value", "hello world"));

    let mut lua_b = Lua::new();
    lua_b.with_globals_mut(|g| g.set("str_value", "hello world"));

    let result = lua_a
        .with_globals(|ag| {
            lua_b.with_globals(|bg| {
                ag.view("str_value", |a_str: &StackStr| {
                    bg.view("str_value", |b_str: &StackStr| a_str == b_str)
                })
            })
        })
        .flatten();

    assert!(matches!(result, Some(false)));
}
