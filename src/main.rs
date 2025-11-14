use ljr::{UserData, lua::Lua};
use luajit2_sys as sys;
use macros::user_data;

struct Math;

#[user_data]
impl Math {
    fn new() -> Self {
        Math {}
    }

    fn sum(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

impl Drop for Math {
    fn drop(&mut self) {
        println!("dropping math");
    }
}

fn main() {
    let mut lua = Lua::new();
    lua.open_libs();

    lua.register("math", Math::new());

    lua.do_string::<()>(
        r#"
        local math = require 'math'
        print(string.format('sum: %d', math:sum(2, 2)))
        "#,
    )
    .ok();

    // lua.set_global("math", math);

    // match lua.do_string::<i32>("local a = math:sum(20, 12); math = nil; print('nilou'); return a") {
    //     Ok(n) => println!("20 + 12 = {}", n),
    //     Err(e) => println!("{}", e),
    // }

    // match lua.do_string::<bool>("local m2 = math.new(); m2:sum(10, 2); return true") {
    //     Ok(_) => println!("deu certo"),
    //     Err(e) => println!("{}", e),
    // }

    {
        let stack = lua.stack();
        println!("{}", stack);
    }

    // let math = stack.cast_to::<Math>(-1).unwrap();
    // println!("{}", math.sum(7, 1));

    // let ref_value = stack.cast_to::<&str>(-1).unwrap();
    // println!("{}", ref_value.value());

    // stack.clear();
    // println!("{}", stack);
}
