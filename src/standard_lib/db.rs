use crate::type_system::{DatabasePoolValue, Value};
use crate::{CorvoError, CorvoResult};
use sqlx::any::AnyPoolOptions;
use sqlx::{Column, Row, ValueRef};
use std::collections::HashMap;
use std::sync::{Arc, Once};

static INIT_DRIVERS: Once = Once::new();

pub fn connect(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() || args.len() > 2 {
        return Err(CorvoError::runtime(
            "db.connect expects 1 or 2 arguments: (url, [max_connections])",
        ));
    }

    let url = match &args[0] {
        Value::String(s) => s,
        _ => return Err(CorvoError::r#type("db.connect expects URL as string")),
    };

    let max_connections = if args.len() == 2 {
        match &args[1] {
            Value::Number(n) => *n as u32,
            _ => {
                return Err(CorvoError::r#type(
                    "db.connect expects max_connections as number",
                ))
            }
        }
    } else {
        10
    };

    INIT_DRIVERS.call_once(|| {
        sqlx::any::install_default_drivers();
    });

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CorvoError::runtime(format!("Failed to build tokio runtime: {}", e)))?;

    let pool = rt
        .block_on(async {
            AnyPoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await
        })
        .map_err(|e| CorvoError::runtime(format!("Failed to connect to database: {}", e)))?;

    Ok(Value::DatabasePool(Box::new(DatabasePoolValue(
        Arc::new(rt),
        Arc::new(pool),
    ))))
}

fn bind_args<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>,
    args: &'q [Value],
) -> CorvoResult<sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>> {
    for arg in args {
        match arg {
            Value::String(s) => query = query.bind(s.clone()),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    query = query.bind(*n as i64);
                } else {
                    query = query.bind(*n);
                }
            }
            Value::Boolean(b) => query = query.bind(*b),
            Value::Null => query = query.bind(None::<String>),
            _ => {
                return Err(CorvoError::r#type(
                    "Unsupported argument type for SQL query",
                ))
            }
        }
    }
    Ok(query)
}

fn extract_row(row: sqlx::any::AnyRow) -> Value {
    let mut map = HashMap::new();
    for i in 0..row.columns().len() {
        let col = row.column(i);
        let name = col.name().to_string();

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

        // Try to decode based on common types
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
            // Fallback for unsupported types
            map.insert(name, Value::String("<unsupported db type>".to_string()));
        }
    }
    Value::Map(map)
}

pub fn query(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::runtime(
            "db.query expects at least 2 arguments: (pool, sql, [args...])",
        ));
    }

    let (rt, pool) = match &args[0] {
        Value::DatabasePool(d) => (&d.0, &d.1),
        _ => {
            return Err(CorvoError::r#type(
                "db.query expects a database pool as the first argument",
            ))
        }
    };

    let sql = match &args[1] {
        Value::String(s) => s,
        _ => return Err(CorvoError::r#type("db.query expects SQL query as string")),
    };

    let query_args = &args[2..];

    rt.block_on(async {
        let q = sqlx::query(sql);
        let q = match bind_args(q, query_args) {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        let rows = q
            .fetch_all(pool.as_ref())
            .await
            .map_err(|e| CorvoError::runtime(format!("Query failed: {}", e)))?;

        let mapped_rows = rows.into_iter().map(extract_row).collect();
        Ok(Value::List(mapped_rows))
    })
}

pub fn execute(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::runtime(
            "db.execute expects at least 2 arguments: (pool, sql, [args...])",
        ));
    }

    let (rt, pool) = match &args[0] {
        Value::DatabasePool(d) => (&d.0, &d.1),
        _ => {
            return Err(CorvoError::r#type(
                "db.execute expects a database pool as the first argument",
            ))
        }
    };

    let sql = match &args[1] {
        Value::String(s) => s,
        _ => return Err(CorvoError::r#type("db.execute expects SQL query as string")),
    };

    let query_args = &args[2..];

    rt.block_on(async {
        let q = sqlx::query(sql);
        let q = match bind_args(q, query_args) {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        let result = q
            .execute(pool.as_ref())
            .await
            .map_err(|e| CorvoError::runtime(format!("Execute failed: {}", e)))?;

        Ok(Value::Number(result.rows_affected() as f64))
    })
}

pub fn close(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() != 1 {
        return Err(CorvoError::runtime("db.close expects 1 argument: (pool)"));
    }

    let (rt, pool) = match &args[0] {
        Value::DatabasePool(d) => (&d.0, &d.1),
        _ => {
            return Err(CorvoError::r#type(
                "db.close expects a database pool as the first argument",
            ))
        }
    };

    rt.block_on(async {
        pool.close().await;
        Ok(Value::Null)
    })
}
