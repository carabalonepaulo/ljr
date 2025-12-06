use ljr::sys as ffi;
use std::ffi::{CStr, CString};
use std::os::raw::c_int;

unsafe fn check_ok(l: *mut ffi::lua_State, status: c_int) {
    unsafe {
        if status != ffi::LUA_OK {
            let msg = ffi::lua_tostring(l, -1);
            let s = CStr::from_ptr(msg).to_string_lossy();
            panic!("Lua error: {}", s);
        }
    }
}

unsafe extern "C-unwind" fn raw_ud_is_ok(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        ffi::lua_pushboolean(l, 1);
        1
    }
}

#[repr(C)]
struct RawUD2 {
    val: i32,
}

unsafe extern "C-unwind" fn raw_ud_add(l: *mut ffi::lua_State) -> c_int {
    unsafe {
        let ud_ptr = ffi::lua_touserdata(l, 1) as *mut RawUD2;
        let x = ffi::lua_tointeger(l, 2) as i32;

        (*ud_ptr).val += x;

        ffi::lua_pushinteger(l, (*ud_ptr).val as ffi::lua_Integer);
        1
    }
}

pub fn do_string_primitive() {
    unsafe {
        let l = ffi::luaL_newstate();
        if l.is_null() {
            panic!("cannot create state");
        }

        let code = CString::new("return 12345").unwrap();

        let status = ffi::luaL_dostring(l, code.as_ptr());
        check_ok(l, status);

        let v = ffi::lua_tointeger(l, -1);
        std::hint::black_box(v);

        ffi::lua_close(l);
    }
}

pub fn call_fn_primitive() {
    unsafe {
        let l = ffi::luaL_newstate();
        let code = CString::new("return function(x) return x + 1 end").unwrap();

        check_ok(l, ffi::luaL_dostring(l, code.as_ptr()));

        for i in 0..1000 {
            ffi::lua_pushvalue(l, -1);

            ffi::lua_pushinteger(l, i as ffi::lua_Integer);

            check_ok(l, ffi::lua_pcall(l, 1, 1, 0));

            let v = ffi::lua_tointeger(l, -1);
            std::hint::black_box(v);

            ffi::lua_pop(l, 1);
        }

        ffi::lua_close(l);
    }
}

pub fn call_fn_string_borrowed() {
    unsafe {
        let l = ffi::luaL_newstate();
        ffi::luaL_openlibs(l);

        let code = CString::new("return function() return string.rep('a', 200) end").unwrap();
        check_ok(l, ffi::luaL_dostring(l, code.as_ptr()));

        for _ in 0..1000 {
            ffi::lua_pushvalue(l, -1);

            check_ok(l, ffi::lua_pcall(l, 0, 1, 0));

            let mut len: usize = 0;
            let ptr = ffi::lua_tolstring(l, -1, &mut len);

            if !ptr.is_null() {
                let slice = std::slice::from_raw_parts(ptr as *const u8, len);
                let s_str = std::str::from_utf8(slice).unwrap();
                std::hint::black_box(s_str.len());
            }

            ffi::lua_pop(l, 1);
        }

        ffi::lua_close(l);
    }
}

pub fn call_fn_string_native() {
    unsafe {
        let l = ffi::luaL_newstate();
        ffi::luaL_openlibs(l);

        let code = CString::new("return function() return string.rep('a', 200) end").unwrap();
        check_ok(l, ffi::luaL_dostring(l, code.as_ptr()));

        for _ in 0..1000 {
            ffi::lua_pushvalue(l, -1);

            check_ok(l, ffi::lua_pcall(l, 0, 1, 0));

            let mut len: usize = 0;
            let ptr = ffi::lua_tolstring(l, -1, &mut len);

            if !ptr.is_null() {
                let slice = std::slice::from_raw_parts(ptr as *const u8, len);
                let s_str = std::str::from_utf8(slice).unwrap();
                let s = s_str.to_string();
                std::hint::black_box(s.len());
            }

            ffi::lua_pop(l, 1);
        }

        ffi::lua_close(l);
    }
}

pub fn userdata_simple() {
    unsafe {
        let l = ffi::luaL_newstate();

        let mt_name = CString::new("MluaUD").unwrap();
        ffi::luaL_newmetatable(l, mt_name.as_ptr());

        ffi::lua_pushvalue(l, -1);
        ffi::lua_setfield(l, -2, CString::new("__index").unwrap().as_ptr());

        ffi::lua_pushcclosure(l, raw_ud_is_ok, 0);
        ffi::lua_setfield(l, -2, CString::new("is_ok").unwrap().as_ptr());

        ffi::lua_pop(l, 1);

        let _ud = ffi::lua_newuserdata(l, 0);
        ffi::luaL_setmetatable(l, mt_name.as_ptr());
        ffi::lua_setglobal(l, CString::new("obj").unwrap().as_ptr());

        let code = CString::new("return obj:is_ok()").unwrap();
        check_ok(l, ffi::luaL_loadstring(l, code.as_ptr()));

        check_ok(l, ffi::lua_pcall(l, 0, 1, 0));

        let v = ffi::lua_toboolean(l, -1);
        std::hint::black_box(v);

        ffi::lua_close(l);
    }
}

pub fn userdata_mut() {
    unsafe {
        let l = ffi::luaL_newstate();

        let mt_name = CString::new("MluaUD2").unwrap();
        ffi::luaL_newmetatable(l, mt_name.as_ptr());

        ffi::lua_pushvalue(l, -1);
        ffi::lua_setfield(l, -2, CString::new("__index").unwrap().as_ptr());

        ffi::lua_pushcclosure(l, raw_ud_add, 0);
        ffi::lua_setfield(l, -2, CString::new("add").unwrap().as_ptr());

        ffi::lua_pop(l, 1);

        let ud_ptr = ffi::lua_newuserdata(l, std::mem::size_of::<RawUD2>()) as *mut RawUD2;
        (*ud_ptr).val = 0;
        ffi::luaL_setmetatable(l, mt_name.as_ptr());
        ffi::lua_setglobal(l, CString::new("obj").unwrap().as_ptr());

        let code = CString::new("return function(n) return obj:add(n) end").unwrap();
        check_ok(l, ffi::luaL_dostring(l, code.as_ptr()));

        for i in 0..1000 {
            ffi::lua_pushvalue(l, -1);
            ffi::lua_pushinteger(l, i as ffi::lua_Integer);

            check_ok(l, ffi::lua_pcall(l, 1, 1, 0));

            std::hint::black_box(ffi::lua_tointeger(l, -1));

            ffi::lua_pop(l, 1);
        }

        ffi::lua_close(l);
    }
}

pub fn call_ud_static_sum_loop() {
    use mlua_sys::*;

    unsafe {
        let l = luaL_newstate();
        luaL_openlibs(l);

        extern "C-unwind" fn sum(l: *mut lua_State) -> ::std::os::raw::c_int {
            unsafe {
                let a = lua_tointeger(l, 1) as i32;
                let b = lua_tointeger(l, 2) as i32;
                lua_pushinteger(l, (a + b) as lua_Integer);
                1
            }
        }

        lua_newtable(l);
        lua_pushcfunction(l, sum);
        lua_setfield(l, -2, c"sum".as_ptr() as *const i8);

        lua_setglobal(l, c"test".as_ptr() as *const i8);

        let code = r#"
        return function()
            for i = 1, 1000 do
                test.sum(i, i)
            end
        end
        "#;

        let c_code = std::ffi::CString::new(code).unwrap();
        if luaL_loadstring(l, c_code.as_ptr() as *const i8) != LUA_OK {
            let err = lua_tostring(l, -1);
            panic!(
                "load error: {}",
                std::ffi::CStr::from_ptr(err).to_string_lossy()
            );
        }

        if lua_pcall(l, 0, 1, 0) != LUA_OK {
            let err = lua_tostring(l, -1);
            panic!(
                "pcall error: {}",
                std::ffi::CStr::from_ptr(err).to_string_lossy()
            );
        }

        if lua_pcall(l, 0, 0, 0) != LUA_OK {
            let err = lua_tostring(l, -1);
            panic!(
                "pcall error: {}",
                std::ffi::CStr::from_ptr(err).to_string_lossy()
            );
        }

        lua_close(l);
    }
}

pub fn call_ud_sum_loop() {
    use mlua_sys::*;
    use std::ffi::CString;
    use std::hint::black_box;
    use std::os::raw::c_int;

    #[repr(C)]
    struct Test {
        value: i32,
    }

    // callbacks - happy path, minimal, extern "C-unwind"
    unsafe extern "C-unwind" fn test_sum(l: *mut lua_State) -> c_int {
        unsafe {
            let ud = lua_touserdata(l, 1) as *mut Test;
            // happy path: assume ud != null and arg present and is integer
            let a = lua_tointeger_(l, 2) as i32;
            (*ud).value = (*ud).value.wrapping_add(a);
            0
        }
    }

    unsafe extern "C-unwind" fn test_get(l: *mut lua_State) -> c_int {
        unsafe {
            let ud = lua_touserdata(l, 1) as *mut Test;
            lua_pushinteger(l, (*ud).value as lua_Integer);
            1
        }
    }

    unsafe {
        // cria estado
        let l = luaL_newstate();
        luaL_openlibs(l);

        // nomes C (mantemos CString para o chunk)
        let mt_name = CString::new("Test").unwrap();
        let glob_name = CString::new("test").unwrap();

        // cria metatable e seta métodos
        luaL_newmetatable(l, mt_name.as_ptr());

        lua_pushcfunction(l, test_sum);
        lua_setfield(l, -2, b"sum\0".as_ptr() as *const i8);

        lua_pushcfunction(l, test_get);
        lua_setfield(l, -2, b"get\0".as_ptr() as *const i8);

        // __index = metatable para suportar obj:method
        lua_pushvalue(l, -1);
        lua_setfield(l, -2, b"__index\0".as_ptr() as *const i8);

        lua_pop(l, 1); // limpa metatable do topo

        // cria userdata e inicializa value = 0
        let ud = lua_newuserdata(l, std::mem::size_of::<Test>()) as *mut Test;
        (*ud).value = 0;

        // aplica metatable
        luaL_getmetatable(l, mt_name.as_ptr());
        lua_setmetatable(l, -2);

        // registra global "test"
        lua_setglobal(l, glob_name.as_ptr());

        // chunk Lua (hardcoded)
        let code = r#"
        return function()
            for i = 1, 1000 do
                test:sum(i)
            end
            return test:get()
        end
    "#;
        let c_code = CString::new(code).unwrap();

        // load chunk -> leaves returned function on stack
        if luaL_loadstring(l, c_code.as_ptr()) != LUA_OK {
            let err = lua_tostring(l, -1);
            let s = std::ffi::CStr::from_ptr(err).to_string_lossy();
            lua_close(l);
            panic!("load error: {}", s);
        }

        // execute chunk -> 1 result (the function)
        if lua_pcall(l, 0, 1, 0) != LUA_OK {
            let err = lua_tostring(l, -1);
            let s = std::ffi::CStr::from_ptr(err).to_string_lossy();
            lua_close(l);
            panic!("pcall error (load exec): {}", s);
        }

        // call the returned function -> 1 result (the integer)
        if lua_pcall(l, 0, 1, 0) != LUA_OK {
            let err = lua_tostring(l, -1);
            let s = std::ffi::CStr::from_ptr(err).to_string_lossy();
            lua_close(l);
            panic!("pcall error (call func): {}", s);
        }

        // read result and black_box it for benchmark
        let result = lua_tointeger_(l, -1) as i32;
        black_box(result);

        lua_close(l)
    };
}
