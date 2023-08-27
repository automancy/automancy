import bmesh
import bpy
import sys
import xml.etree.ElementTree as ET

from frozendict import frozendict


def main():
    src = sys.argv[-2]
    print(src)

    dst = sys.argv[-1]

    for obj in bpy.data.objects:
        bpy.data.objects.remove(obj)

    tree = ET.parse(src)
    root = tree.getroot()

    paths = [tuple([path.attrib['id'],
                    frozendict(map(lambda x: tuple(x.split(':')), path.attrib['style'].split(';')))
                    ])
             for path in root.iter('{http://www.w3.org/2000/svg}path')]
    total = float(len(paths))
    ids = dict(map(lambda e: (e[1][0], e[0]), enumerate(paths)))
    styles = dict(map(lambda e: (e[0], e[1]), paths))

    offset = 1.0 - 1.0 / total

    bpy.ops.import_curve.svg(filepath=src)

    curves = list(filter(lambda o: o.type == 'CURVE', bpy.data.objects))

    for obj in curves:
        mesh = bpy.data.meshes.new_from_object(obj)

        new_obj = bpy.data.objects.new(obj.name, mesh)

        new_obj.matrix_world = obj.matrix_world
        new_obj.delta_location.z = (offset + ids[obj.name] / total) / 8.0
        alpha = styles[obj.name].get('fill-opacity')
        if alpha:
            new_obj.active_material.diffuse_color[3] = float(alpha)
        bpy.context.collection.objects.link(new_obj)
        bpy.data.objects.remove(obj)

        new_dim = new_obj.dimensions.copy()
        new_dim.x = new_dim.x / 8.0
        new_dim.y = new_dim.y / 8.0
        new_obj.dimensions = new_dim
        new_obj.location.xy = 1.0, 1.0

        bpy.context.view_layer.objects.active = new_obj
        bpy.ops.object.mode_set(mode='EDIT')
        bm = bmesh.from_edit_mesh(bpy.context.edit_object.data)
        for v in bm.faces:
            v.select = True
        for v in bm.edges:
            v.select = True
        for v in bm.verts:
            v.select = True

        bpy.ops.mesh.beautify_fill()
        bpy.ops.mesh.tris_convert_to_quads(face_threshold=3.141593, shape_threshold=3.141593)
        bpy.ops.mesh.remove_doubles(threshold=0.25)
        bpy.ops.mesh.flip_normals()

        bmesh.update_edit_mesh(bpy.context.edit_object.data)

        bpy.ops.object.mode_set(mode='OBJECT')

    bpy.ops.wm.save_mainfile(filepath=dst)


if __name__ == "__main__":
    main()
