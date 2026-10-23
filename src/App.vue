<script setup>
import { ref, onMounted, shallowRef } from 'vue'
import { loadJob } from './job.js'
import { Viewer } from './view.js'

const canvas = ref(null)
const job = shallowRef(null)
const status = ref('')
const mode = ref('2d')
const zScale = ref(20)
let viewer

onMounted(async () => {
  viewer = new Viewer(canvas.value)
  // ?job=<url> loads a job straight from the server — handy for demos and dev.
  const url = new URLSearchParams(location.search).get('job')
  if (url) open(new File([await (await fetch(url)).blob()], url.split('/').pop()))
})

async function open(file) {
  if (!file) return
  try {
    const t = performance.now()
    job.value = null
    job.value = await loadJob(file, (s) => (status.value = s))
    viewer.setLayers(job.value.layers)
    const tris = job.value.layers.reduce((a, l) => a + l.tris, 0)
    status.value = `${tris.toLocaleString()} triangles · ${Math.round(performance.now() - t)} ms`
  } catch (e) {
    status.value = 'failed: ' + e.message
    console.error(e)
  }
}

function setMode(m) { mode.value = m; viewer.mode = m; viewer.apply() }
function setZ(v) { zScale.value = +v; viewer.zScale = +v; viewer.apply() }
function toggle(l) { l.visible = !l.visible; viewer.apply() }
function only(l) {
  for (const x of job.value.layers) x.visible = x === l
  viewer.apply()
}
function all(v) { for (const x of job.value.layers) x.visible = v && x.tris > 0; viewer.apply() }
</script>

<template>
  <div class="app" @dragover.prevent @drop.prevent="open($event.dataTransfer.files[0])">
    <aside>
      <h1>ODB++ Viewer</h1>
      <label class="file">
        Open .tgz
        <input type="file" accept=".tgz,.tar.gz,.gz" @change="open($event.target.files[0])" />
      </label>

      <div v-if="job" class="meta">
        <b>{{ job.name }}</b><br />
        step {{ job.step }} · {{ job.units }} · {{ job.thickness.toFixed(2) }} mm stack
      </div>

      <div class="modes" v-if="job">
        <button :class="{ on: mode === '2d' }" @click="setMode('2d')">2D</button>
        <button :class="{ on: mode === '3d' }" @click="setMode('3d')">3D</button>
      </div>
      <label v-if="job && mode === '3d'" class="zs">
        Z-scale ×{{ zScale }}
        <input type="range" min="1" max="200" :value="zScale" @input="setZ($event.target.value)" />
      </label>

      <div v-if="job" class="bulk">
        <button @click="all(true)">all</button><button @click="all(false)">none</button>
      </div>

      <ul v-if="job" class="layers">
        <li v-for="l in job.layers" :key="l.name" :class="{ empty: !l.tris }">
          <input type="checkbox" :checked="l.visible" :disabled="!l.tris" @change="toggle(l)" />
          <span class="sw" :style="{ background: l.color }" />
          <span class="nm" :title="l.type" @click="l.tris && only(l)">{{ l.name.toLowerCase() }}</span>
          <span class="ct">{{ l.tris.toLocaleString() }}</span>
        </li>
      </ul>

      <p class="status">{{ status }}</p>
    </aside>

    <main>
      <canvas ref="canvas" />
      <div v-if="!job" class="drop">Drop an ODB++ <code>.tgz</code> here</div>
    </main>
  </div>
</template>

<style scoped>
.app { display: flex; height: 100vh; font: 13px/1.4 system-ui, sans-serif; color: #c9d1d9; background: #0d1117; }
aside { width: 272px; flex: none; padding: 14px; overflow: auto; border-right: 1px solid #21262d; display: flex; flex-direction: column; gap: 10px; }
h1 { font-size: 15px; margin: 0; }
.file { background: #21262d; border-radius: 6px; padding: 8px; text-align: center; cursor: pointer; }
.file input { display: none; }
.meta { color: #8b949e; font-size: 12px; }
.modes, .bulk { display: flex; gap: 6px; }
button { flex: 1; background: #21262d; color: inherit; border: 1px solid #30363d; border-radius: 6px; padding: 5px; cursor: pointer; }
button.on { background: #1f6feb; border-color: #1f6feb; }
.zs { font-size: 12px; color: #8b949e; }
.zs input { width: 100%; }
.layers { list-style: none; margin: 0; padding: 0; flex: 1; }
.layers li { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
.layers li.empty { opacity: 0.35; }
.sw { width: 11px; height: 11px; border-radius: 2px; flex: none; }
.nm { flex: 1; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ct { color: #6e7681; font-size: 11px; }
.status { color: #8b949e; font-size: 12px; margin: 0; }
main { flex: 1; position: relative; min-width: 0; }
canvas { display: block; width: 100%; height: 100%; }
.drop { position: absolute; inset: 0; display: grid; place-items: center; color: #6e7681; pointer-events: none; }
</style>
