use automancy_data::{
    id::{Id, ModelId, RenderId},
    rendering::gpu::GameDrawInstance,
};
use automancy_rendering::{
    AutomancyRenderer,
    instance::DrawInstanceManager,
    model::{GlobalMeshId, MeshId},
};
use criterion::{BatchSize, Criterion};

const MODEL_IDS: u32 = 128;
const MAX_MESH: u16 = 16;
const SAMPLES: usize = 2048;
const CLEAR_FREQ: usize = SAMPLES / 16;

pub(crate) fn bench_instance_manager_insertion_clear_changes(c: &mut Criterion) {
    let mut renderer = AutomancyRenderer::default();

    {
        let mut mesh_count = 0;

        for model_id in 0..MODEL_IDS {
            let model_id = ModelId(Id::from(model_id));

            let meshes = rand::random::<u16>() % MAX_MESH;
            for mesh_id in 0..=meshes {
                let mesh_id = MeshId::from(mesh_id);

                renderer
                    .model_meshes
                    .entry(model_id)
                    .or_default()
                    .push(mesh_id);
                renderer
                    .global_id_map
                    .insert((model_id, mesh_id), GlobalMeshId::from(mesh_count));
                mesh_count += 1;
            }
        }
    }

    c.bench_function("instance manager, with changes cleared sometimes", |b| {
        b.iter_batched(
            || {
                let mut v = vec![];

                for _ in 0..SAMPLES {
                    v.push((
                        RenderId(Id::from(0)),
                        ModelId(Id::from(rand::random::<u32>() % MODEL_IDS)),
                        GameDrawInstance::default(),
                    ));
                }

                (v, DrawInstanceManager::new())
            },
            |(v, mut manager)| {
                for (idx, (tag, model_id, instance)) in v.into_iter().enumerate() {
                    if idx % CLEAR_FREQ == 0 {
                        manager.take_changes();
                    }

                    manager.insert(&renderer, tag, model_id, instance);
                }
            },
            BatchSize::SmallInput,
        );
    });
}

pub(crate) fn bench_instance_manager_insertion_clear_state(c: &mut Criterion) {
    let mut renderer = AutomancyRenderer::default();

    {
        let mut mesh_count = 0;

        for model_id in 0..MODEL_IDS {
            let model_id = ModelId(Id::from(model_id));

            let meshes = rand::random::<u16>() % MAX_MESH;
            for mesh_id in 0..=meshes {
                let mesh_id = MeshId::from(mesh_id);

                renderer
                    .model_meshes
                    .entry(model_id)
                    .or_default()
                    .push(mesh_id);
                renderer
                    .global_id_map
                    .insert((model_id, mesh_id), GlobalMeshId::from(mesh_count));
                mesh_count += 1;
            }
        }
    }

    c.bench_function("instance manager, with state cleared sometimes", |b| {
        b.iter_batched(
            || {
                let mut v = vec![];

                for _ in 0..SAMPLES {
                    v.push((
                        RenderId(Id::from(0)),
                        ModelId(Id::from(rand::random::<u32>() % MODEL_IDS)),
                        GameDrawInstance::default(),
                    ));
                }

                (v, DrawInstanceManager::new())
            },
            |(v, mut manager)| {
                for (idx, (tag, model_id, instance)) in v.into_iter().enumerate() {
                    if idx % CLEAR_FREQ == 0 {
                        manager.clear();
                    }

                    manager.insert(&renderer, tag, model_id, instance);
                }
            },
            BatchSize::SmallInput,
        );
    });
}

pub(crate) fn bench_instance_manager_insertion(c: &mut Criterion) {
    let mut renderer = AutomancyRenderer::default();

    {
        let mut mesh_count = 0;

        for model_id in 0..MODEL_IDS {
            let model_id = ModelId(Id::from(model_id));

            let meshes = rand::random::<u16>() % MAX_MESH;
            for mesh_id in 0..=meshes {
                let mesh_id = MeshId::from(mesh_id);

                renderer
                    .model_meshes
                    .entry(model_id)
                    .or_default()
                    .push(mesh_id);
                renderer
                    .global_id_map
                    .insert((model_id, mesh_id), GlobalMeshId::from(mesh_count));
                mesh_count += 1;
            }
        }
    }

    c.bench_function("instance manager", |b| {
        b.iter_batched(
            || {
                let mut v = vec![];

                for _ in 0..SAMPLES {
                    v.push((
                        RenderId(Id::from(0)),
                        ModelId(Id::from(rand::random::<u32>() % MODEL_IDS)),
                        GameDrawInstance::default(),
                    ));
                }

                (v, DrawInstanceManager::new())
            },
            |(v, mut manager)| {
                for (tag, model_id, instance) in v.into_iter() {
                    manager.insert(&renderer, tag, model_id, instance);
                }
            },
            BatchSize::SmallInput,
        );
    });
}

pub(crate) fn bench_instance_manager_reference(c: &mut Criterion) {
    c.bench_function("instance manager, Vec reference", |b| {
        b.iter_batched(
            || {
                let mut v = vec![];

                for _ in 0..SAMPLES {
                    v.push((
                        RenderId(Id::from(0)),
                        ModelId(Id::from(rand::random::<u32>() % MODEL_IDS)),
                        GameDrawInstance::default(),
                    ));
                }

                (v, Vec::new())
            },
            |(v, mut manager)| {
                for (tag, model_id, instance) in v.into_iter() {
                    manager.push((tag, model_id, instance));
                }
            },
            BatchSize::SmallInput,
        );
    });
}
