#!/bin/sh

path=$1
base=${path%.*}
out_path="$base.blend"

if . ./file_check.sh; then
  blender --background --python export_svg.py -- "$path" "$out_path"
fi