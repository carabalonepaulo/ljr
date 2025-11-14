use ljr::{UserData, lua::Lua, lua_ref::LuaRef};
use luajit2_sys as sys;
use macros::user_data;

struct Person {
    name: String,
    other: Option<LuaRef<Person>>,
}

#[user_data]
impl Person {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_other_name(&self) -> String {
        self.other
            .as_ref()
            .map(|o| o.as_ref().name.clone())
            .unwrap_or_default()
    }

    fn external_ref(&mut self, other: LuaRef<Person>) {
        self.other = Some(other.to_owned());
    }

    fn greet(&self, other: &Person) {
        println!("hello my friend {}, i'm {}", other.name, self.name);
    }

    fn change_name(&self, other: &mut Person, new_name: String) {
        println!("change name");
        other.name = new_name;
    }
}

struct PersonFactory;

#[user_data]
impl PersonFactory {
    fn new(name: String) -> Person {
        println!("criou nome {}", name);
        Person { name, other: None }
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

        print('-------------')
        local sorvete = Person.new('Sorvete')
        soreto:external_ref(sorvete)
        print(soreto:get_other_name())

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
