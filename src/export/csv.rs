use std::path::Path;

pub fn rows_to_csv(columns: &[String], rows: &[Vec<String>], selected: &[usize]) -> String {
    fn escape_csv(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    let rows_to_use: Vec<&Vec<String>> = if selected.is_empty() {
        rows.iter().collect()
    } else {
        selected.iter().map(|&i| &rows[i]).collect()
    };

    let mut out = String::new();
    out.push_str(
        &columns
            .iter()
            .map(|c| escape_csv(c))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push('\n');

    for row in rows_to_use {
        out.push_str(
            &row.iter()
                .map(|v| escape_csv(v))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }

    out
}

pub fn save_to_file(text: &str, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| e.to_string())
}
