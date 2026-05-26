use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{ColumnForm, PG_TYPES, PropSection, PropertyEditor};
use crate::db::schema::{ConstraintInfo, IndexInfo, TriggerInfo};
use crate::db::{DbObject, DbObjectType};

const LOADING: &str = " (loading...)";

pub fn render(f: &mut Frame, area: Rect, editor: &PropertyEditor, obj: &DbObject) {
    f.render_widget(Clear, area);

    let obj_type = if obj.obj_type == DbObjectType::View { "View" } else { "Table" };
    let title = format!(" {} Properties  ({}.{}) ", obj_type, obj.schema_name, obj.name);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Section tabs
    let tabs = ["General", "Columns", "Indexes", "Constraints", "Triggers"];
    let tab_width = inner.width as usize / tabs.len();
    let mut tab_spans = Vec::new();
    for (i, tab) in tabs.iter().enumerate() {
        let is_active = section_index(&editor.section) == i;
        tab_spans.push(Span::styled(
            format!(
                "{:^width$}",
                tab,
                width = tab_width.min(15),
            ),
            if is_active {
                Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        ));
        tab_spans.push(Span::raw(" "));
    }
    let tab_line = Line::from(tab_spans);

    // Render section tabs
    let tab_render_area = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(ratatui::widgets::Paragraph::new(tab_line), tab_render_area);

    // Render section content
    let section_content_y = inner.y + 2;
    let section_height = inner.height.saturating_sub(4);
    let section_area = Rect {
        x: inner.x + 1,
        y: section_content_y,
        width: inner.width.saturating_sub(2),
        height: section_height,
    };

    let items: Vec<ListItem> = match editor.section {
        PropSection::General => render_general(obj, editor.selected),
        PropSection::Columns => render_columns(obj, editor.selected),
        PropSection::Indexes => render_indexes(obj, editor.selected),
        PropSection::Constraints => render_constraints(obj, editor.selected),
        PropSection::Triggers => render_triggers(obj, editor.selected),
    };

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(editor.selected));
    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, section_area, &mut list_state);

    // Action bar
    let action_y = inner.y + inner.height.saturating_sub(1);
    let action_area = Rect {
        x: inner.x + 1,
        y: action_y,
        width: inner.width.saturating_sub(2),
        height: 1,
    };
    let mut action_spans = vec![
        Span::styled(" [d] Drop ", Style::default().fg(Color::Red)),
        Span::styled(" [Tab/h/l] Section ", Style::default().fg(Color::Gray)),
        Span::styled(" [j/k] Move ", Style::default().fg(Color::Gray)),
    ];
    if editor.section == PropSection::Columns {
        action_spans.push(Span::styled(" [a] Add ", Style::default().fg(Color::Green)));
        action_spans.push(Span::styled(" [e] Edit ", Style::default().fg(Color::Green)));
    }
    action_spans.push(Span::styled(" [Esc] Back ", Style::default().fg(Color::Gray)));
    let action_text = Line::from(action_spans);
    f.render_widget(Paragraph::new(action_text), action_area);

    // Column form popup
    if let Some(ref form) = editor.column_form {
        render_column_form(f, inner, form);
    }
}

fn section_index(section: &PropSection) -> usize {
    match section {
        PropSection::General => 0,
        PropSection::Columns => 1,
        PropSection::Indexes => 2,
        PropSection::Constraints => 3,
        PropSection::Triggers => 4,
    }
}

fn render_general(obj: &DbObject, _selected: usize) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Name:       {}", obj.name),
        Style::default().fg(Color::White),
    ))));
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Schema:     {}", obj.schema_name),
        Style::default().fg(Color::White),
    ))));
    if let Some(ref owner) = obj.owner {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Owner:      {}", owner),
            Style::default().fg(Color::White),
        ))));
    }
    if let Some(ref desc) = obj.description {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Comment:    {}", desc),
            Style::default().fg(Color::White),
        ))));
    }
    if let Some(cnt) = obj.row_count {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Rows:       ~{}", cnt),
            Style::default().fg(Color::White),
        ))));
    }
    if let Some(sz) = obj.size_bytes {
        let human = if sz > 1_073_741_824 {
            format!("{:.1} GB", sz as f64 / 1_073_741_824.0)
        } else if sz > 1_048_576 {
            format!("{:.1} MB", sz as f64 / 1_048_576.0)
        } else if sz > 1024 {
            format!("{:.1} KB", sz as f64 / 1024.0)
        } else {
            format!("{} B", sz)
        };
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" Size:       {}", human),
            Style::default().fg(Color::White),
        ))));
    }
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Type:       {}", obj.obj_type.icon()),
        Style::default().fg(Color::White),
    ))));
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Columns:    {}", obj.columns.len()),
        Style::default().fg(Color::White),
    ))));
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Indexes:    {}", obj.indexes.len()),
        Style::default().fg(Color::White),
    ))));
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Constraints: {}", obj.constraints.len()),
        Style::default().fg(Color::White),
    ))));
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" Triggers:   {}", obj.triggers.len()),
        Style::default().fg(Color::White),
    ))));
    items
}

fn render_columns(obj: &DbObject, _selected: usize) -> Vec<ListItem<'static>> {
    if !obj.detail_loaded {
        return vec![ListItem::new(Line::from(Span::styled(LOADING, Style::default().fg(Color::Gray))))];
    }
    if obj.columns.is_empty() {
        return vec![ListItem::new(Line::from(Span::styled(
            " (no columns)",
            Style::default().fg(Color::Gray),
        )))];
    }
    obj.columns.iter().map(|col| {
        let pk = if col.is_primary { " PK" } else { "" };
        let nullable = if col.is_nullable { "" } else { " NOT NULL" };
        let def = col.default_value.as_ref().map(|d| format!(" DEFAULT {}", d)).unwrap_or_default();
        ListItem::new(Line::from(Span::styled(
            format!(" {:<20} : {:<15}{}{}{}", col.name, col.data_type, nullable, def, pk),
            Style::default().fg(Color::White),
        )))
    }).collect()
}

fn render_indexes(obj: &DbObject, _selected: usize) -> Vec<ListItem<'static>> {
    if !obj.detail_loaded {
        return vec![ListItem::new(Line::from(Span::styled(LOADING, Style::default().fg(Color::Gray))))];
    }
    if obj.indexes.is_empty() {
        return vec![ListItem::new(Line::from(Span::styled(
            " (no indexes)",
            Style::default().fg(Color::Gray),
        )))];
    }
    obj.indexes.iter().map(|idx: &IndexInfo| {
        let uniq = if idx.is_unique { " UNIQUE" } else { "" };
        let pk = if idx.is_primary { " PRIMARY" } else { "" };
        ListItem::new(Line::from(Span::styled(
            format!(" {:<25} [{}{}{}]", idx.name, idx.index_type, uniq, pk),
            Style::default().fg(Color::White),
        )))
    }).collect()
}

fn render_constraints(obj: &DbObject, _selected: usize) -> Vec<ListItem<'static>> {
    if !obj.detail_loaded {
        return vec![ListItem::new(Line::from(Span::styled(LOADING, Style::default().fg(Color::Gray))))];
    }
    if obj.constraints.is_empty() {
        return vec![ListItem::new(Line::from(Span::styled(
            " (no constraints)",
            Style::default().fg(Color::Gray),
        )))];
    }
    obj.constraints.iter().map(|con: &ConstraintInfo| {
        let ref_info = if con.constraint_type == "FOREIGN KEY" {
            con.referenced_table.as_ref().map(|t| format!(" → {}", t)).unwrap_or_default()
        } else { String::new() };
        ListItem::new(Line::from(Span::styled(
            format!(" {:<25} ({}){}", con.name, con.constraint_type, ref_info),
            Style::default().fg(Color::White),
        )))
    }).collect()
}

fn render_triggers(obj: &DbObject, _selected: usize) -> Vec<ListItem<'static>> {
    if !obj.detail_loaded {
        return vec![ListItem::new(Line::from(Span::styled(LOADING, Style::default().fg(Color::Gray))))];
    }
    if obj.triggers.is_empty() {
        return vec![ListItem::new(Line::from(Span::styled(
            " (no triggers)",
            Style::default().fg(Color::Gray),
        )))];
    }
    obj.triggers.iter().map(|trg: &TriggerInfo| {
        ListItem::new(Line::from(Span::styled(
            format!(" {} ({} {} {})", trg.name, trg.timing, trg.event, trg.level),
            Style::default().fg(Color::White),
        )))
    }).collect()
}

fn render_column_form(f: &mut Frame, parent_inner: Rect, form: &ColumnForm) {
    let dropdown_rows = if form.focus == 1 { (form.type_filtered.len().min(6) as u16).max(1) } else { 0 };
    let pw = parent_inner.width.saturating_sub(4).min(60);
    let ph: u16 = 10 + dropdown_rows;
    let popup_x = parent_inner.x + (parent_inner.width.saturating_sub(pw)) / 2;
    let popup_y = parent_inner.y + (parent_inner.height.saturating_sub(ph)) / 2;
    let popup_area = Rect { x: popup_x, y: popup_y, width: pw, height: ph };

    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Column ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let field_labels = ["Name", "Type", "Nullable", "Default"];
    let field_values: [&str; 4] = [
        &form.name,
        &form.data_type,
        if form.is_nullable { "YES" } else { "NO" },
        &form.default_value,
    ];

    for (i, label) in field_labels.iter().enumerate() {
        let y = inner.y + i as u16;
        let is_active = form.focus == i;
        let label_style = if is_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let label_span = Span::styled(format!(" {:<10} ", label), label_style);

        let val_style = if is_active {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let val_span = Span::styled(
            format!(" {}", field_values[i]),
            val_style,
        );

        let line = Line::from(vec![label_span, val_span]);
        let line_area = Rect { x: inner.x, y, width: inner.width, height: 1 };
        f.render_widget(ratatui::widgets::Clear, line_area);
        f.render_widget(Paragraph::new(line), line_area);
    }

    // Type dropdown when focused on Type field
    if form.focus == 1 && !form.type_filtered.is_empty() {
        let dd_y = inner.y + 5;
        let dd_h = dropdown_rows;
        let dd_area = Rect { x: inner.x, y: dd_y, width: inner.width.saturating_sub(1), height: dd_h };
        f.render_widget(Clear, dd_area);

        let visible_start = if form.type_selected >= dd_h as usize { form.type_selected - dd_h as usize + 1 } else { 0 };
        let mut type_items: Vec<ListItem> = form.type_filtered.iter()
            .skip(visible_start)
            .take(dd_h as usize)
            .map(|&idx| {
                ListItem::new(Line::from(Span::styled(PG_TYPES[idx], Style::default().fg(Color::White))))
            })
            .collect();
        if type_items.is_empty() {
            type_items.push(ListItem::new(Line::from(Span::styled("(none)", Style::default().fg(Color::Gray)))));
        }
        let rel_selected = form.type_selected.saturating_sub(visible_start);
        let mut dd_list_state = ratatui::widgets::ListState::default();
        dd_list_state.select(Some(rel_selected));
        let dd_list = List::new(type_items)
            .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::Yellow));
        f.render_stateful_widget(dd_list, dd_area, &mut dd_list_state);

        // Add dropdown hint to the help line (so user knows j/k works)
        let hint_dd = Line::from(vec![
            Span::styled(" [j/k] Pick type  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter] Select", Style::default().fg(Color::DarkGray)),
        ]);
        let hint_dd_area = Rect { x: inner.x, y: dd_y + dd_h, width: inner.width, height: 1 };
        f.render_widget(Paragraph::new(hint_dd), hint_dd_area);
    }

    // Help line
    let extra_rows = if form.focus == 1 && !form.type_filtered.is_empty() { dropdown_rows + 1 } else { 0 };
    let hint_y = inner.y + 4 + extra_rows;
    let hint = Line::from(vec![
        Span::styled(" [Tab/Enter] Next  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Space] Toggle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc] Cancel", Style::default().fg(Color::DarkGray)),
    ]);
    let hint_area = Rect { x: inner.x, y: hint_y, width: inner.width, height: 1 };
    f.render_widget(Paragraph::new(hint), hint_area);
}
