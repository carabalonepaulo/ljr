use luajit2_sys as sys;

pub trait BorrowFromLua<'a, T> {
    fn borrow_from_lua(ptr: *mut sys::lua_State) -> &'a T;
}
