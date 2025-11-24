#![allow(unused)]

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
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
    pub fn new(ty: &TypeExpr) -> Option<Self> {
        let (inner_ty, name, ref_kind, has_lifetime) = parse_type_expr(ty)?;
        Some(Self {
            inner_ty,
            name,
            ref_kind,
            has_lifetime,
        })
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
        self.has_lifetime
    }
}

fn clean_name_from_stream(stream: TokenStream) -> String {
    let mut output = String::new();
    let mut iter = stream.into_iter().peekable();
    let mut pending_ident: Option<String> = None;

    while let Some(token) = iter.next() {
        match token {
            TokenTree::Ident(ident) => {
                if let Some(prev) = pending_ident.take() {
                    output.push_str(&prev);
                }
                pending_ident = Some(ident.to_string());
            }
            TokenTree::Punct(p) if p.as_char() == ':' => {
                if let Some(TokenTree::Punct(p2)) = iter.peek() {
                    if p2.as_char() == ':' {
                        iter.next();
                        pending_ident = None;
                        continue;
                    }
                }
                if let Some(prev) = pending_ident.take() {
                    output.push_str(&prev);
                }
                output.push(':');
            }
            TokenTree::Punct(p) => {
                if let Some(prev) = pending_ident.take() {
                    output.push_str(&prev);
                }
                output.push(p.as_char());
            }
            TokenTree::Group(g) => {
                if let Some(prev) = pending_ident.take() {
                    output.push_str(&prev);
                }

                let content = clean_name_from_stream(g.stream());
                match g.delimiter() {
                    Delimiter::Parenthesis => output.push_str(&format!("({})", content)),
                    Delimiter::Bracket => output.push_str(&format!("[{}]", content)),
                    Delimiter::Brace => output.push_str(&format!("{{{}}}", content)),
                    Delimiter::None => output.push_str(&content),
                }
            }
            TokenTree::Literal(l) => {
                if let Some(prev) = pending_ident.take() {
                    output.push_str(&prev);
                }
                output.push_str(&l.to_string());
            }
        }
    }
    if let Some(last) = pending_ident {
        output.push_str(&last);
    }
    output
}

pub fn parse_type_expr(ty: &TypeExpr) -> Option<(TypeExpr, String, Option<Ref>, bool)> {
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

    if let Some(TokenTree::Punct(p)) = iter.peek() {
        if p.as_char() == '&' {
            return None;
        }
    }

    if let Some(TokenTree::Group(g)) = iter.peek() {
        let group_token = g.clone();

        let inner_name = clean_name_from_stream(group_token.stream());

        let name = match group_token.delimiter() {
            Delimiter::Parenthesis => format!("({})", inner_name),
            Delimiter::Bracket => format!("[{}]", inner_name),
            Delimiter::Brace => format!("{{{}}}", inner_name),
            Delimiter::None => inner_name,
        };

        iter.next();
        let inner_type_expr = TypeExpr {
            tokens: vec![TokenTree::Group(group_token)],
        };
        return Some((inner_type_expr, name, ref_type, has_lifetime));
    }

    let mut path_tokens: Vec<TokenTree> = Vec::new();
    let mut last_ident_token: Option<Ident> = None;

    while let Some(token) = iter.peek() {
        match token {
            TokenTree::Ident(ident) => {
                last_ident_token = Some(ident.clone());
                path_tokens.push((*token).clone());
                iter.next();
            }
            TokenTree::Punct(p) if p.as_char() == ':' => {
                path_tokens.push((*token).clone());
                iter.next();
                if let Some(TokenTree::Punct(p2)) = iter.peek() {
                    if p2.as_char() == ':' {
                        path_tokens.push(TokenTree::Punct(p2.clone()));
                        iter.next();
                    }
                }
            }
            _ => break,
        }
    }

    let final_ident = last_ident_token?;
    let final_name_str = final_ident.to_string();

    let mut generics_tokens = Vec::new();
    while let Some(t) = iter.next() {
        generics_tokens.push(t.clone());
    }

    let generics_stream = TokenStream::from_iter(generics_tokens.iter().cloned());
    let generics_str = clean_name_from_stream(generics_stream);

    let full_name = format!("{}{}", final_name_str, generics_str);

    let mut clean_tokens = path_tokens;
    clean_tokens.extend(generics_tokens);

    let inner_type_expr = TypeExpr {
        tokens: clean_tokens,
    };

    Some((inner_type_expr, full_name, ref_type, has_lifetime))
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
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(StackFn))).unwrap();
        assert_eq!(name, "StackFn");
        assert_eq!(rf, None);
        assert_eq!(lt, false);
    }

    #[test]
    fn test_generics() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(Table<Borrowed>))).unwrap();
        assert_eq!(name, "Table<Borrowed>");
        assert_eq!(rf, None);
        assert_eq!(lt, false);
    }

    #[test]
    fn test_path_removal_generic() {
        let (_, name, _, _) = parse_type_expr(&to_expr(quote!(Table<std::vec::Vec<T>>))).unwrap();
        assert_eq!(name, "Table<Vec<T>>");
    }

    #[test]
    fn test_ref_shared() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&Table<Borrowed>))).unwrap();
        assert_tokens_eq(&inner_ty, "Table<Borrowed>");
        assert_eq!(name, "Table<Borrowed>");
        assert_eq!(rf, Some(Ref::Shared));
        assert_eq!(lt, false);
    }

    #[test]
    fn test_ref_mut() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&mut TableRef))).unwrap();
        assert_tokens_eq(&inner_ty, "TableRef");
        assert_eq!(name, "TableRef");
        assert_eq!(rf, Some(Ref::Mut));
        assert_eq!(lt, false);
    }

    #[test]
    fn test_lifetime_shared() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&'a MyType))).unwrap();
        assert_tokens_eq(&inner_ty, "MyType");
        assert_eq!(name, "MyType");
        assert_eq!(rf, Some(Ref::Shared));
        assert_eq!(lt, true);
    }

    #[test]
    fn test_lifetime_mut() {
        let (inner_ty, name, rf, lt) =
            parse_type_expr(&to_expr(quote!(&'ctx mut ljr::Table<T>))).unwrap();
        assert_tokens_eq(&inner_ty, "ljr::Table<T>");
        assert_eq!(name, "Table<T>");
        assert_eq!(rf, Some(Ref::Mut));
        assert_eq!(lt, true);
    }

    #[test]
    fn test_is_ref() {
        let (inner_ty, name, rf, lt) = parse_type_expr(&to_expr(quote!(&StackStr))).unwrap();
        assert_tokens_eq(&inner_ty, "StackStr");
        assert_eq!(name, "StackStr");
        assert!(rf.is_some());
    }

    #[test]
    fn test_is_double_ref() {
        let result = parse_type_expr(&to_expr(quote!(&&StackStr)));
        assert!(result.is_none());
    }

    #[test]
    fn test_nested_mut_ref() {
        let result = parse_type_expr(&to_expr(quote!(&mut &StackStr)));
        assert!(result.is_none());
    }

    #[test]
    fn test_double_ref_with_path() {
        let result = parse_type_expr(&to_expr(quote!(&&ljr::lstr::StackStr)));
        assert!(result.is_none());
    }

    #[test]
    fn test_double_ref_with_lifetime() {
        let result = parse_type_expr(&to_expr(quote!(&'a &StackStr)));
        assert!(result.is_none());

        let result = parse_type_expr(&to_expr(quote!(&&'a StackStr)));
        assert!(result.is_none());
    }

    #[test]
    fn test_tuple() {
        let result = parse_type_expr(&to_expr(quote!((i32, i32))));
        assert!(result.is_some());

        let result = parse_type_expr(&to_expr(quote!((String, bool, &StackFn))));
        assert!(result.is_some());

        let (ty, name, rf, lt) = result.unwrap();
        assert_eq!(name, "(String,bool,&StackFn)");
        assert_eq!(rf, None);
        assert_eq!(lt, false);
    }

    #[test]
    fn test_complex_tuple() {
        let result = parse_type_expr(&to_expr(quote!((String, bool, &ljr::StackFn))));
        assert!(result.is_some());

        let (ty, name, rf, lt) = result.unwrap();
        assert_eq!(name, "(String,bool,&StackFn)");
        assert_eq!(rf, None);
        assert_eq!(lt, false);
    }
}
