#!/bin/sh

DIR=$PWD

cd scripts || exit

. ./make_shaders.sh
. ./make_models.sh
