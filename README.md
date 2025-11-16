# ljr: A Lightweight LuaJIT Binding Layer for Rust

`ljr` is a minimal, zero-magic binding layer for integrating Rust with LuaJIT
through FFI. It focuses on predictable behavior, explicit conversions, and
Rust-safe handling of memory and borrowing rules.

This crate is not a low-level wrapper. It deliberately hides the Lua C API and
does not require any knowledge of the stack or FFI details. The goal is to
provide a clear, safe, and predictable interface that behaves like normal Rust
code.

## Features

- **Zero-cost user data bindings** via a procedural macro.
- **Safe stack handling** with predictable borrow rules.
- **Explicit conversions** between Lua values and Rust types.
- **Stack references** that enforce Rust borrowing at runtime.
- **Automatic generation of `luaL_Reg` lists**.
- **Support for `&T`, `&mut T`, primitives, strings, and custom types**.

## User Data

Implement methods normally inside an `impl` block, then annotate it with the
macro:

```rust
struct Person {
    name: String
}

#[user_data]
impl Person {
    fn greet(&self) {
        println!("Hello, {}", self.name);
    }

    fn rename(&mut self, new_name: &str) {
        self.name = new_name.to_string();
    }
}

struct PersonFactory;

#[user_data]
impl PersonFactory {
    fn new(name: &str) -> Person {
        Person { name: name.into() }
    }
}
```

This automatically generates the `UserData` implementation and exposes the
functions to Lua.

## Lua Integration Example

```rust
let mut lua = Lua::new();
lua.open_libs();

lua.register("person", PersonFactory);

lua.do_string::<()>(r#"
    local Person = require 'person'
    local p = Person.new("Paulo")
    p:greet()
    p:rename("Alex")
    p:greet()
"#).unwrap();
```

## Borrowing Rules

`ljr` enforces Rust-like borrowing at runtime:

- `&T` functions borrow immutably.
- `&mut T` functions borrow mutably.
- Overlapping borrows result in a Lua error.
- Stack references guarantee drop-based cleanup and safe access.

These rules prevent UB while keeping the API close to idiomatic Rust.

## String Handling

Two string forms are supported:

- `&str` is treated as a temporary borrowed view into the Lua string, valid only
  for the duration of the call and never heap-allocates.
- `String` produces an owned Rust string when the value needs to outlive the
  call.

## Table Handling

`Table` values are always safe to clone and store. A `Table` internally holds an
`Rc<Inner>` that manages a `lua_ref` and calls `lua_unref` on drop.

This means:

- Cloning a `Table` does not duplicate the Lua table.
- All clones point to the same underlying Lua value.
- The value stays alive in Lua as long as at least one `Table` exists in Rust.
- Dropping the last `Table` releases the reference on the Lua side.

You can store `Table` anywhere you want:

- Inside structs
- Inside vectors
- In global state
- Pass across FFI boundaries
- Return to Lua normally

No extra wrapper is required.

Example:

```rust
let t = lua.create_table();

t.with(|t| {
    t.set("a", 10);
    t.set("b", 20);
});

let mut list = Vec::new();
list.push(t.clone());
list.push(t.clone());

list[0].with(|tbl| {
    println!("{}", tbl.get::<i32>("a"));
});
```

Both elements in `list` refer to the same Lua table. The table stays alive until
all clones are dropped.

## Error Handling

Functions automatically run inside a `catch_unwind` wrapper:

- Rust panics become Lua errors.
- Incorrect argument count raises a Lua error.
- Type mismatches produce clear error messages.

## Goals

- Provide predictable and explicit semantics.
- Make the Lua stack safe to work with.
- Avoid hidden behavior.
- Avoid allocations unless necessary.
- Mirror Rust’s ownership model as closely as Lua allows.

## Non-goals

- High-level abstractions.
- Async integration.
- Automatic type reflection.
- “Magic” conversions.
- General-purpose dynamic frameworks.

## Minimum Requirements

- Rust stable
- LuaJIT (via `luajit2-sys`)
- C toolchain
