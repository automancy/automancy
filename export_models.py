import bpy
import sys

# Doc can be found here: https://docs.blender.org/api/current/bpy.ops.export_mesh.html

for object in bpy.data.objects:
    edge_split = object.modifiers.new(name="EdgeSplit", type='EDGE_SPLIT')
    edge_split.split_angle = 0.0
    object.modifiers.new(name="Triangulate", type='TRIANGULATE')

bpy.ops.export_mesh.ply(filepath=sys.argv[-1], use_uv_coords=False, axis_forward='-Y', use_ascii=True)
