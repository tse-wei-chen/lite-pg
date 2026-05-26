pub mod cache;
pub mod connection;
pub mod databases;
pub mod extensions;
pub mod functions;
pub mod query;
pub mod replication;
pub mod roles;
pub mod schema;
pub mod search;
pub mod settings;
pub mod statistics;

pub use connection::connect;
pub use databases::{fetch_databases, generate_create_database_sql, generate_drop_database_sql, DatabaseInfo};
pub use extensions::{fetch_extensions, ExtensionInfo};
pub use functions::{fetch_functions, FunctionInfo};
pub use query::{execute_query, QueryResult};
pub use replication::{fetch_publications, fetch_subscriptions, PublicationInfo, SubscriptionInfo};
pub use roles::{fetch_roles, generate_create_role_sql, generate_drop_role_sql, RoleInfo};
pub use schema::{
    fetch_ddl, fetch_object_detail, fetch_schemas, fetch_table_data, prefetch_all_details,
    DbObject, DbObjectType, SchemaInfo,
};
pub use search::{search_objects, SearchResult};
pub use settings::{fetch_settings, SettingInfo};
pub use statistics::{
    fetch_active_queries, fetch_db_stats, fetch_server_overview, fetch_table_stats, ActiveQuery,
    DbStatEntry, ServerOverview, TableStatEntry,
};
