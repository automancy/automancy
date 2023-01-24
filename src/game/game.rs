use std::time::Instant;

use riker::actors::{Actor, Context, Sender, ActorFactoryArgs};
use crate::game::data::map::{RenderContext};

use crate::game::ticking::{MAX_ALLOWED_TICK_INTERVAL};

use super::{data::map::Map};



#[derive(Debug, Clone)]
pub struct Ticked;



#[derive(Debug, Clone, Copy)]
pub struct GameState {
    pub tick_count: usize,
}

pub struct Game {
    tick_count: usize,

    map: Map,
}

impl ActorFactoryArgs<Map> for Game { // TODO dont clone Map
    fn create_args(args: Map) -> Self {
        Self::new(args)
    }
}

#[derive(Debug, Clone)]
pub enum GameMsg {
    Tick,
    RenderInfoRequest {
        context: RenderContext,
    },
}

impl Actor for Game {
    type Msg = GameMsg;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        match msg {
            GameMsg::Tick { .. } => {
                let start = Instant::now();
                self.tick();
                let finish = Instant::now();

                let tick_time = finish - start;

                if tick_time >= MAX_ALLOWED_TICK_INTERVAL {
                    log::warn!(
                        "tick took longer than allowed maximum! tick_time: {:?}, maximum: {:?}",
                        tick_time,
                        MAX_ALLOWED_TICK_INTERVAL
                    );
                }
            }
            GameMsg::RenderInfoRequest { context } => {
                let render_info = self.map.render_info(&context);

                sender.inspect(|v| v.try_tell(render_info, Some(ctx.myself().into())).unwrap());
            }
        }
    }
}

impl Game {
    pub fn new(
        map: Map,
    ) -> Self {
         Self {
            tick_count: 0,

            map,
        }
    }

    fn tick(&mut self) {
        self.tick_count = self.tick_count.overflowing_add(1).0;
    }
}
