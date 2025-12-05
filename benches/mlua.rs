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
        let s: mlua::String = func.call(()).unwrap();
        let str_ref = s.to_str().unwrap();
        std::hint::black_box(str_ref.len());
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
