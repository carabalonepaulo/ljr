#[cfg(test)]
use ljr::{Error, prelude::*};

#[cfg(test)]
use crate::STDERR_LOCK;

#[test]
fn test_reentrancy() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test {
        pub value: i32,
    }

    #[user_data]
    impl Test {
        fn fail(&mut self, other: &Test) -> i32 {
            self.value + other.value
        }

        fn pass_stack(a: &mut StackUd<Test>, b: &StackUd<Test>) -> i32 {
            let va = a.with_mut(|a| a.value);
            let vb = b.with(|b| b.value);
            va + vb
        }

        fn pass_ref(mut a: UdRef<Test>, b: UdRef<Test>) -> i32 {
            let va = a.with_mut(|a| a.value);
            let vb = b.with(|b| b.value);
            va + vb
        }
    }

    lua.register("test", Test { value: 10 });

    {
        let result = lua.do_string::<bool>(
            r#"
            local test = require 'test'
            return test:fail(test) == 20
            "#,
        );
        let expected_msg = "cannot modify value";
        assert!(matches!(result, Err(Error::LuaError(ref msg)) if msg.contains(expected_msg)));
        assert_eq!(lua.top(), 0);
    }

    {
        let result = lua.do_string::<bool>(
            r#"
            local test = require 'test'
            return test:pass_stack(test) == 20
            "#,
        );
        assert!(matches!(result, Ok(true)));
        assert_eq!(lua.top(), 0);
    }

    {
        let result = lua.do_string::<bool>(
            r#"
            local test = require 'test'
            return test:pass_ref(test) == 20
            "#,
        );
        assert!(matches!(result, Ok(true)));
        assert_eq!(lua.top(), 0);
    }
}

#[test]
fn test_callback_reentrancy() {
    let _guard = STDERR_LOCK.lock().unwrap();

    let mut lua = Lua::new();
    lua.open_libs();

    struct System {
        state: String,
    }

    #[user_data]
    impl System {
        fn get_state(&self) -> String {
            self.state.clone()
        }

        fn unsafe_run(&mut self, callback: &StackFn) {
            self.state = "running".to_string();
            callback.call::<_, ()>(()).unwrap();
        }

        fn safe_run(ud: &mut StackUd<System>, callback: &StackFn) {
            {
                ud.with_mut(|v| v.state = "running".to_string());
            }
            callback.call::<_, ()>(()).unwrap();
            {
                ud.with_mut(|v| v.state = "finished".to_string());
            }
        }
    }

    lua.register(
        "sys",
        System {
            state: "idle".to_string(),
        },
    );

    {
        let redirect = gag::BufferRedirect::stderr().unwrap();
        let result = lua.exec(
            r#"
            local sys = require 'sys'
            sys:unsafe_run(function() sys:get_state() end)
            "#,
        );
        let _ = redirect.into_inner();
        let expected_msg = "cannot modify value";
        assert!(matches!(result, Err(Error::LuaError(ref msg)) if msg.contains(expected_msg)));
        assert_eq!(lua.top(), 0);
    }

    {
        let result = lua.do_string::<bool>(
            r#"
            local sys = require 'sys'
            local worked = false
            sys:safe_run(function()
                local s = sys:get_state()
                worked = s == 'running'
            end)
            return worked
            "#,
        );
        assert!(matches!(result, Ok(true)));
        assert_eq!(lua.top(), 0);

        let result = lua.do_string::<String>("return require('sys'):get_state()");
        assert!(matches!(result, Ok(ref s) if s == "finished"));
        assert_eq!(lua.top(), 0);
    }
}

#[test]
fn test_callback_reentrancy_with_result() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct System {
        state: String,
    }

    #[user_data]
    impl System {
        fn get_state(&self) -> String {
            self.state.clone()
        }

        fn unsafe_run(&mut self, callback: &StackFn) -> Result<(), Error> {
            self.state = "running".to_string();
            callback.call::<_, ()>(())
        }
    }

    lua.register(
        "sys",
        System {
            state: "idle".to_string(),
        },
    );

    let result = lua.do_string::<bool>(
        r#"
        local sys = require 'sys'
        local err = sys:unsafe_run(function() sys:get_state() end)
        return err == 'lua error: cannot modify value state: it is currently borrowed/in use'
        "#,
    );
    assert!(matches!(result, Ok(true)));
    assert_eq!(lua.top(), 0);
}
