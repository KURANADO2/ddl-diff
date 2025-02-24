use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Table {
    pub table_name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Column {
    pub table_name: String,
    pub column_name: String,
    pub original_position: u8,
    pub column_default: Option<String>,
    pub is_nullable: String,
    pub column_type: String,
    pub extra: String,
    pub column_comment: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Index {
    pub table_name: String,
    pub index_name: String,
    pub non_unique: bool,
    pub column_names: Vec<String>,
    pub index_type: String,
    pub extra: String,
}