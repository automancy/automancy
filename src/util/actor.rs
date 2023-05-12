use ractor::concurrency::MpscSender;
use ractor::{concurrency, ActorCell, Message, MessagingErr};

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
