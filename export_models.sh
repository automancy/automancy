#!/bin/bash

run_blender() {
  path=$1
  blender "$path" --background --python export_models.py -- "${path%.*}.ply"
}

export -f run_blender

find ./resources -name "**.blend" -exec bash -c 'run_blender "$0"' {} \;