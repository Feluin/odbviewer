/** .tgz -> Map<path, Uint8Array>, using the browser's native gzip decoder. */
export async function untar(file) {
  const stream = file.stream().pipeThrough(new DecompressionStream('gzip'))
  const buf = new Uint8Array(await new Response(stream).arrayBuffer())
  const dec = new TextDecoder()
  const str = (a, b) => dec.decode(buf.subarray(a, b)).replace(/\0[\s\S]*/, '')
  const out = new Map()
  let off = 0, longName = null

  while (off + 512 <= buf.length) {
    if (buf[off] === 0) { off += 512; continue }
    let name = str(off, off + 100)
    const prefix = str(off + 345, off + 500)
    if (prefix) name = prefix + '/' + name
    const size = parseInt(str(off + 124, off + 136).replace(/[^0-7]/g, ''), 8) || 0
    const type = String.fromCharCode(buf[off + 156])
    off += 512
    const data = buf.subarray(off, off + size)
    off += Math.ceil(size / 512) * 512

    if (type === 'L') { longName = dec.decode(data).replace(/\0[\s\S]*/, ''); continue }
    if (longName) { name = longName; longName = null }
    if (type === '0' || type === '\0') out.set(name, data)
  }
  return out
}
