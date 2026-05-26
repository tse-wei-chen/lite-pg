pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub fn quote_literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub fn build_regclass(schema: &str, object: &str) -> String {
    format!("{}::regclass", ident(schema, object))
}

pub fn ident(schema: &str, object: &str) -> String {
    format!("{}.{}", quote_ident(schema), quote_ident(object))
}
