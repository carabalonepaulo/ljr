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

pub fn call_fn_string() {
    let mut lua = Lua::new();
    lua.open_libs();

    let func = lua
        .do_string::<FnRef<(), StackStr>>("return function() return string.rep('a', 200) end")
        .unwrap();

    for _ in 0..1000 {
        std::hint::black_box(func.call_then((), |s| s.as_str().unwrap().len()).unwrap());
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
    lua.set_global("obj", Ud { val: true });
    let v = lua.do_string::<bool>("return obj:is_ok()").unwrap();
    std::hint::black_box(v);
}

pub fn userdata_mut() {
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

    lua.set_global("obj", Ud { val: 0 });
    let func = lua
        .do_string::<FnRef<i32, i32>>("return function(n) return obj:add(n) end")
        .unwrap();

    for i in 0..1000 {
        let _ = func.call(i).unwrap();
    }
}
