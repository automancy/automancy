use futures::channel::mpsc;
use futures::future::RemoteHandle;
use futures::StreamExt;
use riker::actors::{Actor, Context, Props, Run, Sender, Tell};
use riker::actors::{ActorRefFactory, TmpActorRefFactory};
use riker::Message;

pub fn ask_multi<Msg, Ctx, R, T>(
    ctx: &Ctx,
    receiver: &T,
    msgs: impl Iterator<Item = Msg>,
    size: usize,
) -> RemoteHandle<Vec<R>>
where
    Msg: Message,
    R: Message,
    Ctx: TmpActorRefFactory + Run,
    T: Tell<Msg>,
{
    let (tx, rx) = mpsc::channel::<R>(size);

    let props = Props::new_from_args(AskActor::new, (tx, size));
    let actor = ctx.tmp_actor_of_props(props).unwrap();

    for msg in msgs {
        receiver.tell(msg, Some(actor.clone().into()));
    }

    ctx.run(rx.collect()).unwrap()
}

struct AskActor<Msg> {
    tx: mpsc::Sender<Msg>,
    size: usize,
    sent: usize,
}

impl<Msg: Message> AskActor<Msg> {
    fn new((tx, size): (mpsc::Sender<Msg>, usize)) -> AskActor<Msg> {
        AskActor { tx, size, sent: 0 }
    }
}

impl<Msg: Message> Actor for AskActor<Msg> {
    type Msg = Msg;

    fn recv(&mut self, ctx: &Context<Msg>, msg: Msg, _: Sender) {
        self.tx.try_send(msg).unwrap();

        self.sent += 1;
        if self.sent >= self.size {
            ctx.stop(&ctx.myself);
        }
    }
}
