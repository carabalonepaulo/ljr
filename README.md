# ljr: A Lightweight LuaJIT Binding Layer for Rust

## Roadmap

- `#[func]` and `#[func(name)]` attributes to select which functions to export,
  with support for renaming.
- A `Weak` mode for ref values (similar to `Borrowed` and `Owned`).
- An mdBook detailing architectural decisions, internal mechanics, and safety
  guarantees.

## Philosophy & Governance

### Development Model

ljr follows a "cathedral" development model. To ensure strict adherence to
memory safety guarantees and architectural consistency, I do not accept external
code contributions or Pull Requests. This approach allows the library to remain
focused, cohesive, and free from the complexity overhead of community
management.

### Continuity

The project is licensed under the MIT License. This ensures that you, the user,
always have the legal right and technical ability to fork, modify, and maintain
the library should the need arise. The codebase is designed to be idiomatic and
self-explanatory to facilitate this freedom.

### Support & Customization

This library is provided "as is".

- **Donations**: I do not accept donations. I prefer to keep this project free
  from the implied obligations of sponsorship.
- **Consulting**: If your company requires dedicated support, custom features,
  or architectural integration, I am available for B2B consulting contracts.
