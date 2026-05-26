use crate::app::{App, SchemaTreeItem};
use crate::db::DbObjectType;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;
use std::collections::HashMap;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &mut App, focus: bool) {
    rebuild_tree(app);

    let items: Vec<ListItem> = app
        .schema_tree
        .flat_items
        .iter()
        .map(|item| match item {
            SchemaTreeItem::SchemaHeader(si) => {
                let schema = &app.schemas[*si];
                let icon = if app.schema_tree.schema_expanded[*si] {
                    "▼"
                } else {
                    "▶"
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {} {}", icon, schema.name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )))
            }
            SchemaTreeItem::ObjectRow(si, oi) => {
                let obj = &app.schemas[*si].objects[*oi];
                let icon = obj.obj_type.icon();
                let expanded = app.schema_tree.is_object_expanded(*si, *oi);
                let expand_marker = if expanded { "▼" } else { "▶" };
                let color = match obj.obj_type {
                    DbObjectType::Table => Color::White,
                    DbObjectType::View => Color::Magenta,
                    DbObjectType::MaterializedView => Color::Magenta,
                    DbObjectType::Sequence => Color::Yellow,
                    DbObjectType::ForeignTable => Color::Blue,
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {} {} {}", expand_marker, icon, obj.name),
                    Style::default().fg(color),
                )))
            }
            SchemaTreeItem::ColumnSection(si, oi) => {
                let obj = &app.schemas[*si].objects[*oi];
                let expanded = app.schema_tree.is_section_expanded(*si, *oi, 0);
                let marker = if expanded { "▼" } else { "▶" };
                ListItem::new(Line::from(Span::styled(
                    format!("   {} Columns ({})", marker, obj.columns.len()),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )))
            }
            SchemaTreeItem::IndexSection(si, oi) => {
                let obj = &app.schemas[*si].objects[*oi];
                let expanded = app.schema_tree.is_section_expanded(*si, *oi, 1);
                let marker = if expanded { "▼" } else { "▶" };
                ListItem::new(Line::from(Span::styled(
                    format!("   {} Indexes ({})", marker, obj.indexes.len()),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )))
            }
            SchemaTreeItem::TriggerSection(si, oi) => {
                let obj = &app.schemas[*si].objects[*oi];
                let expanded = app.schema_tree.is_section_expanded(*si, *oi, 2);
                let marker = if expanded { "▼" } else { "▶" };
                ListItem::new(Line::from(Span::styled(
                    format!("   {} Triggers ({})", marker, obj.triggers.len()),
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                )))
            }
            SchemaTreeItem::ConstraintSection(si, oi) => {
                let obj = &app.schemas[*si].objects[*oi];
                let expanded = app.schema_tree.is_section_expanded(*si, *oi, 3);
                let marker = if expanded { "▼" } else { "▶" };
                ListItem::new(Line::from(Span::styled(
                    format!("   {} Constraints ({})", marker, obj.constraints.len()),
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )))
            }
            SchemaTreeItem::ColumnRow(_si, _oi, ci) => {
                let col = &app.schemas[*_si].objects[*_oi].columns[*ci];
                let nullable = if col.is_nullable { "" } else { " NOT NULL" };
                let pk = if col.is_primary { " PK" } else { "" };
                ListItem::new(Line::from(Span::styled(
                    format!(
                        "     {} : {}{}{}",
                        col.name, col.data_type, nullable, pk
                    ),
                    Style::default().fg(Color::Green),
                )))
            }
            SchemaTreeItem::IndexRow(_si, _oi, ii) => {
                let idx = &app.schemas[*_si].objects[*_oi].indexes[*ii];
                let uniq = if idx.is_unique { " UNIQUE" } else { "" };
                ListItem::new(Line::from(Span::styled(
                    format!("     I  {} [{}]{}", idx.name, idx.index_type, uniq),
                    Style::default().fg(Color::Yellow),
                )))
            }
            SchemaTreeItem::TriggerRow(_si, _oi, ti) => {
                let trg = &app.schemas[*_si].objects[*_oi].triggers[*ti];
                ListItem::new(Line::from(Span::styled(
                    format!(
                        "     R  {} ({} {} {})",
                        trg.name, trg.timing, trg.event, trg.level
                    ),
                    Style::default().fg(Color::Magenta),
                )))
            }
            SchemaTreeItem::ConstraintRow(_si, _oi, ci) => {
                let con = &app.schemas[*_si].objects[*_oi].constraints[*ci];
                let ref_info = if con.constraint_type == "FOREIGN KEY" {
                    con.referenced_table
                        .as_ref()
                        .map(|t| format!(" → {}", t))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                ListItem::new(Line::from(Span::styled(
                    format!(
                        "     C  {} ({}){}",
                        con.name, con.constraint_type, ref_info
                    ),
                    Style::default().fg(Color::Blue),
                )))
            }
            SchemaTreeItem::SequenceSection(si, oi) => {
                let obj = &app.schemas[*si].objects[*oi];
                let seq_count = app.schemas[*si].objects.iter()
                    .filter(|o| o.obj_type == DbObjectType::Sequence)
                    .filter(|o| o.owned_table.as_deref() == Some(&obj.name))
                    .count();
                let expanded = app.schema_tree.is_section_expanded(*si, *oi, 4);
                let marker = if expanded { "▼" } else { "▶" };
                ListItem::new(Line::from(Span::styled(
                    format!("   {} Sequences ({})", marker, seq_count),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )))
            }
            SchemaTreeItem::SequenceRow(_si, oi) => {
                let obj = &app.schemas[*_si].objects[*oi];
                ListItem::new(Line::from(Span::styled(
                    format!("     S  {}", obj.name),
                    Style::default().fg(Color::Yellow),
                )))
            }
        })
        .collect();

    let border_color = if focus { Color::Cyan } else { Color::DarkGray };
    let list = List::new(items)
        .block(Block::default().title(" Schema ").borders(Borders::ALL).border_style(Style::default().fg(border_color)))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, area, &mut app.schema_tree.list_state);
}

fn rebuild_tree(app: &mut App) {
    let old_len = app.schema_tree.flat_items.len();
    app.schema_tree.flat_items.clear();

    while app.schema_tree.schema_expanded.len() < app.schemas.len() {
        app.schema_tree.schema_expanded.push(false);
    }
    while app.schema_tree.object_expanded.len() < app.schemas.len() {
        app.schema_tree.object_expanded.push(Vec::new());
    }
    while app.schema_tree.section_expanded.len() < app.schemas.len() {
        app.schema_tree.section_expanded.push(Vec::new());
    }

    for (si, schema) in app.schemas.iter().enumerate() {
        app.schema_tree
            .flat_items
            .push(SchemaTreeItem::SchemaHeader(si));

        if si < app.schema_tree.schema_expanded.len() && app.schema_tree.schema_expanded[si] {
            while app.schema_tree.object_expanded[si].len() < schema.objects.len() {
                app.schema_tree.object_expanded[si].push(false);
            }
            while app.schema_tree.section_expanded[si].len() < schema.objects.len() {
                app.schema_tree.section_expanded[si].push([true, true, true, true, true]);
            }
            // Build map: table_name -> owned sequence indices
            let mut owned_seqs: HashMap<&str, Vec<usize>> = HashMap::new();
            for (oi, obj) in schema.objects.iter().enumerate() {
                if obj.obj_type == DbObjectType::Sequence {
                    if let Some(ref owned) = obj.owned_table {
                        owned_seqs.entry(owned.as_str()).or_default().push(oi);
                    }
                }
            }

            for (oi, obj) in schema.objects.iter().enumerate() {
                if obj.obj_type == DbObjectType::Sequence && obj.owned_table.is_some() {
                    continue;
                }

                app.schema_tree
                    .flat_items
                    .push(SchemaTreeItem::ObjectRow(si, oi));

                // If object is expanded and has detail loaded, show section headers
                if app.schema_tree.is_object_expanded(si, oi) && !obj.columns.is_empty() {
                    app.schema_tree
                        .flat_items
                        .push(SchemaTreeItem::ColumnSection(si, oi));
                    if app.schema_tree.is_section_expanded(si, oi, 0) {
                        for (ci, _col) in obj.columns.iter().enumerate() {
                            app.schema_tree
                                .flat_items
                                .push(SchemaTreeItem::ColumnRow(si, oi, ci));
                        }
                    }

                    app.schema_tree
                        .flat_items
                        .push(SchemaTreeItem::IndexSection(si, oi));
                    if app.schema_tree.is_section_expanded(si, oi, 1) {
                        for (ii, _idx) in obj.indexes.iter().enumerate() {
                            app.schema_tree
                                .flat_items
                                .push(SchemaTreeItem::IndexRow(si, oi, ii));
                        }
                    }

                    app.schema_tree
                        .flat_items
                        .push(SchemaTreeItem::TriggerSection(si, oi));
                    if app.schema_tree.is_section_expanded(si, oi, 2) {
                        for (ti, _trg) in obj.triggers.iter().enumerate() {
                            app.schema_tree
                                .flat_items
                                .push(SchemaTreeItem::TriggerRow(si, oi, ti));
                        }
                    }

                    app.schema_tree
                        .flat_items
                        .push(SchemaTreeItem::ConstraintSection(si, oi));
                    if app.schema_tree.is_section_expanded(si, oi, 3) {
                        for (ci, _con) in obj.constraints.iter().enumerate() {
                            app.schema_tree
                                .flat_items
                                .push(SchemaTreeItem::ConstraintRow(si, oi, ci));
                        }
                    }

                    // Show owned sequences under this table
                    if let Some(seq_indices) = owned_seqs.get(obj.name.as_str()) {
                        app.schema_tree
                            .flat_items
                            .push(SchemaTreeItem::SequenceSection(si, oi));
                        if app.schema_tree.is_section_expanded(si, oi, 4) {
                            for &seq_oi in seq_indices {
                                app.schema_tree
                                    .flat_items
                                    .push(SchemaTreeItem::SequenceRow(si, seq_oi));
                            }
                        }
                    }
                }
            }
        }
    }

    if app.schema_tree.flat_items.len() != old_len {
        let max = app.schema_tree.flat_items.len().saturating_sub(1);
        let sel = app.schema_tree.list_state.selected().unwrap_or(0);
        if app.schema_tree.flat_items.is_empty() {
            app.schema_tree.list_state.select(None);
        } else {
            app.schema_tree.list_state.select(Some(sel.min(max)));
        }
    }
}
