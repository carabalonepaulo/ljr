#[cfg(test)]
use ljr::prelude::*;
#[cfg(test)]
use ljr::sys;

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

    assert!(matches!(
        table.try_with(|t| t.len()),
        Err(Error::LuaStateClosed)
    ));
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
    assert!(matches!(ud.try_as_ref(), Err(Error::LuaStateClosed)));
}

#[test]
fn test_fn_ref_call_after_vm_close_panics_safely() {
    let func = unsafe {
        setup_and_kill_vm(|lua| {
            lua.do_string::<FnRef<(), ()>>("return function() end")
                .unwrap()
        })
    };
    assert!(matches!(func.call(()), Err(Error::LuaStateClosed)));
}

#[test]
fn test_str_ref_access_after_vm_close_panics_safely() {
    let s_ref = unsafe { setup_and_kill_vm(|lua| lua.create_str("I will survive... or not")) };
    assert!(matches!(s_ref.try_as_str(), Err(Error::LuaStateClosed)));
}

#[test]
fn test_owned_lua_drop_behavior() {
    let t2 = unsafe { setup_and_kill_vm(|lua| lua.create_table()) };
    drop(t2);
}

#[test]
fn test_no_uaf() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        value: i32,
    }

    #[user_data]
    impl Test {
        fn sum(&self, a: i32, b: i32) -> i32 {
            self.value + a + b
        }
    }

    lua.with_globals_mut(|g| g.set("test", Test { value: 10 }));
    let ud: UdRef<Test> = lua.with_globals(|g| g.get("test")).unwrap();
    let ud_ref = ud.as_ref();
    std::mem::drop(lua);

    assert_eq!(ud_ref.sum(1, 2), 13);
}
