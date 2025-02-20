//! The derive macro
//!
//! # Supported attributes:
//! ## structs
//! #### on the type
//! `#[anno(name = "name", exact)]`
//!
//! | attribute | description | required |
//! | --- | --- | --- |
//! |`name` | allows you to rename the type | no |
//! | `exact` | marks the class as an `exact` class | no |
//!
//! #### on struct fields
//! `#[anno(name = "name", lua_type = "type_name")]`
//!
//! | attribute | description | required |
//! | --- | --- | --- |
//! |`name` | allows you to rename the field | no |
//! | `lua_type` | the lua type this type should appear as | yes |
//! | `ignore` | skips this field entirely | no |
//!
//! ## enums
//! #### on the type
//! `#[anno(name = "name", self)]`
//!
//! | attribute | description | required |
//! | --- | --- | --- |
//! | `name` | allows you to rename the type | no |
//! | `self` | should the variant discriminants use this type? | no |
//!
//! #### on variants
//! `#[anno(name = "name")]`
//!
//! | attribute | description | required |
//! | --- | --- | --- |
//! | `name` | allows you to rename the variant | no |
//!
//! ## [`AnnoEnum`]
//! This trait is generated for enums, it gives you the lua_name mapped to the enum variant
//!
//! The function [`AnnoEnum::variants`] is useful for doing similar in mlua:
//! ```rust,ignore
//! use anno_lua::AnnoEnum as _;
//!
//! lua.register_userdata_type::<MyEnum>(|registry| {
//!     for (kind, this) in MyEnum::variants() {
//!         registry.add_field_function_get(kind, move |_lua, _| Ok(*this));
//!     }
//! })?;
//! ```
//!
//! # Notes about enums
//! - Currently only unit variants are supported.
//! - Without `self` the variants start to count from 0
//!
//! ---
//!
//! You can manually number the variants
//! If you do number them and let rust pick the others -- you can end up with duplicates.
//! This is intended, lua "enums" aren't algrebiac data types (e.g. sum types), so aliasing is potentially desired
//!
//! # Examples
//!
//! for structs:
//! ```rust,ignore
//! /// Counts stuff from the user
//! #[derive(Anno)]
//! #[anno(name = "Foobar", exact)]
//! struct Foo {
//!     /// The foo count
//!     #[anno(lua_type = "integer")]
//!     count: i32,
//!
//!     #[anno(ignore)]
//!     something: (),
//!
//!     /// A user name
//!     ///
//!     /// This can be optional
//!     #[anno(lua_type = "string?", name = "user_name")]
//!     optional: Option<String>
//! }
//! ```
//!
//! for enums:
//! ```rust,ignore
//! /// Some cardinal directions
//! #[derive(Anno)]
//! #[anno(name = "Dir", self)]
//! enum Direction {
//!     Up,
//!     Down,
//!
//!     #[anno(name = "right")]
//!     Forward,
//!
//!     #[anno(name = "left")]
//!     Back,
//! }
//! ```
//!
pub use anno_lua_derive::Anno;
pub use anno_lua_impl::{
    generate, generate_class, generate_enum, generate_type, Anno, AnnoEnum, Class, Discriminant,
    Enum, Field, Type, Variant,
};
