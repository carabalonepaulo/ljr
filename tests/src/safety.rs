#[cfg(test)]
use ljr::prelude::*;
#[cfg(test)]
use ljr::sys;
#[cfg(test)]
use std::panic::{self, AssertUnwindSafe};

#[cfg(test)]
fn expect_lua_panic<F: FnOnce() + panic::UnwindSafe>(f: F) {
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(f);
    panic::set_hook(prev_hook);

    match result {
        Ok(_) => panic!("no panic"),
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                *s
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.as_str()
            } else {
                "unknown"
            };

            assert_eq!(msg, "lua state has been closed");
        }
    }
}

#[cfg(test)]
unsafe fn setup_and_kill_vm<T, F>(factory: F) -> T
where
    F: FnOnce(&mut Lua) -> T,
{
    unsafe {
        let ptr = sys::luaL_newstate();
        sys::luaL_openlibs(ptr);

        let mut lua = Lua::from_ptr(ptr);
        let value = factory(&mut lua);
        drop(lua);
        sys::lua_close(ptr);

        value
    }
}

#[test]
fn test_table_access_after_vm_close_panics_safely() {
    let table = unsafe {
        setup_and_kill_vm(|lua| {
            let mut t = lua.create_table();
            t.with_mut(|tab| tab.set("foo", "bar"));
            t
        })
    };

    expect_lua_panic(AssertUnwindSafe(|| {
        let _ = table.len();
    }));
}

#[test]
fn test_ud_access_after_vm_close_panics_safely() {
    struct Test;
    #[user_data]
    impl Test {
        fn get(&self) -> i32 {
            42
        }
    }

    let ud = unsafe { setup_and_kill_vm(|lua| lua.create_ref(Test)) };

    expect_lua_panic(AssertUnwindSafe(|| {
        let _ = ud.as_ref();
    }));
}

#[test]
fn test_fn_ref_call_after_vm_close_panics_safely() {
    let func = unsafe {
        setup_and_kill_vm(|lua| {
            lua.do_string::<FnRef<(), ()>>("return function() end")
                .unwrap()
        })
    };

    expect_lua_panic(AssertUnwindSafe(|| {
        let _ = func.call(());
    }));
}

#[test]
fn test_str_ref_access_after_vm_close_panics_safely() {
    let s_ref = unsafe { setup_and_kill_vm(|lua| lua.create_str("I will survive... or not")) };

    expect_lua_panic(AssertUnwindSafe(|| {
        let _ = s_ref.as_str();
    }));
}

#[test]
fn test_owned_lua_drop_behavior() {
    let t2 = unsafe { setup_and_kill_vm(|lua| lua.create_table()) };
    drop(t2);
}
