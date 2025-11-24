#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_result_ok_primitive() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn do_success() -> Result<i32, String> {
            Ok(100)
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local val, err = test.do_success()        
        return val == 100 and err == nil
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_result_err_primitive() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Test;

    #[user_data]
    impl Test {
        fn do_fail() -> Result<i32, String> {
            Err("fail".to_string())
        }
    }

    lua.register("test", Test);

    let result = lua.do_string::<bool>(
        r#"
        local test = require 'test'
        local val, err = test.do_fail()        
        return val == nil and err == "fail"
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_result_idiomatic_check() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Calculator;

    #[user_data]
    impl Calculator {
        fn div(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("division by zero".into())
            } else {
                Ok(a / b)
            }
        }
    }

    lua.register("calc", Calculator);

    let error_case = lua.do_string::<String>(
        r#"
        local calc = require 'calc'
        local res, err = calc.div(10, 0)
        
        if not res then
            return "erro capturado: " .. err
        end
        return "sucesso inesperado"
        "#,
    );
    assert_eq!(
        error_case,
        Ok("erro capturado: division by zero".to_string())
    );

    let success_case = lua.do_string::<i32>(
        r#"
        local calc = require 'calc'
        local res, err = calc.div(10, 2)
        
        if not res then
            error(err)
        end
        return res
        "#,
    );
    assert_eq!(success_case, Ok(5));

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_result_ok_userdata() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct User {
        id: i32,
    }
    #[user_data]
    impl User {
        fn get_id(&self) -> i32 {
            self.id
        }
    }

    struct Repo;
    #[user_data]
    impl Repo {
        fn find(id: i32) -> Result<User, String> {
            if id > 0 {
                Ok(User { id })
            } else {
                Err("invalid id".into())
            }
        }
    }

    lua.register("repo", Repo);

    let result = lua.do_string::<bool>(
        r#"
        local repo = require 'repo'
        
        local user, err = repo.find(10)
        if user == nil then return false end
        if user:get_id() ~= 10 then return false end
        if err ~= nil then return false end

        local user2, err2 = repo.find(-1)
        if user2 ~= nil then return false end
        if err2 ~= "invalid id" then return false end

        return true
        "#,
    );

    assert_eq!(result, Ok(true));
    assert_eq!(lua.top(), 0);
}

#[test]
fn test_result_custom_error_code() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct System;
    #[user_data]
    impl System {
        fn check_status() -> Result<String, i32> {
            Err(503)
        }
    }

    lua.register("sys", System);

    let result = lua.do_string::<i32>(
        r#"
        local sys = require 'sys'
        local status, code = sys.check_status()
        
        if not status then
            return code
        end
        return 0
        "#,
    );

    assert_eq!(result, Ok(503));
    assert_eq!(lua.top(), 0);
}
