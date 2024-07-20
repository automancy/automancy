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
    styles = dict(map(lambda a: (a['id'], dict(map(lambda p: p.split(':'), (a.get('style') or 'ihate:python').split(';')))), attribs))
    total = float(len(attribs))

    bpy.ops.import_curve.svg(filepath=src)

    curves = list(filter(lambda o: o.type == 'CURVE', bpy.data.objects))

    for obj in curves:
        mesh = bpy.data.meshes.new_from_object(obj)

        new_obj = bpy.data.objects.new(obj.name, mesh)

        new_obj.matrix_world = obj.matrix_world
        new_obj.delta_location.z = (ids[obj.name] / total) / 64.0 + 0.005
        alpha = styles[obj.name].get('fill-opacity')
        if alpha:
            new_obj.active_material.diffuse_color[3] = float(alpha)
        bpy.context.collection.objects.link(new_obj)
        bpy.data.objects.remove(obj)

        edge_smallen_ratio = 0.95

        new_dim = new_obj.dimensions.copy()
        new_dim.x = (new_dim.x / 8.0) * edge_smallen_ratio
        new_dim.y = (new_dim.y / 8.0) * edge_smallen_ratio
        new_obj.dimensions = new_dim

        new_obj.delta_location.x += 1.0 - edge_smallen_ratio
        new_obj.delta_location.y += 1.0 - edge_smallen_ratio

        bpy.context.view_layer.objects.active = new_obj
        bpy.ops.object.mode_set(mode='EDIT')
        bm = bmesh.from_edit_mesh(bpy.context.edit_object.data)
        for v in bm.faces:
            v.select = True
        for v in bm.edges:
            v.select = True
        for v in bm.verts:
            v.select = True

        bmesh.ops.translate(bm, verts=bm.verts, vec=(-1.0, -1.0, 0.0), space=bpy.context.object.matrix_world)
        bmesh.ops.beautify_fill(bm, faces=bm.faces, edges=bm.edges)
        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=0.1)

        bmesh.update_edit_mesh(bpy.context.edit_object.data)
        bpy.ops.object.mode_set(mode='OBJECT')

    bpy.ops.wm.save_mainfile(filepath=dst)


if __name__ == "__main__":
    main()
