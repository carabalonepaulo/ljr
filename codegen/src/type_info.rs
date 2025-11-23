#![allow(unused)]

use proc_macro2::{Span, TokenStream, TokenTree};
use syn::Ident;
use venial::TypeExpr;

#[derive(Debug, PartialEq, Clone)]
pub enum Ref {
    Shared,
    Mut,
}

#[derive(Debug)]
pub struct TypeInfo {
    inner_ty: TypeExpr,
    name: String,
    ref_kind: Option<Ref>,
    has_lifetime: bool,
}

impl TypeInfo {
    pub fn new(ty: &TypeExpr) -> Self {
        let (inner_ty, name, ref_kind, has_lifetime) = parse_type_expr(ty);
        Self {
            inner_ty,
            name,
            ref_kind,
            has_lifetime,
        }
    }

    pub fn inner_ty(&self) -> &TypeExpr {
        &self.inner_ty
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ref_kind(&self) -> Option<Ref> {
        self.ref_kind.clone()
    }

    pub fn has_lifetime(&self) -> bool {
        false
    }
}

pub fn parse_type_expr(ty: &TypeExpr) -> (TypeExpr, String, Option<Ref>, bool) {
    let mut iter = ty.tokens.iter().peekable();

    let mut ref_type = None;
    let mut has_lifetime = false;

    if let Some(TokenTree::Punct(p)) = iter.peek() {
        if p.as_char() == '&' {
            iter.next();
            ref_type = Some(Ref::Shared);

            if let Some(TokenTree::Punct(p)) = iter.peek() {
                if p.as_char() == '\'' {
                    has_lifetime = true;
                    iter.next();
                    if let Some(TokenTree::Ident(_)) = iter.peek() {
                        iter.next();
                    }
                }
            }

            if let Some(TokenTree::Ident(id)) = iter.peek() {
                if id.to_string() == "mut" {
                    ref_type = Some(Ref::Mut);
                    iter.next();
                }
            }
        }
    }

    let mut last_ident_token: Option<Ident> = None;

    while let Some(token) = iter.peek() {
        match token {
            TokenTree::Ident(ident) => {
                last_ident_token = Some(ident.clone());
                iter.next();
            }
            TokenTree::Punct(p) if p.as_char() == ':' => {
                iter.next();
                if let Some(TokenTree::Punct(p2)) = iter.peek() {
                    if p2.as_char() == ':' {
                        iter.next();
                    }
                }
            }
            _ => break,
        }
    }

    let final_ident = last_ident_token.unwrap_or_else(|| Ident::new("Unknown", Span::call_site()));
    let final_name_str = final_ident.to_string();

    let mut generics_tokens = Vec::new();

    while let Some(t) = iter.next() {
        generics_tokens.push(t.clone());
    }

    let generics_stream = TokenStream::from_iter(generics_tokens.iter().cloned());
    let generics_str = generics_stream.to_string().replace(" ", "");
    let full_name = format!("{}{}", final_name_str, generics_str);

    let mut clean_tokens = vec![TokenTree::Ident(final_ident)];
    clean_tokens.extend(generics_tokens);

    let inner_type_expr = TypeExpr {
        tokens: clean_tokens,
    };

    (inner_type_expr, full_name, ref_type, has_lifetime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn to_expr(tks: TokenStream) -> TypeExpr {
        TypeExpr {
            tokens: tks.into_iter().collect(),
        }
    }

    fn assert_tokens_eq(ty: &TypeExpr, expected: &str) {
        let ts: TokenStream = ty.tokens.iter().cloned().collect();
        assert_eq!(ts.to_string().replace(" ", ""), expected);
    }

    #[test]
    fn test_simple() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(StackFn)));
        assert_eq!(name, "StackFn");
        assert_eq!(rf, None);
        assert_eq!(lt, false);
    }

    #[test]
    fn test_generics() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(Table<Borrowed>)));
        assert_eq!(name, "Table<Borrowed>");
        assert_eq!(rf, None);
        assert_eq!(lt, false);
    }

    #[test]
    fn test_ref_shared() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&Table<Borrowed>)));
        assert_tokens_eq(&inner_ty, "Table<Borrowed>");
        assert_eq!(name, "Table<Borrowed>");
        assert_eq!(rf, Some(Ref::Shared));
        assert_eq!(lt, false);
    }

    #[test]
    fn test_ref_mut() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&mut TableRef)));
        assert_tokens_eq(&inner_ty, "TableRef");
        assert_eq!(name, "TableRef");
        assert_eq!(rf, Some(Ref::Mut));
        assert_eq!(lt, false);
    }

    #[test]
    fn test_lifetime_shared() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&'a MyType)));
        assert_tokens_eq(&inner_ty, "MyType");
        assert_eq!(name, "MyType");
        assert_eq!(rf, Some(Ref::Shared));
        assert_eq!(lt, true);
    }

    #[test]
    fn test_lifetime_mut() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&'ctx mut ljr::Table<T>)));
        assert_tokens_eq(&inner_ty, "Table<T>");
        assert_eq!(name, "Table<T>");
        assert_eq!(rf, Some(Ref::Mut));
        assert_eq!(lt, true);
    }

    #[test]
    fn test_is_ref() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&StackStr)));
        assert_tokens_eq(&inner_ty, "StackStr");
        assert_eq!(name, "StackStr");
        assert!(rf.is_some());
    }
}
