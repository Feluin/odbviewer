import { untar } from './tar.js'
import { initWasm, addSymbol, tessellate } from './wasm.js'

// Nominal stack thicknesses in mm. This job carries no stackup file, so the 3D
// stack is a guess — tune here (or with the Z-scale slider) against the real board.
const THICK = {
  SIGNAL: 0.035, POWER_GROUND: 0.035, DIELECTRIC: 0.12,
  SOLDER_MASK: 0.02, SOLDER_PASTE: 0.012,
}

const SIGNAL_PALETTE = ['#e8b64c', '#4aa3df', '#6cc24a', '#e2725b', '#b084cc', '#58c4c4', '#d96fa5', '#9fb33a']
const TYPE_COLOR = {
  SOLDER_MASK: '#1f7a3f', SOLDER_PASTE: '#9aa0a6', DRILL: '#ffffff', ROUT: '#ff5252',
  DOCUMENT: '#b9c0c7', COMPONENT: '#ffd54f', MASK: '#7e8a97', DIELECTRIC: '#8a7a2a',
}

function parseMatrix(text) {
  const out = { steps: [], layers: [] }
  const re = /(\w+)\s*\{([^}]*)\}/g
  let m
  while ((m = re.exec(text))) {
    const rec = {}
    for (const line of m[2].split('\n')) {
      const i = line.indexOf('=')
      if (i > 0) rec[line.slice(0, i).trim()] = line.slice(i + 1).trim()
    }
    if (m[1] === 'STEP') out.steps.push(rec)
    else if (m[1] === 'LAYER') out.layers.push(rec)
  }
  out.layers.sort((a, b) => +a.ROW - +b.ROW)
  return out
}

/** Walk the matrix top-to-bottom, assigning each layer a Z (mm, top = 0, downwards negative). */
function assignZ(layers) {
  let z = 0
  const byName = new Map()
  for (const l of layers) {
    if (l.CONTEXT !== 'BOARD') continue
    const t = THICK[l.TYPE] || 0
    l.z = z - t / 2
    z -= t
    byName.set(l.NAME, l)
  }
  const bottom = z
  for (const l of layers) {
    if (l.CONTEXT !== 'BOARD') l.z = 0.1              // misc/doc layers float above the stack
    else if (l.TYPE === 'DRILL') l.z = (byName.get(l.START_NAME)?.z ?? 0)
  }
  return { top: 0, bottom }
}

export async function loadJob(file, onProgress = () => {}) {
  onProgress('unpacking…')
  const files = await untar(file)
  const root = [...files.keys()][0].split('/')[0] + '/'
  const text = (p) => { const d = files.get(root + p); return d ? new TextDecoder().decode(d) : null }

  const matrix = parseMatrix(text('matrix/matrix') || '')
  const step = (matrix.steps[0]?.NAME || '').toLowerCase()
  const mm = /UNITS\s*=\s*MM/i.test(text(`steps/${step}/stephdr`) || '')

  onProgress('loading wasm…')
  await initWasm()

  onProgress('registering symbols…')
  for (const key of files.keys()) {
    const m = key.slice(root.length).match(/^symbols\/([^/]+)\/features$/)
    if (m) addSymbol(m[1], text(`symbols/${m[1]}/features`))
  }

  const { bottom } = assignZ(matrix.layers)
  let sig = 0
  const layers = []
  for (const l of matrix.layers) {
    const dir = l.NAME.toLowerCase()
    const src = text(`steps/${step}/layers/${dir}/features`) ?? ''
    onProgress(`tessellating ${dir}…`)
    const geom = src ? tessellate(src, mm) : { v: new Float32Array(0), i: new Uint32Array(0) }
    const isSig = l.TYPE === 'SIGNAL' || l.TYPE === 'POWER_GROUND'
    const color = +l.COLOR ? '#' + l.COLOR.padStart(6, '0')
      : isSig ? SIGNAL_PALETTE[sig++ % SIGNAL_PALETTE.length]
      : TYPE_COLOR[l.TYPE] || '#888888'
    layers.push({
      name: l.NAME, type: l.TYPE, context: l.CONTEXT, z: l.z, color, geom,
      tris: geom.i.length / 3,
      visible: isSig || l.TYPE === 'DRILL',
    })
  }
  const profile = text(`steps/${step}/profile`)
  return {
    name: text('misc/job_name')?.trim() || file.name,
    step, units: mm ? 'mm' : 'inch', layers, thickness: -bottom,
    outline: profile ? tessellate(profile, mm) : null,
  }
}
