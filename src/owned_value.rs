use std::{cell::RefCell, rc::Rc};

use crate::{error::Error, lua::InnerLua};

pub(crate) mod private {
    pub trait Sealed {}
}

pub struct LuaInnerHandle<'a>(pub(crate) &'a RefCell<Rc<InnerLua>>);

pub trait OwnedValue: private::Sealed {
    fn inner_lua<'a>(&'a self) -> LuaInnerHandle<'a>;

    fn try_detach(&self) -> Result<(), Error> {
        let mut guard = self.inner_lua().0.try_borrow_mut()?;
        let main_inner_lua = unsafe { guard.try_main_state()? };
        *guard = main_inner_lua;
        Ok(())
    }

    fn try_anchor_to(&self, other: impl OwnedValue) -> Result<(), Error> {
        let other_guard = other.inner_lua().0.try_borrow()?;
        {
            let self_guard = self.inner_lua().0.try_borrow()?;
            self_guard.assert_context(&other_guard)?;
            if &**self_guard == &**other_guard {
                return Ok(());
            }
        }

        let mut self_guard = self.inner_lua().0.try_borrow_mut()?;
        *self_guard = (*other_guard).clone();

        Ok(())
    }
}
