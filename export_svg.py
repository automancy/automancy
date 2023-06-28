import sys

import bpy


def main():
    src = sys.argv[-2]
    dst = sys.argv[-1]

    for obj in bpy.data.objects:
        bpy.data.objects.remove(obj)

    bpy.ops.import_curve.svg(filepath=src)

    for obj in filter(lambda o: o.type == 'CURVE', bpy.data.objects):
        mesh = bpy.data.meshes.new_from_object(obj)
        new_obj = bpy.data.objects.new(obj.name, mesh)
        new_obj.matrix_world = obj.matrix_world
        bpy.context.collection.objects.link(new_obj)
        bpy.data.objects.remove(obj)

    for obj in bpy.data.objects:
        obj.dimensions.xy = 1.0, 1.0
        obj.location.xy = -0.5, -0.5

    bpy.ops.wm.save_mainfile(filepath=dst)


if __name__ == "__main__":
    main()
