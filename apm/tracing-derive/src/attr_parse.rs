use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_str, Expr, Lit, Meta, NestedMeta};

use std::collections::HashMap;

static KIND: &str = "kind";
static TRACING_NAME: &str = "name";
static TRACING_TAGS: &str = "tags";
static TRACING_LOGS: &str = "logs";

#[derive(Default)]
pub struct TracingAttrs {
    pub kind: String,
    pub tracing_name: Option<String>,
    pub tracing_tags: HashMap<String, String>,
    pub tracing_logs: HashMap<String, String>,
}

impl TracingAttrs {
    pub fn _get_tracing_name(&self) -> Option<String> {
        self.tracing_name.clone()
    }

    pub fn get_tag_map(&self) -> HashMap<String, String> {
        let mut res = self.tracing_tags.clone();
        res.insert("kind".to_string(), self.kind.clone());
        res
    }

    pub fn get_log_map(&self) -> HashMap<String, String> {
        self.tracing_logs.clone()
    }

    fn set_kind(&mut self, kind: String) {
        self.kind = kind;
    }

    fn set_tracing_name(&mut self, name: String) {
        self.tracing_name = Some(name);
    }

    fn set_tracing_tags(&mut self, tags: HashMap<String, String>) {
        self.tracing_tags = tags;
    }

    fn set_tracing_logs(&mut self, logs: HashMap<String, String>) {
        self.tracing_logs = logs;
    }
}

pub fn parse_attrs(input: Vec<NestedMeta>) -> TracingAttrs {
    let mut attrs = TracingAttrs::default();
    for attr in input.iter() {
        match_attr(&mut attrs, attr);
    }

    attrs
}

pub fn span_log(key: String, val: String) -> TokenStream {
    if let Ok(expr) = parse_str::<Expr>(&val) {
        quote! { span_logs.push(LogField::new(#key, (#expr).to_string())); }
    } else {
        quote! { span_logs.push(LogField::new(#key, #val)); }
    }
}

pub fn span_tag(key: String, val: String) -> TokenStream {
    if key == KIND {
        return quote! { span_tags.push((#key.as_str(), #val)); };
    }

    if let Ok(expr) = parse_str::<Expr>(&val) {
        quote! { span_tags.push(Tag::new(#key.as_str(), (#expr).to_string())); }
    } else {
        quote! { span_tags.push(Tag::new(#key.as_str(), #val)); }
    }
}

fn match_attr(tracing_attrs: &mut TracingAttrs, input: &NestedMeta) {
    match input {
        NestedMeta::Meta(data) => match data {
            Meta::NameValue(name_value) => {
                let ident = &name_value
                    .path
                    .segments
                    .first()
                    .expect("there must be at least 1 segment")
                    .ident;

                if ident == KIND {
                    tracing_attrs.set_kind(get_lit_str(&name_value.lit));
                } else if ident == TRACING_NAME {
                    tracing_attrs.set_tracing_name(get_lit_str(&name_value.lit));
                } else if ident == TRACING_TAGS {
                    tracing_attrs.set_tracing_tags(parse_json(&get_lit_str(&name_value.lit)));
                } else if ident == TRACING_LOGS {
                    tracing_attrs.set_tracing_logs(parse_json(&get_lit_str(&name_value.lit)));
                } else {
                    panic!("");
                }
            }
            _ => unreachable!("name_value"),
        },
        _ => unreachable!("meta"),
    };
}

fn get_lit_str(lit: &Lit) -> String {
    match lit {
        Lit::Str(value) => value.value(),
        _ => unreachable!("lit_str"),
    }
}

fn parse_json(input: &str) -> HashMap<String, String> {
    serde_json::from_str::<HashMap<String, String>>(&transfer_string(input))
        .expect("deserialize json error")
}

fn transfer_string(input: &str) -> String {
    input.replace('\'', "\"")
}

#[cfg(test)]
mod test {
    use super::transfer_string;

    #[test]
    fn test_transfer_string() {
        assert_eq!(
            transfer_string("{'a': 'b', 'c': 'd'}"),
            "{\"a\": \"b\", \"c\": \"d\"}",
        );
    }
}
