import bpy
import sys


def main():
    dst = sys.argv[-1]

    bpy.context.active_object.select_set(False)

    for obj in filter(lambda o: o.type == 'MESH', bpy.data.objects):
        mesh = obj.data
        material = obj.active_material

        if mesh.is_editmode:
            bpy.ops.object.editmode_toggle()

        obj.select_set(True)
        bpy.ops.transform.rotate(value=-3.1415926536, orient_axis='Z')
        obj.select_set(False)

        _triangulate = obj.modifiers.new(name='Triangulate', type='TRIANGULATE')
        edge_split = obj.modifiers.new(name='EdgeSplit', type='EDGE_SPLIT')
        edge_split.split_angle = 0.0

        if not mesh.color_attributes and material:
            mesh.color_attributes.new(name='Col', type='BYTE_COLOR', domain='CORNER')

            color = material.diffuse_color

            for datum in mesh.attributes.active_color.data:
                datum.color = color

    bpy.ops.export_scene.gltf(filepath=dst, check_existing=False, export_format='GLTF_EMBEDDED',
                              export_image_format='NONE', export_texcoords=False, export_materials='NONE',
                              export_apply=True, export_skins=False, export_lights=False, export_yup=False,
                              will_save_settings=False)


if __name__ == "__main__":
    main()
