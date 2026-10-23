import wasmUrl from './odb.wasm?url'

let e
const enc = new TextEncoder()

export async function initWasm() {
  if (!e) e = (await WebAssembly.instantiateStreaming(fetch(wasmUrl), {})).instance.exports
  return e
}

function put(str) {
  const b = enc.encode(str)
  const p = e.alloc(b.length)
  new Uint8Array(e.memory.buffer).set(b, p)
  return [p, b.length]
}

export function addSymbol(name, src) {
  const n = put(name), s = put(src)
  e.add_symbol(n[0], n[1], s[0], s[1])
  e.dealloc(n[0], n[1])
  e.dealloc(s[0], s[1])
}

/** features text -> { v: Float32Array (x,y pairs), i: Uint32Array } */
export function tessellate(src, mm) {
  const [p, l] = put(src)
  e.tessellate(p, l, mm ? 1 : 0)
  const vl = e.vlen(), il = e.ilen()
  const v = vl ? new Float32Array(e.memory.buffer, e.vptr(), vl).slice() : new Float32Array(0)
  const i = il ? new Uint32Array(e.memory.buffer, e.iptr(), il).slice() : new Uint32Array(0)
  e.dealloc(p, l)
  return { v, i }
}
