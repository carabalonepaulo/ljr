#[cfg(test)]
use ljr::prelude::*;
#[cfg(test)]
use ljr::table::view::TableView;

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

#[test]
fn test_pop_with_preserves_stack_balance_on_error_simulation() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| t.extend_from_slice(&[1, 2, 3, 4, 5]));

    table.with_mut(|t| while let Some(_) = t.pop_then(|_: &i32| {}) {});

    assert_eq!(table.with(|t| t.len()), 0);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_pop_with_empty_table() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();

    let result = table.with_mut(|t| {
        t.pop_then(|_: &i32| {
            panic!("Closure should not be called on empty table pop");
        })
    });

    assert_eq!(result, None);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_pop_with_userdata_side_effects() {
    let lua = Lua::new();
    lua.open_libs();

    struct Counter {
        count: i32,
    }

    #[user_data]
    impl Counter {
        fn get(&self) -> i32 {
            self.count
        }
    }

    let ud = lua.create_ref(Counter { count: 42 });
    let mut table = lua.create_table();
    table.with_mut(|t| {
        t.push(ud);
    });
    assert_eq!(table.with(|t| t.len()), 1);

    let extracted_value = table.with_mut(|t| t.pop_then(|u: &StackUd<Counter>| u.as_ref().get()));
    assert_eq!(extracted_value, Some(42));
    assert_eq!(table.with(|t| t.len()), 0);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_get_with_str_ref_optimization() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| {
        t.set("msg", "hello world");
    });

    table.with(|t| {
        let len = t.view("msg", |s: &StackStr| s.as_str().unwrap_or("").len());
        assert_eq!(len, Some(11));
    });

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_pop_with_primitive() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| t.extend_from_slice(&[10, 20, 30]));
    assert_eq!(table.with(|t| t.len()), 3);

    table.with_mut(|t| {
        let was_thirty = t.pop_then(|val: &i32| *val == 30);
        assert_eq!(was_thirty, Some(true));
    });
    assert_eq!(table.with(|t| t.len()), 2);

    table.with(|t| {
        let last = t.get::<i32>(3);
        assert_eq!(last, None);

        let new_last = t.get::<i32>(2);
        assert_eq!(new_last, Some(20));
    });

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_get_with_primitive() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| {
        t.set("score", 100);
        t.set("lives", 3);
    });

    table.with(|t| {
        let double_score = t.view("score", |val: &i32| *val * 2);
        assert_eq!(double_score, Some(200));

        let exists: Option<i32> = t.get("score");
        assert_eq!(exists, Some(100));
    });

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_for_each_sum_primitive() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| t.extend_from_slice(&[10, 20, 30]));

    let mut sum = 0;

    table.with(|t| {
        t.for_each(|_k: &i32, v: &i32| {
            sum += *v;
            true
        });
    });

    assert_eq!(sum, 60);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_for_each_filter_types() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| {
        t.set("a", 10);
        t.set("b", "ignorar");
        t.set(100, 20);
        t.set("c", 30);
    });

    let mut count = 0;
    let mut sum = 0;

    table.with(|t| {
        t.for_each(|k: &String, v: &i32| {
            count += 1;
            sum += *v;

            assert!(k == "a" || k == "c");
            true
        });
    });

    assert_eq!(count, 2);
    assert_eq!(sum, 40);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_for_each_break_behavior() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| t.extend_from_slice(&[1, 2, 3, 4, 5]));

    let mut visited = 0;

    table.with(|t| {
        t.for_each(|_k: &i32, v: &i32| {
            visited += 1;
            if *v == 3 {
                return false;
            }
            true
        });
    });

    assert_eq!(visited, 3);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_for_each_nested_str_ref() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| {
        t.set("nome", "lua");
        t.set("tipo", "linguagem");
    });

    let mut total_len = 0;

    table.with(|t| {
        t.for_each(|k: &StackStr, v: &StackStr| {
            total_len += k.as_str().unwrap().len();
            total_len += v.as_str().unwrap().len();
            true
        });
    });

    assert_eq!(total_len, 20);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_for_each_with_userdata_borrow() {
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

    let mut lua = Lua::new();
    lua.open_libs();
    lua.register("item", ItemFactory);

    let table = lua
        .do_string::<TableRef>(
            r#"
            local Item = require 'item'
            local a = Item.new(100)
            local b = Item.new(200)
            return { a, b }
            "#,
        )
        .unwrap();

    let mut sum = 0;
    table.with(|t| {
        t.for_each(|_: &i32, v: &StackUd<Item>| {
            sum += v.as_ref().get();
            true
        });
    });

    assert_eq!(sum, 300);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_pairs_iterator_break_leak() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = lua.create_table();
    table.with_mut(|t| t.extend_from_slice(&[10, 20, 30, 40, 50]));

    assert_eq!(lua.top(), 0);

    table.with(|t| {
        let top = lua.top();
        assert!(top > 0);

        for (_k, v) in t.pairs::<i32, i32>() {
            if v == 20 {
                break;
            }
        }

        assert_eq!(lua.top(), top);
    });

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_builder_simple() {
    let mut lua = Lua::new();
    lua.open_libs();

    let builder = TableBuilder::new(|t| {
        t.push(10);
        t.push(20);
        t.set("active", true);
    })
    .with_capacity(2, 1);

    lua.set_global("config", builder);

    let result = lua.do_string::<bool>(
        r#"
        return config[1] == 10 
           and config[2] == 20 
           and config.active == true
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_builder_nested() {
    let mut lua = Lua::new();
    lua.open_libs();

    let complex_structure = TableBuilder::new(|root| {
        root.set("id", "root_node");

        root.set(
            "metadata",
            TableBuilder::new(|meta| {
                meta.set("version", 1.0);
                meta.set("author", "Rust");
            })
            .with_capacity(0, 2),
        );

        root.set(
            "items",
            TableBuilder::new(|arr| {
                arr.push(TableBuilder::new(|item| {
                    item.set("val", 100);
                }));
                arr.push(TableBuilder::new(|item| {
                    item.set("val", 200);
                }));
            })
            .with_capacity(2, 0),
        );
    });

    lua.set_global("data", complex_structure);

    let result = lua.do_string::<bool>(
        r#"
        if data.id ~= "root_node" then return false end
        if data.metadata.version ~= 1.0 then return false end
        if data.items[1].val ~= 100 then return false end
        if data.items[2].val ~= 200 then return false end
        
        return true
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_builder_return_from_rust() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Api;

    #[user_data]
    impl Api {
        fn make_response(&self, status: i32) -> TableBuilder<impl FnOnce(&mut TableView) + use<>> {
            TableBuilder::new(move |t| {
                t.set("status", status);
                t.set("ok", status == 200);
                t.set(
                    "payload",
                    TableBuilder::new(|p| {
                        p.push("data");
                    }),
                );
            })
        }
    }

    lua.register("api", Api);

    let result = lua.do_string::<bool>(
        r#"
        local api = require 'api'
        local res = api:make_response(200)
        
        return res.status == 200 
           and res.ok == true 
           and res.payload[1] == "data"
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_insert_basic() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 10, 20, 30 });

    table.with_mut(|t| {
        t.insert(2, 15);
    });

    let values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());
    assert_eq!(values, vec![10, 15, 20, 30]);
    assert_eq!(table.with(|t| t.len()), 4);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_insert_boundaries() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { "b" });

    table.with_mut(|t| {
        t.insert(1, "a");
        t.insert(3, "c");
    });

    let values: Vec<String> = table.with(|t| t.ipairs::<String>().map(|(_, v)| v).collect());
    assert_eq!(values, vec!["a", "b", "c"]);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_insert_on_hash_side() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 10 });

    table.with_mut(|t| {
        t.insert(-5, 0);
        t.insert(100, 20);
    });

    let values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());
    assert_eq!(values, vec![10]);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_remove_basic() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 10, 20, 30, 40 });

    table.with_mut(|t| {
        let removed = t.remove::<i32>(2);
        assert_eq!(removed, Ok(20));
    });

    let values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());
    assert_eq!(values, vec![10, 30, 40]);
    assert_eq!(table.with(|t| t.len()), 3);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_remove_boundaries() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { "a", "b", "c" });

    table.with_mut(|t| {
        let first = t.remove::<String>(1);
        assert_eq!(first.as_deref(), Ok("a"));

        let last = t.remove::<String>(2);
        assert_eq!(last.as_deref(), Ok("c"));
    });

    let values: Vec<String> = table.with(|t| t.ipairs::<String>().map(|(_, v)| v).collect());
    assert_eq!(values, vec!["b"]);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_remove_invalid_index() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 10, 20 });

    table.with_mut(|t| {
        let zero = t.remove::<i32>(0);
        assert!(matches!(zero, Err(Error::WrongReturnType)));

        let overflow = t.remove::<i32>(3);
        assert!(matches!(overflow, Err(Error::WrongReturnType)));
    });

    assert_eq!(table.with(|t| t.len()), 2);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_table_insert_remove_integration() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, {});

    table.with_mut(|t| {
        t.insert(1, 10);
        t.insert(1, 20);
        t.insert(1, 30);

        assert_eq!(t.len(), 3);

        let v1 = t.remove::<i32>(1);
        assert_eq!(v1, Ok(30));

        let v2 = t.remove::<i32>(2);
        assert_eq!(v2, Ok(10));
    });

    let values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());
    assert_eq!(values, vec![20]);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_returns_ud() {
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
        fn new(val: i32) -> Item {
            Item { val }
        }
    }

    lua.register("factory", Factory);

    let mut table = lua
        .do_string::<TableRef>(
            r#"
            local F = require 'factory'
            return { F.new(100), F.new(200), F.new(300) }
            "#,
        )
        .unwrap();

    table.with_mut(|t| {
        let ud = t.remove::<UdRef<Item>>(2).expect("Should return userdata");
        assert_eq!(ud.as_ref().get(), 200);
    });

    assert_eq!(table.with(|t| t.len()), 2);

    let sum = table.with(|t| {
        let mut s = 0;
        t.for_each::<i32, StackUd<Item>, _>(|_, item| {
            s += item.as_ref().get();
            true
        });
        s
    });

    assert_eq!(sum, 100 + 300);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_then_borrowed_string() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, {
        "primeiro",
        "segundo",
        "terceiro"
    });

    let length_of_removed = table.with_mut(|t| {
        t.remove_then(2, |s: &StackStr| {
            let str_slice = s.as_str().unwrap();
            assert_eq!(str_slice, "segundo");
            str_slice.len()
        })
    });

    assert_eq!(length_of_removed, Ok(7));
    assert_eq!(table.with(|t| t.len()), 2);

    let values: Vec<String> = table.with(|t| t.ipairs::<String>().map(|(_, v)| v).collect());
    assert_eq!(values, vec!["primeiro", "terceiro"]);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_then_type_mismatch() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 10, "texto", 30 });

    let res = table.with_mut(|t| t.remove_then(2, |v: &i32| *v * 2));

    assert!(matches!(res, Err(Error::WrongReturnType)));
    assert_eq!(table.with(|t| t.len()), 3);

    let val_at_2: String = table.with(|t| t.get(2).unwrap());
    assert_eq!(val_at_2, "texto");
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_success_middle() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 10, 20, 30 });

    table.with_mut(|t| {
        let val = t.remove::<i32>(2);
        assert_eq!(val, Ok(20));
    });

    let values: Vec<i32> = table.with(|t| t.ipairs::<i32>().map(|(_, v)| v).collect());
    assert_eq!(values, vec![10, 30]);
    assert_eq!(table.with(|t| t.len()), 2);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_success_last() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { "a", "b" });

    table.with_mut(|t| {
        let val = t.remove::<String>(2);
        assert_eq!(val.as_deref(), Ok("b"));
    });

    let values: Vec<String> = table.with(|t| t.ipairs::<String>().map(|(_, v)| v).collect());
    assert_eq!(values, vec!["a"]);
    assert_eq!(table.with(|t| t.len()), 1);
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_fail_type_mismatch_preserves_table() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, {
        10,
        "não sou um inteiro",
        30
    });

    table.with_mut(|t| {
        let val = t.remove::<i32>(2);
        assert!(matches!(val, Err(Error::WrongReturnType)));
    });

    assert_eq!(table.with(|t| t.len()), 3);

    let v1: i32 = table.with(|t| t.get(1).unwrap());
    let v2: String = table.with(|t| t.get(2).unwrap());
    let v3: i32 = table.with(|t| t.get(3).unwrap());

    assert_eq!(v1, 10);
    assert_eq!(v2, "não sou um inteiro");
    assert_eq!(v3, 30);

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_remove_out_of_bounds() {
    let lua = Lua::new();
    lua.open_libs();

    let mut table = create_table!(lua, { 1, 2, 3 });

    table.with_mut(|t| {
        assert!(matches!(t.remove::<i32>(0), Err(Error::WrongReturnType)));
        assert!(matches!(t.remove::<i32>(4), Err(Error::WrongReturnType)));
    });

    assert_eq!(table.with(|t| t.len()), 3);
    assert_eq!(lua.top(), 0);
}
