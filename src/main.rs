use clap::Parser;
use sqlx::mysql::MySqlPool;
use sqlx::Row;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct DbConfig {
    #[arg(long)]
    original_user: String,
    #[arg(long)]
    original_password: String,
    #[arg(long)]
    original_host: String,
    #[arg(long, default_value = "3306")]
    original_port: String,
    #[arg(long)]
    original_schema: String,

    #[arg(long)]
    target_user: String,
    #[arg(long)]
    target_password: String,
    #[arg(long)]
    target_host: String,
    #[arg(long, default_value = "3306")]
    target_port: String,
    #[arg(long)]
    target_schema: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct Column {
    table_name: String,
    column_name: String,
    original_position: u8,
    column_default: Option<String>,
    is_nullable: String,
    column_type: String,
    extra: String,
    column_comment: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct Index {
    table_name: String,
    index_name: String,
    non_unique: bool,
    column_names: Vec<String>,
}

struct ColumnResult {
    ddl_statements: Vec<String>,
    deleted_tables: Vec<String>,
}

async fn get_columns(pool: &MySqlPool, schema: &str) -> Vec<Column> {
    let query = format!(
        "SELECT
         TABLE_NAME AS table_name,
         COLUMN_NAME AS column_name,
         ORDINAL_POSITION AS original_position,
         CAST(COLUMN_DEFAULT AS CHAR) AS column_default,
         IS_NULLABLE AS is_nullable,
         CAST(COLUMN_TYPE AS CHAR) AS column_type,
         EXTRA AS extra,
         CAST(COLUMN_COMMENT AS CHAR) AS column_comment
         FROM INFORMATION_SCHEMA.COLUMNS
         WHERE TABLE_SCHEMA = '{}' ORDER BY TABLE_NAME, ORDINAL_POSITION;",
        schema
    );

    sqlx::query_as::<_, Column>(&query)
        .bind(schema)
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn get_indexes(pool: &MySqlPool, schema: &str) -> Vec<Index> {
    let query = format!(
        "SELECT TABLE_NAME as table_name,
         INDEX_NAME as index_name,
         NON_UNIQUE as non_unique,
         COLUMN_NAME as column_name
         FROM information_schema.statistics
         WHERE TABLE_SCHEMA = '{}' ORDER BY TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX;",
        schema
    );

    let rows = sqlx::query(&query)
        .bind(schema)
        .fetch_all(pool)
        .await
        .unwrap();

    let mut map: HashMap<(String, String), (bool, Vec<String>)> = HashMap::new();

    for row in rows {
        let table_name: String = row.get(0);
        let index_name: String = row.get(1);
        let non_unique: bool = row.get::<i8, _>(2) == 1;
        let column_name: String = row.get(3);

        let key = (table_name.clone(), index_name.clone());
        let value = map.entry(key).or_insert((non_unique, Vec::new()));
        value.1.push(column_name);
    }

    map.into_iter()
        .map(
            |((table_name, index_name), (non_unique, column_names))| Index {
                table_name,
                index_name,
                non_unique,
                column_names,
            },
        )
        .collect()
}

fn compare_columns(original_columns: &Vec<Column>, target_columns: &Vec<Column>) -> ColumnResult {
    let mut ddl_statements = Vec::new();

    // group columns by table name
    let original = group_column_by_table(original_columns);
    let target = group_column_by_table(target_columns);

    // Traversal original columns
    for (table_name, original_columns) in original.iter() {
        // Get the same table from target tables
        match target.get(table_name) {
            // Table exists
            Some(target_columns) => {
                // Compare fields
                for (original_column_name, original_column_detail) in original_columns.iter() {
                    // Field does not exist
                    if !target_columns.contains_key(original_column_name) {
                        ddl_statements.push(generate_add_column(&original_column_detail));
                        // Field exists, but the field attributes are different
                    } else if !compare_column_attr(
                        &target_columns[original_column_name],
                        original_column_detail,
                    ) {
                        ddl_statements.push(generate_modify_column(&original_column_detail));
                    }
                }
                for (target_column_name, target_column_detail) in target_columns.iter() {
                    if !original_columns.contains_key(target_column_name) {
                        ddl_statements.push(generate_drop_column(target_column_detail));
                    }
                }
            }
            None => {
                // Table does not exist
                ddl_statements.push(generate_create_table(&table_name, original_columns));
            }
        }
    }

    let mut deleted_tables = Vec::new();
    // The table is being deleted
    for table in target.keys() {
        if !original.contains_key(table) {
            deleted_tables.push(table.clone());
            ddl_statements.push(format!("DROP TABLE {};", table));
        }
    }

    ColumnResult {
        ddl_statements,
        deleted_tables,
    }
}

fn compare_indexes(
    original_indexes: &Vec<Index>,
    target_indexes: &Vec<Index>,
    deleted_tables: &Vec<String>,
) -> Vec<String> {
    let mut result = Vec::new();

    let original = group_index_by_table_index(original_indexes);
    let target = group_index_by_table_index(target_indexes);

    for (key, original_index) in original.iter() {
        if !target.contains_key(key) {
            result.push(generate_create_index(original_index));
        } else if let Some(target_index) = target.get(key) {
            // The index attr has difference
            if original_index.column_names != target_index.column_names
                || original_index.non_unique != target_index.non_unique
            {
                result.push(generate_drop_index(original_index));
                result.push(generate_create_index(original_index));
            }
        }
    }

    for (key, target_index) in &target {
        if !original.contains_key(key) && !deleted_tables.contains(&target_index.table_name) {
            result.push(generate_drop_index(target_index));
        }
    }

    result
}

fn generate_add_column(column: &Column) -> String {
    format!(
        "ALTER TABLE {} ADD COLUMN {};",
        column.table_name,
        generate_column(column)
    )
}

fn generate_modify_column(column: &Column) -> String {
    format!(
        "ALTER TABLE {} MODIFY COLUMN {};",
        column.table_name,
        generate_column(column)
    )
}

fn generate_drop_column(column: &Column) -> String {
    format!(
        "ALTER TABLE {} DROP COLUMN {};",
        column.table_name, column.column_name
    )
}

fn generate_column(column: &Column) -> String {
    format!(
        "{} {} {} {} COMMENT '{}'",
        column.column_name,
        column.column_type,
        if column.is_nullable == "YES" {
            "NULL"
        } else {
            "NOT NULL"
        },
        match &column.column_default {
            Some(default) => format!("DEFAULT {}", default),
            None => String::new(),
        },
        column.column_comment
    )
}

fn generate_create_index(index: &Index) -> String {
    if "PRIMARY" == index.index_name {
        return format!(
            "ALTER TABLE {} ADD PRIMARY KEY ({});",
            index.table_name,
            index.column_names.join(", ")
        );
    }

    format!(
        "CREATE {}INDEX {} ON {} ({});",
        if index.non_unique { "" } else { "UNIQUE " },
        index.index_name,
        index.table_name,
        index.column_names.join(", ")
    )
}

fn generate_drop_index(index: &Index) -> String {
    format!("DROP INDEX {} ON {};", index.index_name, index.table_name)
}

fn generate_create_table(table_name: &String, columns: &HashMap<String, Column>) -> String {
    let mut result = String::from("CREATE TABLE ");
    result.push_str(table_name);
    result.push_str("(\n");

    for (i, (column_name, column_detail)) in columns.iter().enumerate() {
        result.push_str(generate_column(column_detail).as_str());
        if i < columns.len() - 1 {
            result.push_str(",");
        }
        result.push_str("\n");
    }

    result.push_str(");");

    result
}

fn compare_column_attr(original_column: &Column, target_column: &Column) -> bool {
    original_column.column_type == target_column.column_type
        && original_column.is_nullable == target_column.is_nullable
        && original_column.column_default == target_column.column_default
        && original_column.column_comment == target_column.column_comment
}

// HashMap<Table Name, HashMap<Column Name, Column>>
fn group_column_by_table(columns: &Vec<Column>) -> HashMap<String, HashMap<String, Column>> {
    let mut result: HashMap<String, HashMap<String, Column>> = HashMap::new();

    for column in columns {
        let entry = result
            .entry(column.table_name.clone())
            .or_insert_with(HashMap::new);
        entry.insert(column.column_name.clone(), column.clone());
    }

    result
}

fn group_index_by_table_index(indexes: &Vec<Index>) -> HashMap<(String, String), Index> {
    indexes
        .iter()
        .map(|index| {
            (
                (index.table_name.clone(), index.index_name.clone()),
                index.clone(),
            )
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let args = DbConfig::parse();

    let original_pool = MySqlPool::connect(
        format!(
            "mysql://{}:{}@{}:{}/{}",
            args.original_user,
            args.original_password,
            args.original_host,
            args.original_port,
            args.original_schema
        )
        .as_str(),
    )
    .await
    .unwrap();

    let target_pool = MySqlPool::connect(
        format!(
            "mysql://{}:{}@{}:{}/{}",
            args.target_user,
            args.target_password,
            args.target_host,
            args.target_port,
            args.target_schema
        )
        .as_str(),
    )
    .await
    .unwrap();

    let original_columns = get_columns(&original_pool, &args.original_schema).await;
    let target_columns = get_columns(&target_pool, &args.target_schema).await;

    let mut column_result = compare_columns(&original_columns, &target_columns);

    let original_indexes = get_indexes(&original_pool, &args.original_schema).await;
    let target_indexes = get_indexes(&target_pool, &args.target_schema).await;

    column_result.ddl_statements.extend(compare_indexes(
        &original_indexes,
        &target_indexes,
        &column_result.deleted_tables,
    ));

    if column_result.ddl_statements.len() > 0 {
        println!("use {};", &args.target_schema);
    }

    for ddl in column_result.ddl_statements {
        println!("{}", ddl);
    }
}
