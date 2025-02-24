mod options;
mod table;

use crate::options::DbConfig;
use crate::table::{Column, Index, Table};
use clap::Parser;
use sqlx::mysql::MySqlPool;
use sqlx::Row;
use std::collections::HashMap;

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
        "SELECT
         t1.TABLE_NAME AS table_name,
         t1.INDEX_NAME AS index_name,
         t1.NON_UNIQUE AS non_unique,
         t1.COLUMN_NAME AS column_name,
         CAST(t1.INDEX_TYPE AS CHAR) AS index_type,
         t2.extra AS extra
         FROM INFORMATION_SCHEMA.STATISTICS t1
         INNER JOIN INFORMATION_SCHEMA.COLUMNS t2
         ON t1.TABLE_SCHEMA = t2.TABLE_SCHEMA AND t1.TABLE_NAME = t2.TABLE_NAME AND t1.COLUMN_NAME = t2.COLUMN_NAME
         WHERE t1.TABLE_SCHEMA = '{}' ORDER BY t1.TABLE_NAME, t1.INDEX_NAME, t1.SEQ_IN_INDEX;",
        schema
    );

    let rows = sqlx::query(&query)
        .bind(schema)
        .fetch_all(pool)
        .await
        .unwrap();

    let mut map: HashMap<(String, String), (bool, Vec<String>, String, String)> = HashMap::new();

    for row in rows {
        let table_name: String = row.get(0);
        let index_name: String = row.get(1);
        let non_unique: bool = row.get::<i8, _>(2) == 1;
        let column_name: String = row.get(3);
        let index_type: String = row.get(4);
        let extra: String = row.get(5);

        let key = (table_name.clone(), index_name.clone());
        let value = map
            .entry(key)
            .or_insert((non_unique, Vec::new(), index_type, extra));
        value.1.push(column_name);
    }

    map.into_iter()
        .map(
            |((table_name, index_name), (non_unique, column_names, index_type, extra))| Index {
                table_name,
                index_name,
                non_unique,
                column_names,
                index_type,
                extra,
            },
        )
        .collect()
}

async fn get_tables(pool: &MySqlPool, schema: &str) -> Vec<Table> {
    let mut result: Vec<Table> = Vec::new();

    let columns = get_columns(pool, schema).await;
    let columns = group_columns(columns);

    let indexes = get_indexes(pool, schema).await;
    let indexes = group_indexes(indexes);

    for (table_name, columns) in columns {
        let indexes = indexes.get(&table_name).unwrap_or(&Vec::new()).clone();
        result.push(Table {
            table_name: table_name.clone(),
            columns,
            indexes,
        });
    }

    result
}

fn map_tables(tables: Vec<Table>) -> HashMap<String, Table> {
    tables
        .into_iter()
        .map(|t| (t.table_name.clone(), t))
        .collect()
}

fn map_columns(columns: &Vec<Column>) -> HashMap<String, Column> {
    columns
        .into_iter()
        .map(|c| (c.column_name.clone(), c.clone()))
        .collect()
}

fn map_indexes(indexes: &Vec<Index>) -> HashMap<String, Index> {
    indexes
        .into_iter()
        .map(|i| (i.index_name.clone(), i.clone()))
        .collect()
}

fn group_columns(columns: Vec<Column>) -> HashMap<String, Vec<Column>> {
    let mut result: HashMap<String, Vec<Column>> = HashMap::new();

    for column in columns {
        let entry = result
            .entry(column.table_name.clone())
            .or_insert_with(Vec::new);
        entry.push(column);
    }

    result
}

fn group_indexes(indexes: Vec<Index>) -> HashMap<String, Vec<Index>> {
    let mut result: HashMap<String, Vec<Index>> = HashMap::new();

    for index in indexes {
        let entry = result
            .entry(index.table_name.clone())
            .or_insert_with(Vec::new);
        entry.push(index);
    }

    result
}

fn compare_tables(original_tables: Vec<Table>, target_tables: Vec<Table>) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();

    let original_tables = map_tables(original_tables);
    let target_tables = map_tables(target_tables);

    // Create tables
    for (original_table_name, original_table) in &original_tables {
        match target_tables.get(original_table_name) {
            // Not found -> New table
            None => result.push(generate_create_table(original_table)),
            _ => {}
        }
    }

    // Alter tables
    for (original_table_name, original_table) in &original_tables {
        match target_tables.get(original_table_name) {
            // Same table, compare columns
            Some(target_table) => {
                result.extend(alter_tables(&original_table, &target_table));
            }
            _ => {}
        }
    }

    // Drop tables
    for (target_table_name, target_table) in &target_tables {
        // Not found
        if !original_tables.contains_key(target_table_name) {
            // drop table
            result.push(generate_drop_table(&target_table));
        }
    }

    result
}

fn alter_tables(original_table: &Table, target_table: &Table) -> Vec<String> {
    let mut result = Vec::new();

    // drop index -> add column -> modify column -> drop column -> add index

    let original_indexes = map_indexes(&original_table.indexes);
    let target_indexes = map_indexes(&target_table.indexes);

    let original_columns = map_columns(&original_table.columns);
    let target_columns = map_columns(&target_table.columns);

    // Drop indexes
    for (target_index_name, target_index) in &target_indexes {
        // Not found
        if !original_indexes.contains_key(target_index_name) {
            // drop index
            result.push(generate_drop_index(target_index));
        }
    }

    // Drop indexes
    for (original_index_name, original_index) in &original_indexes {
        match target_indexes.get(original_index_name) {
            // Same index, compare index attr
            Some(target_index) => {
                if !index_is_same_attr(original_index, target_index) {
                    result.push(generate_drop_index(target_index));
                }
            }
            _ => {}
        }
    }

    // Add columns
    let mut prev_column_name = String::from("");
    // traverse by column order
    for original_column in &original_table.columns {
        let original_column = &original_columns.get(&original_column.column_name).unwrap();
        match target_columns.get(&original_column.column_name) {
            // Not found -> New column
            None => result.push(generate_add_column(original_column, prev_column_name)),
            _ => {}
        }
        prev_column_name = original_column.column_name.to_string();
    }

    // Modify columns
    let mut prev_column_name = String::from("");
    // traverse by column order
    for original_column in &original_table.columns {
        let original_column = &original_columns.get(&original_column.column_name).unwrap();
        match target_columns.get(&original_column.column_name) {
            // Same column, compare column attr
            Some(target_column) => {
                if !column_is_same_attr(original_column, target_column) {
                    result.push(generate_modify_column(original_column, prev_column_name));
                }
            }
            _ => {}
        }
        prev_column_name = original_column.column_name.to_string();
    }

    // Drop columns
    for (target_column_name, target_column) in &target_columns {
        // Not found
        if !original_columns.contains_key(target_column_name) {
            // drop column
            result.push(generate_drop_column(target_column));
        }
    }

    // Add index
    for (original_index_name, original_index) in &original_indexes {
        match target_indexes.get(original_index_name) {
            // Not found -> New index
            None => result.push(generate_add_index(original_index)),
            // Same index, compare index attr
            Some(target_index) => {
                if !index_is_same_attr(original_index, target_index) {
                    result.push(generate_add_index(original_index));
                }
            }
        }
    }

    result
}

fn generate_create_table(table: &Table) -> String {
    let mut result = String::from("CREATE TABLE `");

    result.push_str(&table.table_name);
    result.push_str("`(\n");

    let mut list: Vec<String> = Vec::new();

    // columns
    for column in table.columns.iter() {
        list.push(generate_column(&column));
    }

    // indexes
    for index in table.indexes.iter() {
        let index_detail = generate_index(&index);
        if index_detail != "" {
            list.push(index_detail);
        }
    }

    result.push_str(list.join(",\n").as_str());

    result.push_str(");");

    result
}

fn generate_drop_table(table: &Table) -> String {
    format!("DROP TABLE IF EXISTS `{}`;", table.table_name)
}

fn generate_add_column(column: &Column, prev_column_name: String) -> String {
    let mut result = format!(
        "ALTER TABLE `{}` ADD COLUMN {}",
        column.table_name,
        generate_column(column)
    );

    match prev_column_name.as_str() {
        "" => result.push_str(&" FIRST".to_string()),
        _ => result.push_str(&format!(" AFTER `{}`", prev_column_name)),
    }

    result.push_str(";");

    result
}

fn generate_modify_column(column: &Column, prev_column_name: String) -> String {
    let mut result = format!(
        "ALTER TABLE `{}` MODIFY COLUMN {}",
        column.table_name,
        generate_column(column)
    );

    match prev_column_name.as_str() {
        "" => result.push_str(&" FIRST".to_string()),
        _ => result.push_str(&format!(" AFTER `{}`", prev_column_name)),
    }

    result.push_str(";");

    result
}

fn generate_drop_column(column: &Column) -> String {
    format!(
        "ALTER TABLE `{}` DROP COLUMN `{}`;",
        column.table_name, column.column_name
    )
}

fn generate_column(column: &Column) -> String {
    format!(
        "`{}` {} {} {} {} COMMENT '{}'",
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
        if column.extra == "auto_increment" {
            "PRIMARY KEY AUTO_INCREMENT"
        } else {
            ""
        },
        column.column_comment
    )
}

fn generate_add_index(index: &Index) -> String {
    let index_detail = generate_index(index);

    if index_detail != "" {
        return format!("ALTER TABLE `{}` ADD {};", index.table_name, index_detail);
    }

    String::new()
}

fn generate_drop_index(index: &Index) -> String {
    format!(
        "ALTER TABLE `{}` DROP INDEX `{}`;",
        index.table_name, index.index_name
    )
}

fn generate_index(index: &Index) -> String {
    let mut result = String::new();

    if index.index_name == "PRIMARY" && index.extra == "auto_increment" {
        return result;
    }

    match index.non_unique {
        false => match index.index_name.as_str() {
            "PRIMARY" => result.push_str("PRIMARY KEY"),
            _ => result.push_str(&format!("UNIQUE INDEX `{}`", index.index_name)),
        },
        true => result.push_str(&format!("INDEX `{}`", index.index_name)),
    }

    result.push_str(" (");

    for (i, column_name) in index.column_names.iter().enumerate() {
        result.push_str(&format!("`{}`", column_name));
        if i < index.column_names.iter().len() - 1 {
            result.push_str(",");
        }
    }

    result.push_str(") ");

    result.push_str(&format!("USING {}", index.index_type));

    result
}

fn column_is_same_attr(original_column: &Column, target_column: &Column) -> bool {
    original_column.column_type == target_column.column_type
        && original_column.is_nullable == target_column.is_nullable
        && original_column.column_default == target_column.column_default
        && original_column.extra == target_column.extra
        && original_column.column_comment == target_column.column_comment
}

fn index_is_same_attr(original_index: &Index, target_index: &Index) -> bool {
    original_index.non_unique == target_index.non_unique
        && original_index.column_names == target_index.column_names
        && original_index.index_type == target_index.index_type
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

    let original_tables = get_tables(&original_pool, &args.original_schema).await;
    let target_tables = get_tables(&target_pool, &args.target_schema).await;

    let ddl_statements = compare_tables(original_tables, target_tables);

    if ddl_statements.len() > 0 {
        println!("use {};", &args.target_schema);
        for ddl in ddl_statements {
            println!("{}", ddl);
        }
    }
}
