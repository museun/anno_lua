pub enum Error {
    Syn(syn::Error),
    Union(proc_macro2::Span),
    UnnamedField(proc_macro2::Span),
    TyRequire(proc_macro2::Span),
    OnlyName(proc_macro2::Span),
    SelfDiscriminant(proc_macro2::Span),
    ExpectedNumber(proc_macro2::Span),
    OnlyUnitVariants(proc_macro2::Span),
    DuplicateName(proc_macro2::Span),
    EmptyName(proc_macro2::Span),
}

impl From<syn::Error> for Error {
    fn from(value: syn::Error) -> Self {
        Self::Syn(value)
    }
}

impl Error {
    pub fn into_syn_error(self) -> syn::Error {
        let (span, msg) = match self {
            Self::Syn(syn) => return syn,
            Self::Union(span) => (span, "unions are not supported"),
            Self::UnnamedField(span) => (span, "unnamed fields are not allowed"),
            Self::TyRequire(span) => (span, "lua_type = \"type\" is required"),
            Self::OnlyName(span) => (span, "only name = \"name\" is allowed here"),
            Self::SelfDiscriminant(span) => (
                span,
                "a discriminant was provided when `self` was requested",
            ),
            Self::ExpectedNumber(span) => (span, "expected a number here"),
            Self::OnlyUnitVariants(span) => (span, "only unit variants are allowed"),
            Self::DuplicateName(span) => (span, "duplicate name provided"),
            Self::EmptyName(span) => (span, "name cannot be empty"),
        };
        syn::Error::new(span, msg)
    }

    pub fn into_compile_error(self) -> proc_macro::TokenStream {
        self.into_syn_error().into_compile_error().into()
    }
}
