use winit::event::{DeviceEvent, WindowEvent};

use crate::{
    game::data::id::Comparable,
    math::data::{Num, Vector2},
};

pub trait TickListener: Comparable {
    fn on_tick(&mut self);
}

pub trait EventListener: Comparable {
    fn on_event(&mut self, window_event: &Option<WindowEvent>, device_event: &Option<DeviceEvent>);
}

pub trait ScrollListener: Comparable {
    fn on_scroll(&mut self, delta: Vector2);
}

pub trait MainHoldListener: Comparable {
    fn on_holding_main(&mut self, elapsed: Num);
    fn on_not_holding_main(&mut self);
}

pub trait MainClickListener: Comparable {
    fn on_clicking_main(&mut self);
}

pub trait MainMoveListener: Comparable {
    fn on_moving_main(&mut self, delta: Vector2);
}

pub trait MoveListener: Comparable {
    fn on_moving(&mut self, delta: Vector2);
}
