# [svg-alchemy](https://pavelverigo.github.io/svg-alchemy/)

Needed tool to tessellate simple SVG into triangle mesh, that will be consumed by other applications.

Build upon usvg (https://github.com/RazrFalcon/resvg) for parsing SVG file, and lyon (https://github.com/nical/lyon) for tessellation.
Implementation ideas partially taken from (https://github.com/nical/lyon/tree/master/examples/wgpu_svg).

Note: only a simple subset SVG could be processed.

# Build

```
wasm-pack build --target web --no-typescript --no-pack --dev
```

# Serve locally

```
python3 -m http.server
```
