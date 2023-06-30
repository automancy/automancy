import sys

import bpy


def main():
    dst = sys.argv[-1]

    # Doc can be found here: https://docs.blender.org/api/current/bpy.ops.export_mesh.html

    for obj in filter(lambda o: o.type == 'MESH', bpy.data.objects):
        edge_split = obj.modifiers.new(name='EdgeSplit', type='EDGE_SPLIT')
        edge_split.split_angle = 0.0
        obj.modifiers.new(name='Triangulate', type='TRIANGULATE')

        mesh = obj.data
        material = obj.active_material

        if not mesh.color_attributes and material:
            color_layer = mesh.color_attributes.new(name='Col', type='BYTE_COLOR', domain='CORNER')

            color = material.diffuse_color

            for poly in mesh.polygons:
                for idx in poly.loop_indices:
                    color_layer.data[idx].color = color

    bpy.ops.export_mesh.ply(filepath=dst, use_uv_coords=False, use_ascii=True, axis_forward='-Y', axis_up='Z')


if __name__ == "__main__":
    main()
