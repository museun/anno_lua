use std::io::Write;

/// Exposes a lua-generated type definition for this type
pub trait Anno {
    /// Get a static definition of this type
    fn lua_type() -> Type;
}

/// Variant mapping of the lua named variants to the enum type
pub trait AnnoEnum: Sized + 'static {
    /// Get the variant mappings
    fn variants() -> &'static [(&'static str, Self)];

    /// Get the variant name
    fn variant_name(&self) -> &'static str;
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Type {
    Class(Class),
    Enum(Enum),
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Class {
    pub exact: bool,
    pub docs: &'static [&'static str],
    pub name: &'static str,
    pub fields: &'static [Field],
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Enum {
    pub docs: &'static [&'static str],
    pub name: &'static str,
    pub variants: &'static [Variant],
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Field {
    pub name: &'static str,
    pub ty: &'static str,
    pub docs: &'static [&'static str],
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Variant {
    pub name: &'static str,
    pub discriminant: Discriminant,
    pub docs: &'static [&'static str],
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Discriminant {
    Number(isize),
    Named(&'static str),
}

/// Generate [LuaLS](https://github.com/LuaLS/lua-language-server) compatible annotations for this [`type`](Anno)
///
/// This'll append to the writer passed into it
pub fn generate<T>(out: &mut impl Write) -> std::io::Result<()>
where
    T: Anno,
{
    generate_type(out, &T::lua_type())
}

/// Generate a specific type
///
/// This'll append to the writer passed into it
pub fn generate_type(out: &mut impl Write, ty: &Type) -> std::io::Result<()> {
    match ty {
        Type::Class(class) => generate_class(out, &class),
        Type::Enum(enum_) => generate_enum(out, &enum_),
    }
}

/// Generate a specific class
///
/// This'll append to the writer passed into it
pub fn generate_class(out: &mut impl Write, class: &Class) -> std::io::Result<()> {
    for doc in class.docs {
        writeln!(out, "--- {doc}", doc = doc.trim_start())?;
    }
    write!(out, "---@class ")?;
    if class.exact {
        write!(out, "(exact) ")?;
    }
    writeln!(out, "{name}", name = class.name.trim_start())?;

    for field in class.fields {
        for doc in field.docs {
            writeln!(out, "--- {doc}", doc = doc.trim_start())?;
        }
        writeln!(
            out,
            "---@field {name} {ty}",
            name = field.name.trim_start(),
            ty = field.ty.trim_start()
        )?;
    }

    writeln!(out, "{name} = {{ }}", name = class.name.trim_start())?;
    writeln!(out)
}

/// Generate a specific enum
///
/// This'll append to the writer passed into it
pub fn generate_enum(out: &mut impl Write, enum_: &Enum) -> std::io::Result<()> {
    for doc in enum_.docs {
        writeln!(out, "--- {doc}", doc = doc.trim_start())?;
    }

    writeln!(out, "---@enum {name}", name = enum_.name.trim_start())?;
    writeln!(out, "{name} = {{", name = enum_.name.trim_start())?;
    for variant in enum_.variants {
        for doc in variant.docs {
            writeln!(out, "    --- {doc}", doc = doc.trim_start())?;
        }
        write!(out, "    {name} = ", name = variant.name.trim_start())?;
        match variant.discriminant {
            Discriminant::Number(n) => writeln!(out, "{n},")?,
            Discriminant::Named(n) => writeln!(out, "{n},")?,
        }
    }
    writeln!(out, "}}")?;
    writeln!(out)
}
