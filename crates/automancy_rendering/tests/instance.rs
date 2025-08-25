use automancy_data::{
    id::{Id, ModelId, RenderId},
    rendering::gpu::GameDrawInstance,
};
use automancy_rendering::{
    AutomancyRenderer,
    instance::DrawInstanceManager,
    model::{GlobalMeshId, MeshId},
};

#[test]
fn test_instance_manager_insertion() {
    let mut renderer = AutomancyRenderer::default();

    const MODEL_IDS: u32 = 64;
    const MAX_MESH: u16 = 6;

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

    for _ in 0..4 {
        let mut manager = DrawInstanceManager::new();

        for _ in 0..256 {
            for _ in 0..64 {
                let model_id = ModelId(Id::from(rand::random::<u32>() % MODEL_IDS));

                manager.checked_insert(
                    &renderer,
                    RenderId(Id::from(0)),
                    model_id,
                    GameDrawInstance::default(),
                );
            }
            manager.take_changes();
        }
    }
}
