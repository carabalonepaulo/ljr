#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_get_global_primitives_and_string() {
    let mut lua = Lua::new();
    lua.open_libs();

    lua.set_global("x", 42i32);
    assert_eq!(lua.get_global::<i32>("x"), Some(42));

    lua.set_global("f", 3.14f64);
    assert_eq!(lua.get_global::<f64>("f"), Some(3.14));

    lua.set_global("b", true);
    assert_eq!(lua.get_global::<bool>("b"), Some(true));

    lua.set_global("s", "hello");
    let s = lua.get_global::<String>("s");
    assert_eq!(s.as_deref(), Some("hello"));
}

#[test]
fn test_get_global_luastr() {
    let mut lua = Lua::new();
    lua.open_libs();

    let s = lua.create_str("hello");
    lua.set_global("ls", s.clone());

    let got = lua.get_global::<LuaStr>("ls");
    assert!(got.is_some());
    assert_eq!(got.unwrap().as_str(), "hello");
}

#[test]
fn test_get_global_luaref_userdata() {
    let mut lua = Lua::new();
    lua.open_libs();

    struct Person {
        value: i32,
    }

    #[user_data]
    impl Person {
        fn get_value(&self) -> i32 {
            self.value
        }
    }

    let r = lua.create_ref(Person { value: 7 });
    lua.set_global("person_ref", r.clone());

    let got = lua.get_global::<LuaRef<Person>>("person_ref");
    assert!(got.is_some());
    assert_eq!(got.unwrap().as_ref().get_value(), 7);
}

#[test]
fn test_get_global_fnref_and_call() {
    let mut lua = Lua::new();
    lua.open_libs();

    lua.exec(
        r#"
            function add(a, b)
                return a + b, true
            end
        "#,
    )
    .unwrap();

    let f = lua.get_global::<FnRef<(i32, i32), (i32, bool)>>("add");
    assert!(f.is_some());

    let result = f.unwrap().call((5, 6)).unwrap();
    assert_eq!(result, (11, true));
}

#[test]
fn test_get_global_option_none_and_some() {
    let mut lua = Lua::new();
    lua.open_libs();

    let none = lua.get_global::<i32>("maybe_nil");
    assert_eq!(none, None);

    lua.set_global("maybe_num", 123i32);
    let some = lua.get_global::<i32>("maybe_num");
    assert_eq!(some, Some(123));
}
