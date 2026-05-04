use std::future::Future;

fn with_amqp_connection<F, Fut, R>(arg: &crate::type_system::Value, name: &str, f: F) -> crate::CorvoResult<R>
where
    F: FnOnce(lapin::Channel) -> Fut,
    Fut: Future<Output = crate::CorvoResult<R>>,
{
    unimplemented!()
}
