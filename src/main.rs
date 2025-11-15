use ljr::{UserData, lua::Lua, table::Table};
use luajit2_sys as sys;
use macros::user_data;

// struct Person {
//     name: String,
//     other: Option<LuaRef<Person>>,
// }

// #[user_data]
// impl Person {
//     fn get_name(&self) -> String {
//         self.name.clone()
//     }

//     fn get_name2(me: LuaRef<Person>) -> String {
//         me.as_ref().name.clone()
//     }

//     fn get_other_name(&self) -> String {
//         self.other
//             .as_ref()
//             .map(|o| o.as_ref().name.clone())
//             .unwrap_or_default()
//     }

//     fn external_ref(&mut self, other: LuaRef<Person>) {
//         self.other = Some(other.clone());
//     }

//     fn greet(&self, other: &Person) {
//         println!("hello my friend {}, i'm {}", other.name, self.name);
//     }

//     fn change_name(&self, other: &mut Person, new_name: String) {
//         println!("change name");
//         other.name = new_name;
//     }
// }

// struct PersonFactory;

// #[user_data]
// impl PersonFactory {
//     fn new(name: String) -> Person {
//         println!("criou nome {}", name);
//         Person { name, other: None }
//     }
// }

struct Test;
// t.push(10i32);
// t.push(false);
// t.push(12i32);

// t.ipairs::<i32>()
//     .for_each(|v| println!("ipairs value: {:?}", v));

#[user_data]
impl Test {
    fn create_table(lua: &Lua) -> Table {
        let table = lua.create_table();
        table.with(|t| {
            t.push(213i32);
            t.push(10923i32);
            t.push("hello world");
        });
        table
    }

    fn test_with_str(first: &str) {
        println!("first: {}", first);
    }

    fn get_from_table(table: Table) {
        table.with(|t| {
            t.pairs::<String, String>()
                .for_each(|(k, v)| println!("{}: {}", k, v));
        });
    }
}

fn main() {
    let mut lua = Lua::new();
    lua.open_libs();

    // lua.register("math", Math::new());
    // lua.register("person", PersonFactory);
    lua.register("test", Test);

    lua.do_string::<bool>(
        r#"
        local test = require "test"
        local t = test.create_table()
        for i, v in ipairs(t) do print(i, v) end
        
        --local value = { [false] = "sumba", ["hello"] = "world" , [12] = "hello", sorvete = "vanilla" }
        --test.get_from_table(value)
        --print(value[1])
        
        --test.test_with_str(value.hello)

        return true
        "#,
    )
    .unwrap();

    // match lua.do_string::<bool>(
    //     r#"
    //     local Person = require 'person'

    //     local paulo = Person.new('Paulo')
    //     print(paulo:get_name())

    //     local soreto = Person.new('Soreto')
    //     paulo:greet(soreto)
    //     print(soreto:get_name())

    //     soreto:change_name(paulo, "Soretinho")
    //     print(paulo:get_name())

    //     print('-------------')
    //     soreto:external_ref(Person.new('Sorvete'))
    //     collectgarbage("collect")
    //     print(soreto:get_other_name())
    //     print(soreto:get_name2())

    //     return true
    //     "#,
    // ) {
    //     Ok(_) => {}
    //     Err(e) => eprintln!("{}", e),
    // }

    {
        let stack = lua.stack();
        println!("{}", stack);
    }
}
