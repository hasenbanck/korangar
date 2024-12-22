#!/usr/bin/env sh
slangc -profile glsl_450 -target spirv -stage vertex -entry vs_main -o ./src/graphics/passes/interface/shader/rectangle_bindless.vs.spv ./src/graphics/passes/interface/shader/rectangle_bindless.slang
slangc -profile glsl_450 -target spirv -stage pixel -entry fs_main -o ./src/graphics/passes/interface/shader/rectangle_bindless.fs.spv ./src/graphics/passes/interface/shader/rectangle_bindless.slang
