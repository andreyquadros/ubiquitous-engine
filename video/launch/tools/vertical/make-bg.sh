#!/bin/bash
# Ambient background for the 9:16 cut: the 16:9 film scaled to cover 1080x1920, heavily blurred and
# darkened, so the vertical frame around the sharp panels carries the same colours and motion.
set -euo pipefail
cd "$(dirname "$0")/../.."
ffmpeg -loglevel error -y -i public/vertical/film.mp4 \
  -vf "scale=-2:1920:flags=bicubic,crop=1080:1920,boxblur=luma_radius=60:luma_power=3:chroma_radius=60:chroma_power=3,eq=brightness=0.0:saturation=1.3" \
  -an -c:v libx264 -preset veryfast -crf 26 -pix_fmt yuv420p public/vertical/bg.mp4
echo "wrote public/vertical/bg.mp4"
