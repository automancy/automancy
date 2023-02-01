import bpy
import sys

# Doc can be found here: https://docs.blender.org/api/current/bpy.ops.export_mesh.html
bpy.ops.export_mesh.ply(filepath=sys.argv[-1], use_normals=False, use_uv_coords=False)
