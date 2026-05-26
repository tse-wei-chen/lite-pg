use std::path::Path;

pub fn rows_to_json(columns: &[String], rows: &[Vec<String>], selected: &[usize]) -> String {
    let rows_to_use: Vec<&Vec<String>> = if selected.is_empty() {
        rows.iter().collect()
    } else {
        selected.iter().map(|&i| &rows[i]).collect()
    };

    let mut out = String::from("[\n");
    for (ri, row) in rows_to_use.iter().enumerate() {
        out.push_str("  {");
        for (ci, col) in columns.iter().enumerate() {
            let val = &row[ci];
            let escaped = serde_json::to_string(val).unwrap_or_else(|_| format!("\"{}\"", val));
            out.push_str(&format!("\"{}\": {}", col, escaped));
            if ci < columns.len() - 1 {
                out.push_str(", ");
            }
        }
        out.push('}');
        if ri < rows_to_use.len() - 1 {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(']');
    out
}

pub fn save_to_file(text: &str, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| e.to_string())
}
