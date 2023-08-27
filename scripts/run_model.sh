#!/bin/sh

path=$1
base=${path%.*}
out_path="$base.gltf"

if . ./file_check.sh; then
  blender "$path" --background --python export_blender.py -- "$out_path"
fi
