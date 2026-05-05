use crate::type_system::{AmqpConnectionValue, Value};
use crate::{CorvoError, CorvoResult};
use lapin::{options::*, BasicProperties, Connection, ConnectionProperties};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

async fn run_with_channel<F, Fut, R>(conn: &Connection, f: F) -> CorvoResult<R>
where
    F: FnOnce(lapin::Channel) -> Fut,
    Fut: Future<Output = CorvoResult<R>>,
{
    let channel = conn
        .create_channel()
        .await
        .map_err(|e| CorvoError::runtime(format!("Failed to create AMQP channel: {}", e)))?;
    f(channel).await
}

fn with_amqp_connection<F, Fut, R>(arg: &Value, name: &str, f: F) -> CorvoResult<R>
where
    F: FnOnce(lapin::Channel) -> Fut + Send,
    Fut: Future<Output = CorvoResult<R>> + Send,
    R: Send,
{
    match arg {
        Value::AmqpConnection(c) => {
            let rt = &c.0;
            let conn = &c.1;
            if tokio::runtime::Handle::try_current().is_ok() {
                std::thread::scope(|s| {
                    s.spawn(|| rt.block_on(run_with_channel(conn, f)))
                        .join()
                        .unwrap_or_else(|_| {
                            Err(CorvoError::runtime("Thread panicked during AMQP operation"))
                        })
                })
            } else {
                rt.block_on(run_with_channel(conn, f))
            }
        }
        Value::String(url) => {
            if tokio::runtime::Handle::try_current().is_ok() {
                std::thread::scope(|s| {
                    s.spawn(|| {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| {
                                CorvoError::runtime(format!("Failed to build tokio runtime: {}", e))
                            })?;
                        rt.block_on(async {
                            let conn = Connection::connect(url, ConnectionProperties::default())
                                .await
                                .map_err(|e| {
                                    CorvoError::runtime(format!(
                                        "Failed to connect to AMQP broker: {}",
                                        e
                                    ))
                                })?;
                            let result = run_with_channel(&conn, f).await;
                            let _ = conn.close(0, "").await;
                            result
                        })
                    })
                    .join()
                    .unwrap_or_else(|_| {
                        Err(CorvoError::runtime("Thread panicked during AMQP operation"))
                    })
                })
            } else {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        CorvoError::runtime(format!("Failed to build tokio runtime: {}", e))
                    })?;

                rt.block_on(async {
                    let conn = Connection::connect(url, ConnectionProperties::default())
                        .await
                        .map_err(|e| {
                            CorvoError::runtime(format!("Failed to connect to AMQP broker: {}", e))
                        })?;
                    let result = run_with_channel(&conn, f).await;
                    let _ = conn.close(0, "").await;
                    result
                })
            }
        }
        _ => Err(CorvoError::r#type(format!(
            "{} expects an AMQP connection or URL string as the first argument",
            name
        ))),
    }
}

pub fn connect(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() || args.len() > 1 {
        return Err(CorvoError::runtime(
            "amqp.connect expects 1 argument: (url)",
        ));
    }

    let url = args[0]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("amqp.connect expects URL as string"))?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CorvoError::runtime(format!("Failed to build tokio runtime: {}", e)))?;

    let conn = rt
        .block_on(async { Connection::connect(url, ConnectionProperties::default()).await })
        .map_err(|e| CorvoError::runtime(format!("Failed to connect to AMQP broker: {}", e)))?;

    Ok(Value::AmqpConnection(Box::new(AmqpConnectionValue(
        Arc::new(rt),
        Arc::new(conn),
    ))))
}

pub fn publish(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() != 4 {
        return Err(CorvoError::runtime(
            "amqp.publish expects 4 arguments: (connection_or_url, exchange, routing_key, body)",
        ));
    }

    let exchange = args[1]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("amqp.publish expects exchange as string"))?;
    let routing_key = args[2]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("amqp.publish expects routing_key as string"))?;
    let body = args[3]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("amqp.publish expects body as string"))?;

    with_amqp_connection(&args[0], "amqp.publish", |channel| async move {
        channel
            .basic_publish(
                exchange,
                routing_key,
                BasicPublishOptions::default(),
                body.as_bytes(),
                BasicProperties::default(),
            )
            .await
            .map_err(|e| CorvoError::runtime(format!("Failed to publish message: {}", e)))?
            .await
            .map_err(|e| CorvoError::runtime(format!("Failed to publish message: {}", e)))?;
        Ok(Value::Boolean(true))
    })
}

pub fn queue_delete(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() != 2 {
        return Err(CorvoError::runtime(
            "amqp.queue_delete expects 2 arguments: (connection_or_url, queue_name)",
        ));
    }

    let queue = args[1]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("amqp.queue_delete expects queue name as string"))?;

    with_amqp_connection(&args[0], "amqp.queue_delete", |channel| async move {
        let count = channel
            .queue_delete(queue, QueueDeleteOptions::default())
            .await
            .map_err(|e| CorvoError::runtime(format!("Failed to delete queue: {}", e)))?;
        Ok(Value::Number(count as f64))
    })
}

pub fn queue_purge(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() != 2 {
        return Err(CorvoError::runtime(
            "amqp.queue_purge expects 2 arguments: (connection_or_url, queue_name)",
        ));
    }

    let queue = args[1]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("amqp.queue_purge expects queue name as string"))?;

    with_amqp_connection(&args[0], "amqp.queue_purge", |channel| async move {
        let count = channel
            .queue_purge(queue, QueuePurgeOptions::default())
            .await
            .map_err(|e| CorvoError::runtime(format!("Failed to purge queue: {}", e)))?;
        Ok(Value::Number(count as f64))
    })
}
