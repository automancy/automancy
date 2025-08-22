use core::cell::RefCell;
use std::rc::Rc;

use hashbrown::HashMap;
use strum_macros::EnumDiscriminants;
use yakui::{context::dom, paint::UserPaintCallId};

use crate::GameObjectPaint;

#[derive(Debug)]
struct CustomRendererInner {
    objects: Vec<RenderObject>,

    should_rerender: bool,
}

#[derive(Debug, Clone)]
pub struct CustomRenderer {
    inner: Rc<RefCell<CustomRendererInner>>,
}

impl CustomRenderer {
    pub fn init() -> Self {
        Self {
            inner: Rc::new(RefCell::new(CustomRendererInner {
                objects: Vec::new(),
                should_rerender: true,
            })),
        }
    }

    pub fn add(&self, object: RenderObject) -> UserPaintCallId {
        let v = &mut self.inner.borrow_mut().objects;

        v.push(object);

        (v.len() - 1) as UserPaintCallId
    }
}

pub fn take_objects() -> HashMap<UserPaintCallId, RenderObject> {
    let renderer = dom().get_global_or_init(CustomRenderer::init);

    renderer.inner.borrow_mut().should_rerender = false;

    renderer
        .inner
        .borrow_mut()
        .objects
        .drain(..)
        .enumerate()
        .map(|(idx, v)| (idx as UserPaintCallId, v))
        .collect()
}

pub fn mark_rerender() {
    let renderer = dom().get_global_or_init(CustomRenderer::init);

    renderer.inner.borrow_mut().should_rerender = true;
}

pub fn should_rerender() -> bool {
    let renderer = dom().get_global_or_init(CustomRenderer::init);

    renderer.inner.borrow().should_rerender
}
