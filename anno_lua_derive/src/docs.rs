use syn::{Attribute, Expr, ExprLit, Lit};

pub fn collect_docs(attrs: &[Attribute]) -> Vec<String> {
    let mut out = vec![];
    for input in attrs {
        let Ok(nv) = input.meta.require_name_value() else {
            continue;
        };
        if !nv.path.is_ident("doc") {
            continue;
        }

        let Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) = &nv.value
        else {
            continue;
        };

        out.push(lit.value().trim().to_string());
    }
    out
}
