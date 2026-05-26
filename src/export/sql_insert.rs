use std::path::Path;

pub fn rows_to_sql_insert(
    columns: &[String],
    rows: &[Vec<String>],
    selected: &[usize],
    table_name: &str,
) -> String {
    fn escape_sql(s: &str) -> String {
        if s == "NULL" {
            return "NULL".to_string();
        }
        format!("'{}'", s.replace('\'', "''"))
    }

    let rows_to_use: Vec<&Vec<String>> = if selected.is_empty() {
        rows.iter().collect()
    } else {
        selected.iter().map(|&i| &rows[i]).collect()
    };

    let cols = columns
        .iter()
        .map(|c| format!("\"{}\"", c))
        .collect::<Vec<_>>()
        .join(", ");

    let mut out = String::new();
    for row in rows_to_use {
        let vals = row
            .iter()
            .map(|v| escape_sql(v))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "INSERT INTO \"{}\" ({}) VALUES ({});\n",
            table_name, cols, vals
        ));
    }

    out
}

pub fn save_to_file(text: &str, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| e.to_string())
}
