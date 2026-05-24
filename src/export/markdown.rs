pub fn rows_to_markdown(columns: &[String], rows: &[Vec<String>], selected: &[usize]) -> String {
    let rows_to_use: Vec<&Vec<String>> = if selected.is_empty() {
        rows.iter().collect()
    } else {
        selected.iter().map(|&i| &rows[i]).collect()
    };

    let header = format!("| {} |", columns.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(" | "));
    let sep = format!("| {} |", columns.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));

    let mut lines = vec![header, sep];
    for row in rows_to_use {
        let line = format!(
            "| {} |",
            row.iter()
                .map(|v| if v.contains('\n') {
                    v.lines().collect::<Vec<_>>().join("<br>")
                } else {
                    v.clone()
                })
                .collect::<Vec<_>>()
                .join(" | ")
        );
        lines.push(line);
    }

    lines.join("\n")
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut ctx = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    ctx.set_text(text).map_err(|e| e.to_string())
}
