# ODB++ Viewer

Browser viewer for ODB++ PCB jobs (`.tgz`), 2D and 3D. The whole job is parsed and
tessellated in WebAssembly; nothing is uploaded anywhere.

```bash
npm install
npm run dev          # http://localhost:5173
npm run build        # rebuilds the wasm, then bundles
node test.mjs        # archive + path-resolution check against designodb_rigidflex.tgz
```

Drop a `.tgz` on the page, or pass `?job=/some/job.tgz` to load one from the server.

## How it works

| | |
|---|---|
| `src/tar.js` | gunzip + untar via the browser's native `DecompressionStream` |
| `src/job.js` | `matrix/matrix` → layer list, colours, Z stack |
| `wasm/src/lib.rs` | ODB++ `features` → triangles, via [lyon](https://github.com/nical/lyon) |
| `src/view.js` | one three.js scene; orthographic camera = 2D, perspective = 3D |

The wasm has no bindgen layer — it exports `alloc`/`tessellate`/`vptr` as plain
`extern "C"` and JS copies the vertex and index buffers out.

Reference: 1.9 M triangles for all 43 layers of `designodb_rigidflex` in ~340 ms.

## Deploying to GitHub Pages

Push to `main`; `.github/workflows/deploy.yml` builds the wasm, bundles with Vite, and
publishes `dist/` via GitHub Pages. One-time repo setup:

1. **Settings → Pages → Source: GitHub Actions.**
2. Push to `main` — the workflow builds and deploys automatically.

`vite.config.js` sets `base: '/odbviewer/'` to match this repo's Pages URL
(`<user>.github.io/odbviewer/`). Renaming the repo, or deploying under a custom domain
or `<user>.github.io` user page, means updating that `base` to match (`'/'` for the
latter two).

## Known limits

- **Stackup is nominal.** The sample job ships no stackup file, so 3D layer
  thicknesses come from `THICK` in `src/job.js` (0.035 mm copper, 0.12 mm
  dielectric, …). Tune those, and the Z-scale slider, against the real board.
- **Components are not drawn.** They live in each component layer's `components`
  file, not `features`, so those two layers render empty.
- Layers are flat planes in 3D, not extruded solids; drills sit at their start layer.
- Rectangular apertures dragged along a line are stroked as square caps of the
  narrow side rather than a true swept outline.
- No text (`T`) records, negative polarity, or per-file compressed `features`
  — none occur in the reference job.
