use std::{hash::Hash, time::Duration};

use hashbrown::HashMap;
use ractor::rpc::CallResult;
use ractor::{concurrency, ActorRef, Message, MessagingErr, RpcReplyPort};

pub async fn multi_call_iter<Key, TMessage, TReply, TMsgBuilder>(
    actors: &HashMap<Key, ActorRef<TMessage>>,
    msg_builder: TMsgBuilder,
    timeout_option: Option<Duration>,
) -> Result<HashMap<Key, TReply>, MessagingErr<TMessage>>
where
    Key: Hash + Eq + Send + Sync + Copy + 'static,
    TMessage: Message,
    TReply: Send + 'static,
    TMsgBuilder: Fn(RpcReplyPort<TReply>, Key) -> TMessage,
{
    let len = actors.len();

    let mut rx_ports = HashMap::with_capacity(len);
    // send to all actors
    for (k, actor) in actors {
        let (tx, rx) = concurrency::oneshot();
        let port: RpcReplyPort<TReply> = match timeout_option {
            Some(duration) => (tx, duration).into(),
            None => tx.into(),
        };
        actor.cast(msg_builder(port, *k))?;
        rx_ports.insert(*k, rx);
    }

    let mut join_set = tokio::task::JoinSet::new();
    for (k, rx) in rx_ports.into_iter() {
        if let Some(duration) = timeout_option {
            join_set.spawn(async move {
                (
                    k,
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
                    k,
                    match rx.await {
                        Ok(result) => CallResult::Success(result),
                        Err(_send_err) => CallResult::SenderError,
                    },
                )
            });
        }
    }

    let mut results = HashMap::with_capacity(len);
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((k, r)) => {
                if let CallResult::Success(r) = r {
                    results.insert(k, r);
                }
            }
            _ => return Err(MessagingErr::ChannelClosed),
        }
    }

    // wait for the replies
    Ok(results)
}
