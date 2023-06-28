#!/bin/bash

run_svg() {
  path=$1
  base=${path%.*}
  out_path="$base.blend"

  #    vvvv doesn't exist or               vvv older than
  if [ ! -f "$out_path" ] || [ "$out_path" -ot "$path" ]; then
    blender --background --python export_svg.py -- "$path" "$out_path"
  fi
}

run_blender() {
  path=$1
  base=${path%.*}
  out_path="$base.ply"

  #    vvvv doesn't exist or               vvv older than
  if [ ! -f "$out_path" ] || [ "$out_path" -ot "$path" ]; then
    blender "$path" --background --python export_blender.py -- "$out_path"
  fi
}

export -f run_svg
export -f run_blender

find ./resources -name "**.svg" | parallel run_svg {}
find ./resources -name "**.blend" | parallel run_blender {}
