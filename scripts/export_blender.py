import bmesh  # type: ignore
import bpy  # type: ignore
import sys


def main():
    dst = sys.argv[-1]

    if bpy.context.active_object:
        bpy.context.active_object.select_set(False)

    for obj in filter(lambda o: o.type == "MESH", bpy.data.objects):
        bpy.context.view_layer.objects.active = obj

        bpy.ops.object.mode_set(mode="EDIT")
        bm = bmesh.from_edit_mesh(bpy.context.edit_object.data)
        for v in bm.faces:
            v.select = True
        for v in bm.edges:
            v.select = True
        for v in bm.verts:
            v.select = True

        # post processing...
        # ...

        bmesh.update_edit_mesh(bpy.context.edit_object.data)
        bpy.ops.object.mode_set(mode="OBJECT")

        if not obj.data.color_attributes and obj.active_material:
            obj.data.color_attributes.new(
                name="Col", type="FLOAT_COLOR", domain="CORNER"
            )

            color = obj.active_material.diffuse_color

            for datum in obj.data.attributes.active_color.data:
                datum.color = color

            obj.data.materials.clear()

    bpy.ops.export_scene.gltf(
        filepath=dst,
        check_existing=False,
        export_format="GLB",
        export_image_format="NONE",
        export_texcoords=False,
        export_materials="NONE",
        export_apply=True,
        export_skins=False,
        export_lights=False,
        export_yup=False,
        will_save_settings=False,
    )


if __name__ == "__main__":
    main()
