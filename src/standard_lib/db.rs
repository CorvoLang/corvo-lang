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

/// Byte before `i` that is not ASCII whitespace, if any.
fn last_non_ws_byte_before(bytes: &[u8], i: usize) -> Option<u8> {
    if i == 0 {
        return None;
    }
    let mut p = i - 1;
    loop {
        if !bytes[p].is_ascii_whitespace() {
            return Some(bytes[p]);
        }
        if p == 0 {
            return None;
        }
        p -= 1;
    }
}

/// PostgreSQL uses numbered placeholders (`$1`, `$2`). Corvo passes SQLite-style `?`
/// bind markers. Rewrite only `?` that appear in SQL code (not inside string literals,
/// quoted identifiers, comments, dollar-quoted bodies, or `$n` parameter tokens).
///
/// Preserves JSONB operators `?|`, `?&`, and `??`, and treats `expr ? '...'` as the
/// key-exists operator when the previous non-whitespace byte is not `=` (bind markers
/// typically follow `=`).
fn postgres_rewrite_placeholders(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0usize;
    let mut n = 0usize;

    while i < bytes.len() {
        // `--` line comment (copy substring to preserve UTF-8)
        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            out.push_str("--");
            i += 2;
            let body_start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&sql[body_start..i]);
            continue;
        }
        // `/* */` block comment (first `*/` closes)
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            out.push_str("/*");
            i += 2;
            let body_start = i;
            let mut closed = false;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    out.push_str(&sql[body_start..i]);
                    out.push_str("*/");
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                out.push_str(&sql[body_start..]);
                i = bytes.len();
            }
            continue;
        }
        // Single-quoted string literal (`''` escape)
        if bytes[i] == b'\'' {
            out.push('\'');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                        out.push_str("''");
                        i += 2;
                        continue;
                    }
                    out.push('\'');
                    i += 1;
                    break;
                }
                // UTF-8 safe copy
                let ch = sql[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
            continue;
        }
        // Double-quoted identifier (`""` escape)
        if bytes[i] == b'"' {
            out.push('"');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'"' {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                        out.push_str("\"\"");
                        i += 2;
                        continue;
                    }
                    out.push('"');
                    i += 1;
                    break;
                }
                let ch = sql[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
            continue;
        }
        // Dollar-quoted string (`$$...$$`, `$tag$...$tag$`). Not `$1` parameters.
        if bytes[i] == b'$' {
            if let Some(end) = skip_dollar_quoted(bytes, i) {
                out.push_str(&sql[i..end]);
                i = end;
                continue;
            }
            // Copy `$123` parameter references or stray `$`
            out.push('$');
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                out.push(char::from(bytes[i]));
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'?' {
            // PostgreSQL JSON/JSONB/key operators (not bind placeholders)
            if bytes.get(i + 1) == Some(&b'|') || bytes.get(i + 1) == Some(&b'&') {
                out.push('?');
                out.push(char::from(bytes[i + 1]));
                i += 2;
                continue;
            }
            if bytes.get(i + 1) == Some(&b'?') {
                out.push_str("??");
                i += 2;
                continue;
            }
            // `expr ? 'key'` JSONB key-exists: `?` is an operator, not `?` bind syntax
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len()
                && bytes[j] == b'\''
                && last_non_ws_byte_before(bytes, i) != Some(b'=')
            {
                out.push('?');
                i += 1;
                continue;
            }
            n += 1;
            out.push('$');
            out.push_str(&n.to_string());
            i += 1;
            continue;
        }
        let ch = sql[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// If `bytes[start]` begins a valid PostgreSQL dollar-quoted literal, returns the
/// exclusive end index; otherwise `None` (caller should treat `$` as ordinary).
fn skip_dollar_quoted(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'$') {
        return None;
    }
    // `$n` references are parameters, not dollar quotes
    if bytes.get(start + 1).is_some_and(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut j = start + 1;
    while j < bytes.len() && bytes[j] != b'$' {
        let c = bytes[j];
        if !(c.is_ascii_alphanumeric() || c == b'_') {
            return None;
        }
        j += 1;
    }
    if j >= bytes.len() {
        return None;
    }
    let delim_len = j - start + 1;
    let mut k = j + 1;
    while k + delim_len <= bytes.len() {
        if bytes[k..k + delim_len] == bytes[start..start + delim_len] {
            return Some(k + delim_len);
        }
        k += 1;
    }
    None
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
    fn test_classify_db_url_errors() {
        assert!(classify_db_url("").is_err());
        assert!(classify_db_url("   ").is_err());
        assert!(classify_db_url("mysql://localhost/x").is_err());
        assert!(classify_db_url("jdbc:postgres://x").is_err());
    }

    #[test]
    fn test_classify_db_url_ok() {
        assert!(matches!(
            classify_db_url("sqlite::memory:").unwrap(),
            DbScheme::Sqlite
        ));
        assert!(matches!(
            classify_db_url(" postgres://x").unwrap(),
            DbScheme::Postgres
        ));
        assert!(matches!(
            classify_db_url("postgresql://x").unwrap(),
            DbScheme::Postgres
        ));
    }

    #[test]
    fn test_postgres_rewrite_utf8_in_comments() {
        let s = "SELECT ? -- éclair ?\nWHERE x = ?";
        assert_eq!(
            postgres_rewrite_placeholders(s),
            "SELECT $1 -- éclair ?\nWHERE x = $2"
        );
    }

    #[test]
    fn test_postgres_rewrite_jsonb_contains_vs_bind() {
        assert_eq!(
            postgres_rewrite_placeholders("SELECT * FROM t WHERE meta ? 'k' AND id = ?"),
            "SELECT * FROM t WHERE meta ? 'k' AND id = $1"
        );
    }

    #[test]
    fn test_postgres_rewrite_jsonb_key_ops() {
        let s = "SELECT * FROM t WHERE j ?| '{a,b}' AND z = ?";
        assert_eq!(
            postgres_rewrite_placeholders(s),
            "SELECT * FROM t WHERE j ?| '{a,b}' AND z = $1"
        );
        let s2 = "SELECT * FROM t WHERE j ?& '{a,b}' AND z = ?";
        assert_eq!(
            postgres_rewrite_placeholders(s2),
            "SELECT * FROM t WHERE j ?& '{a,b}' AND z = $1"
        );
    }

    #[test]
    fn test_postgres_rewrite_double_question_jsonpath() {
        assert_eq!(
            postgres_rewrite_placeholders("SELECT jsonb_path_exists(j, '??') AND v = ?"),
            "SELECT jsonb_path_exists(j, '??') AND v = $1"
        );
    }

    #[test]
    fn test_postgres_rewrite_skips_literal_and_comment() {
        let s = "SELECT 'a?b' FROM t WHERE x = ? -- ? \nAND y = ?";
        assert_eq!(
            postgres_rewrite_placeholders(s),
            "SELECT 'a?b' FROM t WHERE x = $1 -- ? \nAND y = $2"
        );
    }

    #[test]
    fn test_postgres_rewrite_block_comment_and_dollar() {
        let s = "/* ? */ SELECT $$?$$, ?";
        assert_eq!(postgres_rewrite_placeholders(s), "/* ? */ SELECT $$?$$, $1");
    }

    #[test]
    fn test_postgres_rewrite_double_quoted() {
        let s = r#"SELECT "col?" FROM t WHERE z = ?"#;
        assert_eq!(
            postgres_rewrite_placeholders(s),
            r#"SELECT "col?" FROM t WHERE z = $1"#
        );
    }

    #[test]
    fn test_postgres_rewrite_dollar_param_and_bind() {
        // Postgres `$n` placeholders are copied as-is; `?` is renumbered independently.
        let s = r#"SELECT * FROM t WHERE id = ? AND lbl = '$1'"#;
        assert_eq!(
            postgres_rewrite_placeholders(s),
            r#"SELECT * FROM t WHERE id = $1 AND lbl = '$1'"#
        );
    }

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
