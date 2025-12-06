#[cfg(test)]
use ljr::prelude::*;

#[test]
fn test_globals_set_and_get_primitives() {
    let lua = Lua::new();
    lua.open_libs();

    let mut globals = lua.globals();

    globals.with_mut(|t| {
        t.set("x", 42i32);
        t.set("f", 3.14f64);
        t.set("b", true);
        t.set("s", "rust_string");
    });

    globals.with(|t| {
        assert_eq!(t.get::<i32>("x"), Some(42));
        assert_eq!(t.get::<f64>("f"), Some(3.14));
        assert_eq!(t.get::<bool>("b"), Some(true));
        assert_eq!(t.get::<String>("s").as_deref(), Some("rust_string"));
    });

    let missing = globals.with(|t| t.get::<i32>("nao_existe"));
    assert_eq!(missing, None);

    assert_eq!(lua.top(), 0);
}

#[test]
fn test_globals_read_write_separation() {
    let mut lua = Lua::new();
    lua.open_libs();

    lua.with_globals_mut(|t| {
        t.set("inteiro", 42);
        t.set("flutuante", 3.14);
        t.set("texto", "Rust");
    });

    lua.with_globals(|t| {
        assert_eq!(t.get::<i32>("inteiro"), Some(42));
        assert_eq!(t.get::<f64>("flutuante"), Some(3.14));

        let len = t.view("texto", |s: &StackStr| s.as_str().unwrap().len());
        assert_eq!(len, Some(4));
    });

    assert_eq!(lua.top(), 0);
}
