use crate::type_system::{AmqpConnectionValue, Value};
use crate::{CorvoError, CorvoResult};
use lapin::{options::*, BasicProperties, Connection, ConnectionProperties};
use std::collections::HashMap;
use std::sync::Arc;

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

    match &args[0] {
        Value::AmqpConnection(c) => {
            let rt = &c.0;
            let conn = &c.1;
            rt.block_on(async {
                let channel = conn.create_channel().await.map_err(|e| {
                    CorvoError::runtime(format!("Failed to create AMQP channel: {}", e))
                })?;
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
                    .map_err(|e| {
                        CorvoError::runtime(format!("Failed to publish message: {}", e))
                    })?;
                Ok(Value::Null)
            })
        }
        Value::String(url) => {
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
                let channel = conn.create_channel().await.map_err(|e| {
                    CorvoError::runtime(format!("Failed to create AMQP channel: {}", e))
                })?;
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
                    .map_err(|e| {
                        CorvoError::runtime(format!("Failed to publish message: {}", e))
                    })?;
                let _ = conn.close(0, "").await;
                Ok(Value::Null)
            })
        }
        _ => Err(CorvoError::r#type(
            "amqp.publish expects an AMQP connection or URL string as the first argument",
        )),
    }
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

    match &args[0] {
        Value::AmqpConnection(c) => {
            let rt = &c.0;
            let conn = &c.1;
            rt.block_on(async {
                let channel = conn.create_channel().await.map_err(|e| {
                    CorvoError::runtime(format!("Failed to create AMQP channel: {}", e))
                })?;
                let count = channel
                    .queue_delete(queue, QueueDeleteOptions::default())
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Failed to delete queue: {}", e)))?;
                Ok(Value::Number(count as f64))
            })
        }
        Value::String(url) => {
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
                let channel = conn.create_channel().await.map_err(|e| {
                    CorvoError::runtime(format!("Failed to create AMQP channel: {}", e))
                })?;
                let count = channel
                    .queue_delete(queue, QueueDeleteOptions::default())
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Failed to delete queue: {}", e)))?;
                let _ = conn.close(0, "").await;
                Ok(Value::Number(count as f64))
            })
        }
        _ => Err(CorvoError::r#type(
            "amqp.queue_delete expects an AMQP connection or URL string as the first argument",
        )),
    }
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

    match &args[0] {
        Value::AmqpConnection(c) => {
            let rt = &c.0;
            let conn = &c.1;
            rt.block_on(async {
                let channel = conn.create_channel().await.map_err(|e| {
                    CorvoError::runtime(format!("Failed to create AMQP channel: {}", e))
                })?;
                let count = channel
                    .queue_purge(queue, QueuePurgeOptions::default())
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Failed to purge queue: {}", e)))?;
                Ok(Value::Number(count as f64))
            })
        }
        Value::String(url) => {
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
                let channel = conn.create_channel().await.map_err(|e| {
                    CorvoError::runtime(format!("Failed to create AMQP channel: {}", e))
                })?;
                let count = channel
                    .queue_purge(queue, QueuePurgeOptions::default())
                    .await
                    .map_err(|e| CorvoError::runtime(format!("Failed to purge queue: {}", e)))?;
                let _ = conn.close(0, "").await;
                Ok(Value::Number(count as f64))
            })
        }
        _ => Err(CorvoError::r#type(
            "amqp.queue_purge expects an AMQP connection or URL string as the first argument",
        )),
    }
}
