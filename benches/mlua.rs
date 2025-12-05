use criterion::{Criterion, criterion_group, criterion_main};

mod mlua_tests {
    use mlua::{Lua, UserData};

    pub struct MluaUD;

    impl UserData for MluaUD {
        fn add_methods<'lua, M: mlua::UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("is_ok", |_lua, _this, ()| Ok(true));
        }
    }

    pub struct MluaUD2 {
        pub val: i32,
    }

    impl UserData for MluaUD2 {
        fn add_methods<'lua, M: mlua::UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method_mut("add", |_lua, this, x: i32| {
                this.val += x;
                Ok(this.val)
            });
        }
    }

    pub fn do_string_primitive() {
        let lua = Lua::new();
        let v: i64 = lua.load("return 12345").eval().unwrap();
        std::hint::black_box(v);
    }

    pub fn call_fn_primitive() {
        let lua = Lua::new();
        let func = lua
            .load("return function(x) return x + 1 end")
            .eval::<mlua::Function>()
            .unwrap();

        for i in 0..1000 {
            let v: i64 = func.call(i).unwrap();
            std::hint::black_box(v);
        }
    }

    pub fn call_fn_string() {
        let lua = Lua::new();
        let func = lua
            .load("return function() return string.rep('a', 200) end")
            .eval::<mlua::Function>()
            .unwrap();

        for _ in 0..1000 {
            let s: String = func.call(()).unwrap();
            std::hint::black_box(s.len());
        }
    }

    pub fn userdata_simple() {
        let lua = Lua::new();
        lua.globals().set("obj", MluaUD).unwrap();
        let func = lua.load("return obj:is_ok()").into_function().unwrap();
        let v: bool = func.call(()).unwrap();
        std::hint::black_box(v);
    }

    pub fn userdata_mut() {
        let lua = Lua::new();
        lua.globals().set("obj", MluaUD2 { val: 0 }).unwrap();

        let func = lua
            .load("return function(n) return obj:add(n) end")
            .eval::<mlua::Function>()
            .unwrap();

        for i in 0..1000 {
            let _ = func.call::<i32>(i).unwrap();
        }
    }
}

mod ljr_tests {
    use ljr::prelude::*;

    pub fn empty() {
        // vazio conforme solicitado
    }

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
}

fn bench_do_string_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("do_string_primitive");
    group.bench_function("mlua_do_string_primitive", |b| {
        b.iter(|| mlua_tests::do_string_primitive())
    });

    group.bench_function("ljr_do_string_primitive", |b| {
        b.iter(|| ljr_tests::do_string_primitive())
    });
    group.finish();
}

fn bench_call_fn_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("call_fn_primitive");
    group.bench_function("mlua_call_fn_primitive", |b| {
        b.iter(|| mlua_tests::call_fn_primitive())
    });

    group.bench_function("ljr_call_fn_primitive", |b| {
        b.iter(|| ljr_tests::call_fn_primitive())
    });
    group.finish();
}

fn bench_call_fn_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("call_fn_string");
    group.bench_function("mlua_call_fn_string", |b| {
        b.iter(|| mlua_tests::call_fn_string())
    });

    group.bench_function("ljr_call_fn_string", |b| {
        b.iter(|| ljr_tests::call_fn_string())
    });
    group.finish();
}

fn bench_userdata_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("userdata_simple");
    group.bench_function("mlua_userdata_simple", |b| {
        b.iter(|| mlua_tests::userdata_simple())
    });

    group.bench_function("ljr_userdata_simple", |b| {
        b.iter(|| ljr_tests::userdata_simple())
    });
    group.finish();
}

fn bench_userdata_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("userdata_mut");
    group.bench_function("mlua_userdata_mut", |b| {
        b.iter(|| mlua_tests::userdata_mut())
    });

    group.bench_function("ljr_userdata_mut", |b| b.iter(|| ljr_tests::userdata_mut()));
    group.finish();
}

criterion_group!(
    benches,
    bench_do_string_primitive,
    bench_call_fn_primitive,
    bench_call_fn_string,
    bench_userdata_simple,
    bench_userdata_mut
);

criterion_main!(benches);
