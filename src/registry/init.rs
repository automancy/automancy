use std::{
    collections::HashMap,
    ops::Range,
    sync::{Arc, Mutex},
};

use winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
};

use crate::{
    game::{
        player::control::{MainClickListener, MainHoldListener, MainMoveListener},
        render::data::{Face, Vertex},
    },
    util::resource::Resource,
};

pub struct InitData {
    pub resources: Vec<Resource>,
    pub resources_map: HashMap<&'static str, usize>,

    pub all_faces: Vec<Vec<Face>>,
    pub all_index_ranges: Vec<Vec<Range<u32>>>,
    pub combined_vertices: Vec<Vertex>,

    main_clicked: bool,
    main_last_clicked: u32,
}

impl InitData {
    pub fn tick(
        &mut self,
        window_event: Option<WindowEvent>,
        device_event: Option<DeviceEvent>,
        main_hold_listeners: &mut Vec<Arc<Mutex<dyn MainHoldListener>>>,
        main_click_listeners: &mut Vec<Arc<Mutex<dyn MainClickListener>>>,
        main_move_listeners: &mut Vec<Arc<Mutex<dyn MainMoveListener>>>,
    ) {
        let mut cursor_position = None;

        if let Some(event) = window_event {
            match event {
                WindowEvent::KeyboardInput { input, .. } => {}
                WindowEvent::ModifiersChanged(_) => {}
                WindowEvent::MouseWheel { delta, phase, .. } => {}
                WindowEvent::MouseInput { state, button, .. } => {
                    match button {
                        MouseButton::Left => {
                            self.main_clicked = state == ElementState::Pressed;
                            log::debug!("{}", self.main_clicked);
                        }
                        _ => (),
                    };
                }
                _ => (),
            }
        }

        if let Some(event) = device_event {
            match event {
                DeviceEvent::MouseMotion { delta } => cursor_position = Some(delta),
                _ => (),
            }
        }

        if self.main_clicked {
            main_click_listeners
                .into_iter()
                .for_each(|v| v.lock().unwrap().on_clicking_main());
        }

        if self.main_last_clicked > 0 {
            let elapsed = (self.main_last_clicked as f32) / 60.0; // TODO get FPS

            main_hold_listeners
                .into_iter()
                .for_each(|v| v.lock().unwrap().on_holding_main(elapsed));
        }

        if let Some(delta) = cursor_position {
            main_move_listeners
                .into_iter()
                .for_each(|v| v.lock().unwrap().on_moving_main(delta));
        }

        if self.main_clicked {
            self.main_last_clicked += 1;
        } else {
            self.main_last_clicked = 0;

            main_hold_listeners
                .into_iter()
                .for_each(|v| v.lock().unwrap().on_not_holding_main());
        }
    }

    pub fn new(mut resources: Vec<(&'static str, Resource)>) -> Self {
        let mut resources_map: HashMap<&'static str, usize> = HashMap::new();

        // register
        resources
            .iter_mut()
            .enumerate()
            .for_each(|(index, (id, r))| {
                r.register(index);
                resources_map.insert(id.clone(), index);
            });
        let resources = resources.into_iter().map(|(_, r)| r).collect::<Vec<_>>();

        // indices vertices
        let (vertices, faces): (Vec<_>, Vec<_>) = resources
            .iter()
            .map(|r| (r.mesh.vertices.clone(), r.mesh.faces.clone()))
            .unzip();

        let combined_vertices = vertices.into_iter().flatten().collect::<Vec<_>>();

        let mut all_faces = Vec::with_capacity(faces.len());

        faces.into_iter().fold(0, |offset, faces| {
            let offsetted_faces = faces
                .into_iter()
                .map(|face| {
                    let vertex_indices = face
                        .vertex_indices
                        .into_iter()
                        .map(|v| v + offset)
                        .collect::<Vec<_>>();

                    Face { vertex_indices }
                })
                .collect::<Vec<_>>();

            let new_offset = offsetted_faces
                .iter()
                .map(|v| v.vertex_indices.iter().max().unwrap_or(&0))
                .max()
                .unwrap_or(&offset)
                .to_owned();

            all_faces.push(offsetted_faces);

            new_offset + 1
        });

        let mut all_index_ranges = Vec::with_capacity(all_faces.len());
        all_faces.iter().fold(0, |start, faces| {
            let mut index_ranges = Vec::with_capacity(faces.len());

            let end = faces.iter().fold(start, |start, face| {
                let end = start + face.vertex_indices.len() as u32;

                index_ranges.push(start..end);

                end
            });
            all_index_ranges.push(index_ranges);

            end
        });

        log::debug!("all_index_ranges: {:?}", all_index_ranges);
        log::debug!("all_faces: {:?}", all_faces);

        InitData {
            resources,
            resources_map,
            all_faces,
            all_index_ranges,
            combined_vertices,

            main_clicked: false,
            main_last_clicked: 0,
        }
    }
}
