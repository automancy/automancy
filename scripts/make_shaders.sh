#!/bin/sh

find "$DIR"/automancy_defs/shaders -name "**.vert" | parallel . ./run_glsl.sh {} "vert"
find "$DIR"/automancy_defs/shaders -name "**.frag" | parallel . ./run_glsl.sh {} "frag"
