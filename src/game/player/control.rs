use crate::game::data::id::Comparable;

pub trait MainHoldListener: Comparable {
    fn on_holding_main(&mut self, elapsed: f32);
    fn on_not_holding_main(&mut self);
}

pub trait MainClickListener: Comparable {
    fn on_clicking_main(&mut self);
}

pub trait MainMoveListener: Comparable {
    fn on_moving_main(&mut self, delta: (f64, f64));
}

pub trait MoveListener: Comparable {
    fn on_moving(&mut self, delta: (f64, f64));
}
