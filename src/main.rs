use ljr::{UserData, lua::Lua, stack_ref::LuaRef};
use luajit2_sys as sys;
use macros::user_data;

struct Person(String);

#[user_data]
impl Person {
    fn get_name(&self) -> String {
        self.0.clone()
    }

    fn external_ref(&self) {}

    fn greet(&self, other: &Person) {
        println!("hello my friend {}, i'm {}", other.0, self.0);
    }

    fn change_name(&self, other: &mut Person, new_name: String) {
        println!("change name");
        other.0 = new_name;
    }
}

struct PersonFactory;

#[user_data]
impl PersonFactory {
    fn new(name: String) -> Person {
        println!("criou nome {}", name);
        Person(name)
    }
}

fn main() {
    let mut lua = Lua::new();
    lua.open_libs();

    // lua.register("math", Math::new());
    lua.register("person", PersonFactory);

    match lua.do_string::<bool>(
        r#"
        local Person = require 'person'

        local paulo = Person.new('Paulo')
        print(paulo:get_name())
        
        local soreto = Person.new('Soreto')
        paulo:greet(soreto)
        print(soreto:get_name())

        soreto:change_name(paulo, "Soretinho")
        print(paulo:get_name())

        return true
        "#,
    ) {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e),
    }

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
