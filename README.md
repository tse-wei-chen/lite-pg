# lite-pg

Lightning-fast PostgreSQL TUI client built with Rust and [ratatui](https://ratatui.rs/).

https://github.com/user-attachments/assets/9abc322e-9ad0-4ba4-8239-536f4a2e20bb

## Install

| Method | Command |
|--------|---------|
| cargo | `cargo install --git https://github.com/tse-wei-chen/lite-pg` |
| binary | Download from [Releases](https://github.com/tse-wei-chen/lite-pg/releases) |

## Quick Start

```bash
lite-pg
```

Press `n` on the connection panel to add a PostgreSQL connection profile, then `Enter` to connect.

## Features

- Connection profile management with AES-256-GCM encrypted password storage
- Schema tree browser (schemas → tables/views → columns/indexes/triggers/constraints) with batch prefetch
- SQL query editor with multi-tab buffers and auto-completion
- Query execution with result viewer
- **Server dashboard** — version, uptime, active connections, database stats, active queries, table statistics
- **Role/user management** — list, create, drop roles with attributes
- **Database management** — list, create, drop databases with size/owner/encoding
- **Function/procedure browser** — list functions with DDL viewer
- **Extension manager** — list installed extensions
- **Server configuration viewer** — browse pg_settings
- **Replication management** — publications and subscriptions
- **Global object search** — find objects across all schemas
- **Query bookmarks** — save and load frequently used queries
- Object DDL viewer
- DROP / TRUNCATE / VACUUM with confirmation
- Table data browsing with pagination
- Visual mode for selecting result rows
- Export results: CSV, JSON, SQL INSERT, Markdown (file / clipboard)
- Query history with fuzzy search

## Keybindings

The interface uses a vim-inspired modal design: **Normal** (default, navigate + trigger actions), **Insert** (typing SQL), **Visual** (select result rows).

### Common

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus: Schema → Query Input → Results |
| `Esc` | Open / close main menu (overlay popup) |
| `j` / `k` | Move selection down / up |
| `Enter` | Expand/collapse tree node, confirm action |
| `Ctrl+T` | New query tab |
| `Ctrl+W` | Close current query tab |
| `Alt+Enter` | Execute current query |
| `F5` | EXPLAIN ANALYZE current query |
| `i` | Enter Insert mode (SQL editor) / Generate INSERT template (schema tree) |
| `/` | Toggle query history |
| `q` | Quit |

### Schema Tree

| Key | Action |
|-----|--------|
| `o` | Generate SELECT + execute in new tab |
| `u` | Generate UPDATE template in new tab |
| `D` | Show DDL of selected object |
| `d` | Drop selected object (with confirmation) |
| `t` | Truncate selected table (with confirmation) |
| `v` | Vacuum selected table (with confirmation) |
| `n` / `p` | Next / previous page of browsed table data |

### SQL Editor (Normal mode focus on editor)

| Key | Action |
|-----|--------|
| `i` | Enter Insert mode |
| `[` / `]` | Previous / next query tab |

### Results

| Key | Action |
|-----|--------|
| `h` / `l` | Scroll left / right |
| `v` | Enter Visual mode |

### Visual Mode

| Key | Action |
|-----|--------|
| `Esc` / `v` | Return to Normal mode |
| `j` / `k` | Extend selection down / up |

### Insert Mode (SQL Editor)

| Key | Action |
|-----|--------|
| `Esc` | Return to Normal mode |

### Connection Panel

| Key | Action |
|-----|--------|
| `Enter` | Connect to selected profile |
| `n` | New connection profile |
| `e` | Edit selected profile |
| `d` | Delete selected profile |
| `Esc` | Close panel (if connected) |

### Connection Form

| Key | Action |
|-----|--------|
| `Enter` | Next field / Save |
| `Tab` | Next field |
| `Esc` | Cancel |
| `Shift+Insert` / `Ctrl+V` | Paste into field |

### Confirmation Dialog

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm |
| `n` / `Esc` | Cancel |

### Main Menu (ESC overlay)

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up / down |
| `Enter` | Select item / enter submenu |
| `Esc` | Close menu / go back from submenu |

Menu items: Home, Dashboard, Bookmarks, Search, Management (Roles, Databases, Functions, Extensions, Settings, Replication), Connection (Connection Manager), Export (CSV, JSON, SQL INSERT, Markdown file, Markdown clipboard), Help, Quit.

A shortcut hint bar is shown at the bottom of every page, displaying the most relevant keys for the current focus.
