#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub ty: String,
    pub docs: Vec<String>,
}

#[derive(Debug)]
pub struct Variant {
    pub span: proc_macro2::Span,
    pub variant: String,
    pub name: String,
    pub discriminant: Discriminant,
    pub docs: Vec<String>,
}

#[derive(Debug)]
pub enum Discriminant {
    Named(String),
    Number(isize),
}
