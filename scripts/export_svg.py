import bmesh
import bpy
import sys
import xml.etree.ElementTree as ET

def main():
    src = sys.argv[-2]
    print(src)

    dst = sys.argv[-1]

    for obj in bpy.data.objects:
        bpy.data.objects.remove(obj)

    tree = ET.parse(src)
    root = tree.getroot()

    attribs = [path.attrib for path in root.iter('{http://www.w3.org/2000/svg}path')]
    ids = dict(map(lambda e: (e[1], e[0]), enumerate(map(lambda a: a['id'], attribs))))
    total = float(len(attribs))

    bpy.ops.import_curve.svg(filepath=src)

    curves = list(filter(lambda o: o.type == 'CURVE', bpy.data.objects))

    for obj in curves:
        mesh = bpy.data.meshes.new_from_object(obj)

        new_obj = bpy.data.objects.new(obj.name, mesh)

        new_obj.matrix_world = obj.matrix_world
        new_obj.delta_location.z = (ids[obj.name] / total) / 32.0 + 0.01
        #alpha = styles[obj.name].get('fill-opacity')
        #if alpha:
        #    new_obj.active_material.diffuse_color[3] = float(alpha)
        bpy.context.collection.objects.link(new_obj)
        bpy.data.objects.remove(obj)

        new_dim = new_obj.dimensions.copy()
        new_dim.x = new_dim.x / 8.0
        new_dim.y = new_dim.y / 8.0
        new_obj.dimensions = new_dim

        bpy.context.view_layer.objects.active = new_obj
        bpy.ops.object.mode_set(mode='EDIT')
        bm = bmesh.from_edit_mesh(bpy.context.edit_object.data)
        for v in bm.faces:
            v.select = True
        for v in bm.edges:
            v.select = True
        for v in bm.verts:
            v.select = True

        bmesh.ops.translate(bm, vec=(-1.0, -1.0, 0.0), space=bpy.context.object.matrix_world, verts=bm.verts)
        bpy.ops.mesh.remove_doubles(threshold=0.05)

        bmesh.update_edit_mesh(bpy.context.edit_object.data)
        bpy.ops.object.mode_set(mode='OBJECT')

    bpy.ops.wm.save_mainfile(filepath=dst)


if __name__ == "__main__":
    main()
