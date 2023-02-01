#!/bin/bash

run_blender() {
  path=$1
  out_path="${path%.*}.ply"

  #    vvvv doesn't exist or               vvv older than
  if [ ! -f "$out_path" ] || [ "$out_path" -ot "$path" ]; then
    blender "$path" --background --python export_models.py -- "$out_path"
  fi
}

export -f run_blender

find ./resources -name "**.blend" | parallel run_blender {}