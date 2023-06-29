#!/bin/sh

find "$DIR"/resources -name "**.svg"   | parallel . ./run_svg.sh {}
find "$DIR"/resources -name "**.blend" | parallel . ./run_model.sh {}
