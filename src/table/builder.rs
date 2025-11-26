use crate::sys;
use crate::{
    table::{StackTable, TableView},
    to_lua::ToLua,
};

#[derive(Debug)]
pub struct TableBuilder<F: FnOnce(&mut TableView)> {
    pub(crate) builder: F,
    narr: i32,
    nrec: i32,
}

impl<F> TableBuilder<F>
where
    F: FnOnce(&mut TableView),
{
    pub fn new(f: F) -> Self {
        Self {
            builder: f,
            narr: 0,
            nrec: 0,
        }
    }

    pub fn with_capacity(mut self, narr: i32, nrec: i32) -> Self {
        self.narr = narr;
        self.nrec = nrec;
        self
    }
}

unsafe impl<F> ToLua for TableBuilder<F>
where
    F: FnOnce(&mut TableView),
{
    fn to_lua(self, ptr: *mut mlua_sys::lua_State) {
        unsafe { sys::lua_createtable(ptr, self.narr, self.nrec) };
        let mut table = StackTable::borrowed(ptr, -1);
        table.with_mut(self.builder);
    }

    fn len() -> i32 {
        1
    }
}
