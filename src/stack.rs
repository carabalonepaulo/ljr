use std::fmt::Display;

use crate::sys;

use crate::{
    AnyLuaFunction, AnyNativeFunction, AnyUserData, Coroutine, LightUserData, Nil,
    from_lua::FromLua, is_type::IsType, table::Table, to_lua::ToLua,
};

#[derive(Debug)]
pub struct Stack(pub(crate) *mut sys::lua_State);

impl Stack {
    pub fn is<T: IsType>(&self, idx: i32) -> bool {
        T::is_type(self.0, idx)
    }

    pub fn top(&self) -> i32 {
        unsafe { sys::lua_gettop(self.0) }
    }

    pub fn push(&mut self, value: impl ToLua) {
        value.to_lua(self.0);
    }

    pub fn pop(&mut self, n: i32) {
        unsafe { sys::lua_pop(self.0, n) };
    }

    pub fn cast_to<T: FromLua>(&self, idx: i32) -> Option<T::Output> {
        T::from_lua(self.0, idx)
    }

    pub fn clear(&mut self) {
        unsafe { sys::lua_settop(self.0, 0) };
    }
}

impl Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.top();
        writeln!(f, "Stack: {}", size)?;

        for i in 1..=size {
            write!(f, "[{i}/-{}] ", size - i + 1)?;
            if self.is::<i32>(i) {
                writeln!(f, "{}", self.cast_to::<i32>(i).unwrap())?;
            } else if self.is::<f32>(i) {
                writeln!(f, "{}", self.cast_to::<f32>(i).unwrap())?;
            } else if self.is::<f64>(i) {
                writeln!(f, "{}", self.cast_to::<f64>(i).unwrap())?;
            } else if self.is::<bool>(i) {
                writeln!(f, "{}", self.cast_to::<bool>(i).unwrap())?;
            } else if self.is::<String>(i) {
                writeln!(f, "{}", self.cast_to::<String>(i).unwrap())?;
            } else if self.is::<AnyLuaFunction>(i) {
                writeln!(f, "function")?;
            } else if self.is::<AnyNativeFunction>(i) {
                writeln!(f, "native function")?;
            } else if self.is::<LightUserData>(i) {
                writeln!(f, "light user data")?;
            } else if self.is::<Table>(i) {
                writeln!(f, "table")?;
            } else if self.is::<Coroutine>(i) {
                writeln!(f, "coroutine")?;
            } else if self.is::<Nil>(i) {
                writeln!(f, "nil")?;
            } else if self.is::<AnyUserData>(i) {
                writeln!(f, "user data")?;
            }
        }

        Ok(())
    }
}
