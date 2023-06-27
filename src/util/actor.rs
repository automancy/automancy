use std::time::Duration;

use ractor::rpc::CallResult;
use ractor::{concurrency, ActorRef, Message, MessagingErr, RpcReplyPort};

pub async fn multi_call_iter<TMessage, TReply, TMsgBuilder>(
    actors: impl Iterator<Item = &ActorRef<TMessage>>,
    len: usize,
    msg_builder: TMsgBuilder,
    timeout_option: Option<Duration>,
) -> Result<Vec<CallResult<TReply>>, MessagingErr<TMessage>>
where
    TMessage: Message,
    TReply: Send + 'static,
    TMsgBuilder: Fn(RpcReplyPort<TReply>) -> TMessage,
{
    let mut rx_ports = Vec::with_capacity(len);
    // send to all actors
    for actor in actors {
        let (tx, rx) = concurrency::oneshot();
        let port: RpcReplyPort<TReply> = match timeout_option {
            Some(duration) => (tx, duration).into(),
            None => tx.into(),
        };
        actor.cast(msg_builder(port))?;
        rx_ports.push(rx);
    }

    let mut join_set = tokio::task::JoinSet::new();
    for (i, rx) in rx_ports.into_iter().enumerate() {
        if let Some(duration) = timeout_option {
            join_set.spawn(async move {
                (
                    i,
                    match tokio::time::timeout(duration, rx).await {
                        Ok(Ok(result)) => CallResult::Success(result),
                        Ok(Err(_send_err)) => CallResult::SenderError,
                        Err(_) => CallResult::Timeout,
                    },
                )
            });
        } else {
            join_set.spawn(async move {
                (
                    i,
                    match rx.await {
                        Ok(result) => CallResult::Success(result),
                        Err(_send_err) => CallResult::SenderError,
                    },
                )
            });
        }
    }

    // we threaded the index in order to maintain ordering from the originally called
    // actors.
    let mut results = Vec::new();
    results.resize_with(join_set.len(), || CallResult::Timeout);
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((i, r)) => results[i] = r,
            _ => return Err(MessagingErr::ChannelClosed),
        }
    }

    // wait for the replies
    Ok(results)
}
