use ractor::concurrency::{Duration, MpscSender};
use ractor::rpc::CallResult;
use ractor::{concurrency, ActorCell, Message, MessagingErr, RpcReplyPort};

pub async fn call_multi<TMessage, TReply, TMsgBuilder>(
    actor: &ActorCell,
    msg_builder: TMsgBuilder,
    size: usize,
) -> Result<Vec<TReply>, MessagingErr<TMessage>>
where
    TMessage: Message,
    TMsgBuilder: FnOnce(MpscSender<TReply>) -> Vec<TMessage>,
{
    let (tx, mut rx) = concurrency::mpsc_bounded(size);
    let msgs = msg_builder(tx);

    for msg in msgs {
        actor.send_message::<TMessage>(msg)?;
    }

    let mut replies = Vec::with_capacity(size);

    // wait for the reply
    while let Some(result) = rx.recv().await {
        replies.push(result);
    }

    Ok(replies)
}

pub async fn multi_call<TMessage, TReply, TMsgBuilder>(
    actors: &[ActorCell],
    msg_builder: TMsgBuilder,
    timeout_option: Option<Duration>,
) -> Result<Vec<CallResult<TReply>>, MessagingErr<TMessage>>
where
    TMessage: Message,
    TReply: Send + 'static,
    TMsgBuilder: Fn(RpcReplyPort<TReply>) -> TMessage,
{
    let mut rx_ports = Vec::with_capacity(actors.len());
    // send to all actors
    for actor in actors {
        let (tx, rx) = concurrency::oneshot();
        let port: RpcReplyPort<TReply> = match timeout_option {
            Some(duration) => (tx, duration).into(),
            None => tx.into(),
        };
        actor.send_message::<TMessage>(msg_builder(port))?;
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
