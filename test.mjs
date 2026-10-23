// Smallest check that fails if tar reading or ODB++ path resolution breaks.
// Run: node test.mjs   (wasm tessellation is covered separately by the same wasm the app loads)
import assert from 'node:assert'
import { readFileSync } from 'node:fs'
import { untar } from './src/tar.js'

const buf = readFileSync('designodb_rigidflex.tgz')
const files = await untar(new File([buf], 'designodb_rigidflex.tgz'))
const root = [...files.keys()][0].split('/')[0] + '/'
const text = (p) => { const d = files.get(root + p); return d ? new TextDecoder().decode(d) : null }

assert.ok(files.size > 800, 'untar found files')
const matrix = text('matrix/matrix')
assert.match(matrix, /STEP \{/, 'matrix readable')

const step = matrix.match(/NAME=(\S+)/)[1].toLowerCase()
const layers = [...matrix.matchAll(/LAYER \{([^}]*)\}/g)].map((m) => m[1].match(/NAME=(\S*)/)[1])
assert.equal(layers.length, 43)
assert.ok(text(`steps/${step}/stephdr`), 'stephdr resolves')
assert.ok(text(`steps/${step}/profile`), 'profile resolves')

const withFeatures = layers.filter((n) => text(`steps/${step}/layers/${n.toLowerCase()}/features`) != null)
assert.equal(withFeatures.length, layers.length, 'every matrix layer maps to a features file')
assert.equal(withFeatures.filter((n) => text(`steps/${step}/layers/${n.toLowerCase()}/features`) === '').length, 2, 'the two component layers carry no features')

const syms = [...files.keys()].filter((k) => /^symbols\/[^/]+\/features$/.test(k.slice(root.length)))
assert.ok(syms.length > 100, `symbols found: ${syms.length}`)

// the wasm symbol lookup is by bare directory name — must match what features files reference
const names = new Set(syms.map((k) => k.slice(root.length).split('/')[1]))
const refs = [...text(`steps/${step}/layers/signal_1/features`).matchAll(/^\$\d+ (\S+)$/gm)].map((m) => m[1])
const special = refs.filter((r) => !/^(r|rect|oval|sq|s|donut_)/.test(r) || r.startsWith('special') || r.startsWith('sc_'))
assert.ok(special.every((r) => names.has(r)), 'every user symbol referenced resolves: ' + special.filter((r) => !names.has(r)))

console.log(`ok — ${files.size} files, step "${step}", ${layers.length} layers, ${syms.length} symbols, ${special.length} user symbols on signal_1`)
