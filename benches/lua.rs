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

pub fn call_fn_string() {
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

            let _res = ffi::lua_tointeger(l, -1);

            ffi::lua_pop(l, 1);
        }

        ffi::lua_close(l);
    }
}
