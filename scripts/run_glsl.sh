#!/bin/sh

path=$1
base=${path%.*}
out_path="${base}_$2.spv"

if . ./file_check.sh; then
  echo "Input: $path"
  echo "Output: $out_path"
  echo

  glslc "$1" -o "$out_path"
fi