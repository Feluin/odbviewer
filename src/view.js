import * as THREE from 'three'
import { OrbitControls } from 'three/addons/controls/OrbitControls.js'


export class Viewer {
  constructor(canvas) {
    this.canvas = canvas
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true })
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 2))
    this.scene = new THREE.Scene()
    this.scene.background = new THREE.Color('#0d1117')
    this.meshes = []
    this.mode = '2d'
    this.zScale = 20
    this.box = new THREE.Box3()

    this.ortho = new THREE.OrthographicCamera(-1, 1, 1, -1, -1e4, 1e4)
    this.persp = new THREE.PerspectiveCamera(45, 1, 0.1, 1e5)

    const ro = new ResizeObserver(() => this.resize())
    ro.observe(canvas.parentElement)
    this.resize()
    this.renderer.setAnimationLoop(() => {
      this.controls?.update()
      this.renderer.render(this.scene, this.cam)
    })
  }

  get cam() { return this.mode === '3d' ? this.persp : this.ortho }

  setLayers(layers) {
    for (const m of this.meshes) { m.geometry.dispose(); m.material.dispose(); this.scene.remove(m) }
    this.box.makeEmpty()
    this.meshes = layers.map((l, idx) => {
      const g = new THREE.BufferGeometry()
      // itemSize 2: WebGL fills the shader's vec3 position with z = 0, so a layer's depth
      // is carried by mesh.position.z alone — half the vertex memory. three.js cannot derive
      // bounds from a 2-float stride (it reads getZ() past the end and yields NaN), so they
      // are set here from the x,y pairs.
      g.setAttribute('position', new THREE.BufferAttribute(l.geom.v, 2))
      g.setIndex(new THREE.BufferAttribute(l.geom.i, 1))
      const v = l.geom.v
      let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
      for (let k = 0; k < v.length; k += 2) {
        if (v[k] < x0) x0 = v[k]
        if (v[k] > x1) x1 = v[k]
        if (v[k + 1] < y0) y0 = v[k + 1]
        if (v[k + 1] > y1) y1 = v[k + 1]
      }
      if (v.length) {
        g.boundingBox = new THREE.Box3(new THREE.Vector3(x0, y0, 0), new THREE.Vector3(x1, y1, 0))
        g.boundingSphere = new THREE.Sphere(
          new THREE.Vector3((x0 + x1) / 2, (y0 + y1) / 2, 0),
          Math.hypot(x1 - x0, y1 - y0) / 2,
        )
        if (l.visible) this.box.union(g.boundingBox)
      } else {
        g.boundingBox = new THREE.Box3()
        g.boundingSphere = new THREE.Sphere(new THREE.Vector3(), 0)
      }
      const mesh = new THREE.Mesh(g, new THREE.MeshBasicMaterial({
        color: l.color, side: THREE.DoubleSide, transparent: true,
      }))
      mesh.userData = l
      mesh.renderOrder = idx
      this.scene.add(mesh)
      return mesh
    })
    if (this.box.isEmpty()) for (const m of this.meshes) if (m.geometry.boundingBox) this.box.union(m.geometry.boundingBox)
    this.apply()
    this.fit()
  }

  /** Re-read visibility/colour/mode from the layer objects. */
  apply() {
    const three = this.mode === '3d'
    for (const m of this.meshes) {
      const l = m.userData
      m.visible = l.visible && l.geom.i.length > 0
      m.material.color.set(l.color)
      m.material.opacity = three ? 1 : 0.8
      m.material.depthTest = three
      m.material.depthWrite = three
      m.position.z = three ? l.z * this.zScale : 0
    }
    this.controls?.dispose()
    this.controls = new OrbitControls(this.cam, this.canvas)
    this.controls.enableRotate = three
    this.controls.screenSpacePanning = true
    if (!three) this.controls.mouseButtons = { LEFT: THREE.MOUSE.PAN, MIDDLE: THREE.MOUSE.DOLLY, RIGHT: THREE.MOUSE.PAN }
    this.resize()
  }

  fit() {
    if (this.box.isEmpty()) return
    const c = this.box.getCenter(new THREE.Vector3())
    const s = this.box.getSize(new THREE.Vector3())
    const r = Math.max(s.x, s.y) / 2 || 1
    this.orthoRadius = r * 1.1
    this.ortho.position.set(c.x, c.y, 1000)
    this.persp.position.set(c.x, c.y - r * 1.4, r * 1.2)
    for (const ct of [this.ortho, this.persp]) ct.up.set(0, 1, 0)
    this.controls.target.set(c.x, c.y, 0)
    this.resize()
  }

  resize() {
    const el = this.canvas.parentElement
    const w = el.clientWidth || 1, h = el.clientHeight || 1
    this.renderer.setSize(w, h, false)
    const r = this.orthoRadius || 1
    const a = w / h
    this.ortho.left = -r * a; this.ortho.right = r * a; this.ortho.top = r; this.ortho.bottom = -r
    this.ortho.updateProjectionMatrix()
    this.persp.aspect = a
    this.persp.updateProjectionMatrix()
  }
}
