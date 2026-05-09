use crate::type_system::{DatabasePoolValue, SupportedSqlPool, Value};
use crate::{CorvoError, CorvoResult};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Column, ColumnIndex, Row, TypeInfo, ValueRef};
use std::collections::HashMap;
use std::sync::Arc;

fn classify_db_url(url: &str) -> CorvoResult<DbScheme> {
    let u = url.trim();
    if u.starts_with("sqlite:") {
        Ok(DbScheme::Sqlite)
    } else if u.starts_with("postgres://") || u.starts_with("postgresql://") {
        Ok(DbScheme::Postgres)
    } else {
        Err(CorvoError::runtime(
            "db.connect URL must start with sqlite:, postgres://, or postgresql:// (MySQL is not supported)",
        ))
    }
}

#[derive(Clone, Copy)]
enum DbScheme {
    Sqlite,
    Postgres,
}

/// PostgreSQL dialect uses numbered placeholders (`$1`, `$2`). Corvo uses `?`
/// placeholders (matching SQLite style). Rewrite positional `?` in order before
/// calling `sqlx` on PostgreSQL pools.
fn postgres_rewrite_placeholders(sql: &str) -> String {
    let mut n = 0usize;
    let mut out = String::with_capacity(sql.len() + sql.matches('?').count() + 8);
    for ch in sql.chars() {
        if ch == '?' {
            n += 1;
            out.push('$');
            out.push_str(&n.to_string());
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn connect(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() || args.len() > 2 {
        return Err(CorvoError::runtime(
            "db.connect expects 1 or 2 arguments: (url, [max_connections])",
        ));
    }

    let url = args[0]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("db.connect expects URL as string"))?;

    let scheme = classify_db_url(url)?;

    let max_connections = if args.len() == 2 {
        let n = args[1]
            .as_number()
            .ok_or_else(|| CorvoError::r#type("db.connect expects max_connections as number"))?;
        if n <= 0.0 || n.fract() != 0.0 {
            return Err(CorvoError::r#type(
                "db.connect expects max_connections to be a positive integer",
            ));
        }
        n as u32
    } else {
        10
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CorvoError::runtime(format!("Failed to build tokio runtime: {}", e)))?;

    let pool_inner = rt.block_on(async {
        match scheme {
            DbScheme::Sqlite => SqlitePoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await
                .map(SupportedSqlPool::Sqlite)
                .map_err(|e| CorvoError::runtime(format!("Failed to connect to database: {}", e))),
            DbScheme::Postgres => PgPoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await
                .map(SupportedSqlPool::Postgres)
                .map_err(|e| CorvoError::runtime(format!("Failed to connect to database: {}", e))),
        }
    })?;

    Ok(Value::DatabasePool(Box::new(DatabasePoolValue(
        Arc::new(rt),
        Arc::new(pool_inner),
    ))))
}

fn bind_sqlite<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    args: &'q [Value],
) -> CorvoResult<sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>> {
    for arg in args {
        query = bind_one_sqlite(query, arg)?;
    }
    Ok(query)
}

fn bind_one_sqlite<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    arg: &'q Value,
) -> CorvoResult<sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>> {
    match arg {
        Value::String(s) => Ok(query.bind(s.clone())),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                Ok(query.bind(*n as i64))
            } else {
                Ok(query.bind(*n))
            }
        }
        Value::Boolean(b) => Ok(query.bind(*b)),
        Value::Null => Err(CorvoError::r#type(
            "Cannot bind null value to SQL query (unsupported typed null)",
        )),
        _ => Err(CorvoError::r#type(
            "Unsupported argument type for SQL query",
        )),
    }
}

fn bind_postgres<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    args: &'q [Value],
) -> CorvoResult<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>> {
    for arg in args {
        query = bind_one_postgres(query, arg)?;
    }
    Ok(query)
}

fn bind_one_postgres<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    arg: &'q Value,
) -> CorvoResult<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>> {
    match arg {
        Value::String(s) => Ok(query.bind(s.clone())),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                Ok(query.bind(*n as i64))
            } else {
                Ok(query.bind(*n))
            }
        }
        Value::Boolean(b) => Ok(query.bind(*b)),
        Value::Null => Err(CorvoError::r#type(
            "Cannot bind null value to SQL query (unsupported typed null)",
        )),
        _ => Err(CorvoError::r#type(
            "Unsupported argument type for SQL query",
        )),
    }
}

fn extract_row<R>(row: &R) -> CorvoResult<Value>
where
    R: Row,
    usize: ColumnIndex<R>,
    bool: sqlx::Type<R::Database> + for<'r> sqlx::Decode<'r, R::Database>,
    f32: sqlx::Type<R::Database> + for<'r> sqlx::Decode<'r, R::Database>,
    f64: sqlx::Type<R::Database> + for<'r> sqlx::Decode<'r, R::Database>,
    i32: sqlx::Type<R::Database> + for<'r> sqlx::Decode<'r, R::Database>,
    i64: sqlx::Type<R::Database> + for<'r> sqlx::Decode<'r, R::Database>,
    String: sqlx::Type<R::Database> + for<'r> sqlx::Decode<'r, R::Database>,
{
    let mut map = HashMap::new();
    for i in 0..row.columns().len() {
        let col = row.column(i);
        let name = col.name().to_string();

        if map.contains_key(&name) {
            return Err(CorvoError::runtime(format!(
                "Duplicate column name detected: {}",
                name
            )));
        }

        let val_ref = match row.try_get_raw(i) {
            Ok(v) => v,
            Err(_) => {
                map.insert(name, Value::Null);
                continue;
            }
        };

        if val_ref.is_null() {
            map.insert(name, Value::Null);
            continue;
        }

        if let Ok(v) = row.try_get::<String, _>(i) {
            map.insert(name, Value::String(v));
        } else if let Ok(v) = row.try_get::<i64, _>(i) {
            map.insert(name, Value::Number(v as f64));
        } else if let Ok(v) = row.try_get::<i32, _>(i) {
            map.insert(name, Value::Number(v as f64));
        } else if let Ok(v) = row.try_get::<f64, _>(i) {
            map.insert(name, Value::Number(v));
        } else if let Ok(v) = row.try_get::<f32, _>(i) {
            map.insert(name, Value::Number(v as f64));
        } else if let Ok(v) = row.try_get::<bool, _>(i) {
            map.insert(name, Value::Boolean(v));
        } else {
            let type_name = col.type_info().name().to_string();
            return Err(CorvoError::runtime(format!(
                "Unsupported database column type '{}' for column '{}'",
                type_name, name
            )));
        }
    }
    Ok(Value::Map(map))
}

pub fn query(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::runtime(
            "db.query expects at least 2 arguments: (pool, sql, [args...])",
        ));
    }

    let (rt, pool) = match &args[0] {
        Value::DatabasePool(d) => (&d.0, d.1.as_ref()),
        _ => {
            return Err(CorvoError::r#type(
                "db.query expects a database pool as the first argument",
            ))
        }
    };

    let sql = args[1]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("db.query expects SQL query as string"))?;

    let query_args = &args[2..];

    rt.block_on(async {
        match pool {
            SupportedSqlPool::Sqlite(p) => {
                let q = bind_sqlite(sqlx::query(sql), query_args)?;
                let rows = q
                    .fetch_all(p)
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Query failed: {}", e)))?;
                let mut mapped_rows = Vec::new();
                for row in rows {
                    mapped_rows.push(extract_row(&row)?);
                }
                Ok(Value::List(mapped_rows))
            }
            SupportedSqlPool::Postgres(p) => {
                let sql_pg = postgres_rewrite_placeholders(sql);
                let q = bind_postgres(sqlx::query(&sql_pg), query_args)?;
                let rows = q
                    .fetch_all(p)
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Query failed: {}", e)))?;
                let mut mapped_rows = Vec::new();
                for row in rows {
                    mapped_rows.push(extract_row(&row)?);
                }
                Ok(Value::List(mapped_rows))
            }
        }
    })
}

pub fn execute(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::runtime(
            "db.execute expects at least 2 arguments: (pool, sql, [args...])",
        ));
    }

    let (rt, pool) = match &args[0] {
        Value::DatabasePool(d) => (&d.0, d.1.as_ref()),
        _ => {
            return Err(CorvoError::r#type(
                "db.execute expects a database pool as the first argument",
            ))
        }
    };

    let sql = args[1]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("db.execute expects SQL query as string"))?;

    let query_args = &args[2..];

    rt.block_on(async {
        match pool {
            SupportedSqlPool::Sqlite(p) => {
                let q = bind_sqlite(sqlx::query(sql), query_args)?;
                let result = q
                    .execute(p)
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Execute failed: {}", e)))?;

                Ok(Value::Number(result.rows_affected() as f64))
            }
            SupportedSqlPool::Postgres(p) => {
                let sql_pg = postgres_rewrite_placeholders(sql);
                let q = bind_postgres(sqlx::query(&sql_pg), query_args)?;
                let result = q
                    .execute(p)
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Execute failed: {}", e)))?;

                Ok(Value::Number(result.rows_affected() as f64))
            }
        }
    })
}

pub fn close(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() != 1 {
        return Err(CorvoError::runtime("db.close expects 1 argument: (pool)"));
    }

    let (rt, pool) = match &args[0] {
        Value::DatabasePool(d) => (&d.0, d.1.as_ref()),
        _ => {
            return Err(CorvoError::r#type(
                "db.close expects a database pool as the first argument",
            ))
        }
    };

    rt.block_on(async {
        match pool {
            SupportedSqlPool::Sqlite(p) => {
                p.close().await;
                Ok(Value::Null)
            }
            SupportedSqlPool::Postgres(p) => {
                p.close().await;
                Ok(Value::Null)
            }
        }
    })
}

#[macro_export]
macro_rules! db_connect {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.connect", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.connect", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! db_query {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.query", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.query", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! db_execute {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.execute", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.execute", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! db_close {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.close", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("db.close", &[$($arg),*], &$kwargs, $state)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_system::Value;
    use std::collections::HashMap;

    #[test]
    fn test_db_lifecycle() {
        let args_connect = vec![
            Value::String("sqlite://?mode=memory&cache=shared".to_string()),
            Value::Number(1.0),
        ];
        let pool_val = connect(&args_connect, &HashMap::new()).unwrap();

        let args_exec = vec![
            pool_val.clone(),
            Value::String("CREATE TABLE test (id INTEGER, val TEXT)".to_string()),
        ];
        execute(&args_exec, &HashMap::new()).unwrap();

        let args_insert = vec![
            pool_val.clone(),
            Value::String("INSERT INTO test (id, val) VALUES (?, ?)".to_string()),
            Value::Number(1.0),
            Value::String("hello".to_string()),
        ];
        execute(&args_insert, &HashMap::new()).unwrap();

        let args_query = vec![
            pool_val.clone(),
            Value::String("SELECT * FROM test".to_string()),
        ];
        let result = query(&args_query, &HashMap::new()).unwrap();
        let list = result.as_list().unwrap();
        assert_eq!(list.len(), 1);

        let args_close = vec![pool_val];
        close(&args_close, &HashMap::new()).unwrap();
    }
}
