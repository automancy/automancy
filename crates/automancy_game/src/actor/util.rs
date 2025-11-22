use std::{hash::Hash, time::Duration};

use hashbrown::HashMap;
use ractor::{ActorRef, Message, MessagingErr, RpcReplyPort, concurrency, rpc::CallResult};

pub async fn multi_call_iter<Key, Msg, MsgBuilder, Reply, ResultKey, ResultValue>(
    len_hint: usize,
    actors: impl Iterator<Item = (Key, ActorRef<Msg>)>,
    msg_builder: MsgBuilder,
    mut map_result: impl FnMut(Key, Reply) -> (ResultKey, ResultValue),
    timeout_option: Option<Duration>,
) -> Result<HashMap<ResultKey, ResultValue>, MessagingErr<Msg>>
where
    Key: Hash + Eq + Send + Sync + 'static,
    Msg: Message,
    MsgBuilder: Fn(&Key, RpcReplyPort<Reply>) -> Msg,
    Reply: Send + 'static,
    ResultKey: Hash + Eq + 'static,
    ResultValue: 'static,
{
    let mut rx_ports = HashMap::with_capacity(len_hint);
    // send to all actors
    for (k, actor) in actors {
        let (tx, rx) = concurrency::oneshot();
        let port: RpcReplyPort<Reply> = match timeout_option {
            Some(duration) => (tx, duration).into(),
            None => tx.into(),
        };
        actor.cast(msg_builder(&k, port))?;
        rx_ports.insert(k, rx);
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

    let mut results = HashMap::with_capacity(len_hint);

    // wait for the replies
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((k, r)) => {
                if let CallResult::Success(r) = r {
                    let (k, v) = map_result(k, r);
                    results.insert(k, v);
                }
            }
            _ => return Err(MessagingErr::ChannelClosed),
        }
    }

    Ok(results)
}
