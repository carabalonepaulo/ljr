use ljr::prelude::*;

pub fn do_string_primitive() {
    let mut lua = Lua::new();
    let v = lua.do_string::<i32>("return 12345").unwrap();
    std::hint::black_box(v);
}

pub fn call_fn_primitive() {
    let mut lua = Lua::new();
    lua.open_libs();

    let func = lua
        .do_string::<FnRef<i32, i32>>("return function(x) return x + 1 end")
        .unwrap();

    for i in 0..1000 {
        let v = func.call(i).unwrap();
        std::hint::black_box(v);
    }
}

pub fn call_fn_string_borrowed() {
    let mut lua = Lua::new();
    lua.open_libs();

    let code = "return function() return string.rep('a', 200) end";
    lua.do_string_with(code, |f: &StackFn<(), StackStr>| {
        for _ in 0..1000 {
            std::hint::black_box(f.call_then((), |s| s.as_str().unwrap().len()).unwrap());
        }
    })
    .unwrap();
}

pub fn call_fn_string_owned() {
    let mut lua = Lua::new();
    lua.open_libs();

    let func = lua
        .do_string::<FnRef<(), StrRef>>("return function() return string.rep('a', 200) end")
        .unwrap();

    for _ in 0..1000 {
        std::hint::black_box(func.call_then((), |s| s.as_str().unwrap().len()).unwrap());
    }
}

pub fn call_fn_string_native() {
    let mut lua = Lua::new();
    lua.open_libs();

    let func = lua
        .do_string::<FnRef<(), String>>("return function() return string.rep('a', 200) end")
        .unwrap();

    for _ in 0..1000 {
        std::hint::black_box(func.call_then((), |s| s.len()).unwrap());
    }
}

pub fn userdata_simple() {
    struct Ud {
        val: bool,
    }

    #[user_data]
    impl Ud {
        fn is_ok(&self) -> bool {
            self.val
        }
    }

    let mut lua = Lua::new();
    lua.open_libs();
    lua.with_globals_mut(|g| g.set("obj", Ud { val: true }));
    let v = lua.do_string::<bool>("return obj:is_ok()").unwrap();
    std::hint::black_box(v);
}

pub fn userdata_mut_borrowed() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Ud {
        val: i32,
    }

    #[user_data]
    impl Ud {
        fn add(&mut self, v: i32) -> i32 {
            self.val += v;
            self.val
        }
    }

    lua.with_globals_mut(|g| g.set("obj", Ud { val: 0 }));
    lua.do_string_with(
        "return function(n) return obj:add(n) end",
        |f: &StackFn<i32, i32>| {
            for i in 0..1000 {
                std::hint::black_box(f.call(i).unwrap());
            }
        },
    )
    .unwrap();
}

pub fn userdata_mut_owned() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Ud {
        val: i32,
    }

    #[user_data]
    impl Ud {
        fn add(&mut self, v: i32) -> i32 {
            self.val += v;
            self.val
        }
    }

    lua.with_globals_mut(|g| g.set("obj", Ud { val: 0 }));

    let func = lua
        .do_string::<FnRef<i32, i32>>("return function(n) return obj:add(n) end")
        .unwrap();

    for i in 0..1000 {
        std::hint::black_box(func.call(i).unwrap());
    }
}

pub fn call_ud_static_sum_loop_borrowed() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn sum(a: i32, b: i32) -> i32 {
            a + b
        }
    }

    lua.with_globals_mut(|g| g.set("test", Test));

    let code = r#"
    return function()
        for i = 1, 1000 do
            test.sum(i, i)
        end
    end
    "#;

    std::hint::black_box(
        lua.do_string_with(code, |f: &StackFn<(), ()>| f.call(()).unwrap())
            .unwrap(),
    );
}

pub fn call_ud_static_sum_loop_owned() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn sum(a: i32, b: i32) -> i32 {
            a + b
        }
    }

    lua.with_globals_mut(|g| g.set("test", Test));

    let code = r#"
    return function()
        for i = 1, 1000 do
            test.sum(i, i)
        end
    end
    "#;
    let f = lua.do_string::<FnRef<(), ()>>(code).unwrap();
    std::hint::black_box(f.call(()).unwrap());
}

pub fn call_ud_sum_loop_borrowed() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn sum(&mut self, a: i32) {
            self.value += a;
        }

        fn get(&self) -> i32 {
            self.value
        }
    }

    lua.with_globals_mut(|g| g.set("test", Test { value: 0 }));
    let code = r#"
    return function()
        for i = 1, 1000 do
            test:sum(i)
        end
        return test:get()
    end
    "#;
    let result = lua
        .do_string_with(code, |f: &StackFn<(), ()>| f.call(()).unwrap())
        .unwrap();
    std::hint::black_box(result);
}

pub fn call_ud_sum_loop_owned() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn sum(&mut self, a: i32) {
            self.value += a;
        }

        fn get(&self) -> i32 {
            self.value
        }
    }

    lua.with_globals_mut(|g| g.set("test", Test { value: 0 }));
    let code = r#"
    return function()
        for i = 1, 1000 do
            test:sum(i)
        end
        return test:get()
    end
    "#;
    let f = lua.do_string::<FnRef<(), i32>>(code).unwrap();
    std::hint::black_box(f.call(()).unwrap());
}
