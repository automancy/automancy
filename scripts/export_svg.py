import bmesh
import bpy
import sys


def main():
    src = sys.argv[-2]
    dst = sys.argv[-1]

    for obj in bpy.data.objects:
        bpy.data.objects.remove(obj)

    bpy.ops.import_curve.svg(filepath=src)

    curves = list(filter(lambda o: o.type == 'CURVE', bpy.data.objects))

    for idx, obj in enumerate(curves):
        mesh = bpy.data.meshes.new_from_object(obj)
        new_obj = bpy.data.objects.new(obj.name, mesh)
        new_obj.matrix_world = obj.matrix_world
        new_obj.delta_location.z = (1.0 - float(idx) / len(curves)) / 16.0
        bpy.context.collection.objects.link(new_obj)
        bpy.data.objects.remove(obj)

        new_dim = new_obj.dimensions.copy()
        new_dim.x = new_dim.x / 16.0
        new_dim.y = new_dim.y / 16.0
        new_obj.dimensions = new_dim
        new_obj.rotation_euler.y = 3.141593
        new_obj.location.xy = 0.5, -0.5

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
        bpy.ops.mesh.remove_doubles(threshold=0.38)
        bpy.ops.mesh.flip_normals()

        bmesh.update_edit_mesh(bpy.context.edit_object.data)

        # bpy.ops.transform.mirror(orient_matrix=mathutils.Matrix.Rotation(3.141593, 3, 'Y'))

        bpy.ops.object.mode_set(mode='OBJECT')

    bpy.ops.wm.save_mainfile(filepath=dst)


if __name__ == "__main__":
    main()
