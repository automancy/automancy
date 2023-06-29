#!/bin/sh

path=$path || exit
out_path=$out_path || exit

#   vvv doesn't exist   or         vvv older than
if [ ! -f "$out_path" ] || [ -n "$(find -L "$path" -prune -newer "$out_path")" ]; then
  true
else
  false
fi