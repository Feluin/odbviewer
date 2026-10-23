//! ODB++ feature files -> triangles. No wasm-bindgen: raw extern "C", JS copies the buffers out.
use lyon::geom::Box2D;
use lyon::math::{point, Point};
use lyon::path::builder::{BorderRadii, NoAttributes};
use lyon::path::path::BuilderImpl;

type PB = NoAttributes<BuilderImpl>;
use lyon::path::{Path, Winding};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, LineCap, LineJoin,
    StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};
use std::cell::RefCell;
use std::collections::HashMap;

/// Everything is converted to mm on the way in, so one tolerance fits both unit systems.
const TOL: f32 = 0.008;
const TAU: f32 = std::f32::consts::TAU;

type Mesh = VertexBuffers<[f32; 2], u32>;

#[derive(Default)]
struct Out {
    v: Vec<f32>,
    i: Vec<u32>,
}

/// Mirror in X, then rotate, then translate — ODB++ pad placement.
struct Tf {
    dx: f32,
    dy: f32,
    c: f32,
    s: f32,
    mx: f32,
}

impl Out {
    fn add(&mut self, m: &Mesh, tf: Option<&Tf>) {
        let base = (self.v.len() / 2) as u32;
        for p in &m.vertices {
            let (x, y) = match tf {
                None => (p[0], p[1]),
                Some(t) => {
                    let x = p[0] * t.mx;
                    (t.dx + x * t.c - p[1] * t.s, t.dy + x * t.s + p[1] * t.c)
                }
            };
            self.v.push(x);
            self.v.push(y);
        }
        self.i.extend(m.indices.iter().map(|k| k + base));
    }
    fn merge(&mut self, o: &Out, tf: Option<&Tf>) {
        let base = (self.v.len() / 2) as u32;
        for p in o.v.chunks_exact(2) {
            let (x, y) = match tf {
                None => (p[0], p[1]),
                Some(t) => {
                    let x = p[0] * t.mx;
                    (t.dx + x * t.c - p[1] * t.s, t.dy + x * t.s + p[1] * t.c)
                }
            };
            self.v.push(x);
            self.v.push(y);
        }
        self.i.extend(o.i.iter().map(|k| k + base));
    }
}

fn fill(p: &Path) -> Mesh {
    let mut b = Mesh::new();
    // EvenOdd: ODB++ marks holes with OB..H but does not guarantee winding.
    let o = FillOptions::tolerance(TOL).with_fill_rule(FillRule::EvenOdd);
    let _ = FillTessellator::new().tessellate_path(
        p,
        &o,
        &mut BuffersBuilder::new(&mut b, |v: FillVertex| v.position().to_array()),
    );
    b
}

fn stroke(p: &Path, w: f32, cap: LineCap) -> Mesh {
    let mut b = Mesh::new();
    let o = StrokeOptions::tolerance(TOL)
        .with_line_width(w.max(1e-4))
        .with_line_cap(cap)
        .with_line_join(LineJoin::Round);
    let _ = StrokeTessellator::new().tessellate_path(
        p,
        &o,
        &mut BuffersBuilder::new(&mut b, |v: StrokeVertex| v.position().to_array()),
    );
    b
}

fn f(s: &str) -> f32 {
    s.parse().unwrap_or(0.0)
}

/// Flatten an ODB++ arc (start, end, centre, direction) into line segments.
fn arc_to(pb: &mut PB, s: Point, e: Point, c: Point, cw: bool) {
    let r = (s - c).length();
    if !r.is_finite() || r <= 1e-9 {
        pb.line_to(e);
        return;
    }
    let a0 = (s.y - c.y).atan2(s.x - c.x);
    let a1 = (e.y - c.y).atan2(e.x - c.x);
    let mut sweep = a1 - a0;
    if cw {
        if sweep >= 0.0 {
            sweep -= TAU;
        }
    } else if sweep <= 0.0 {
        sweep += TAU;
    }
    if (s - e).length() < 1e-9 {
        sweep = if cw { -TAU } else { TAU };
    }
    let step = 2.0 * (1.0 - (TOL / r).min(1.0)).acos();
    let n = (sweep.abs() / step.max(1e-3)).ceil().min(2048.0).max(1.0) as usize;
    for k in 1..=n {
        let a = a0 + sweep * (k as f32 / n as f32);
        pb.line_to(point(c.x + r * a.cos(), c.y + r * a.sin()));
    }
}

/// Standard ODB++ symbol names. `None` => user symbol, look it up in the symbols map.
fn sym_path(name: &str, us: f32) -> Option<Path> {
    let mut b = Path::builder();
    if let Some(r) = name.strip_prefix("rect") {
        let d = r.split('x').collect::<Vec<_>>();
        let (w, h) = (f(d[0]) * us, f(*d.get(1)?) * us);
        let mut rad = 0.0;
        for p in d.iter().skip(2) {
            if let Some(v) = p.strip_prefix('r').or_else(|| p.strip_prefix('c')) {
                rad = f(v) * us;
            }
        }
        let bx = Box2D::new(point(-w / 2.0, -h / 2.0), point(w / 2.0, h / 2.0));
        if rad > 0.0 {
            b.add_rounded_rectangle(&bx, &BorderRadii::new(rad.min(w / 2.0).min(h / 2.0)), Winding::Positive);
        } else {
            b.add_rectangle(&bx, Winding::Positive);
        }
    } else if let Some(r) = name.strip_prefix("oval") {
        let d = r.split('x').collect::<Vec<_>>();
        let (w, h) = (f(d[0]) * us, f(*d.get(1)?) * us);
        let bx = Box2D::new(point(-w / 2.0, -h / 2.0), point(w / 2.0, h / 2.0));
        b.add_rounded_rectangle(&bx, &BorderRadii::new(w.min(h) / 2.0), Winding::Positive);
    } else if let Some(r) = name.strip_prefix("donut_r") {
        let d = r.split('x').collect::<Vec<_>>();
        b.add_circle(point(0.0, 0.0), f(d[0]) * us / 2.0, Winding::Positive);
        b.add_circle(point(0.0, 0.0), f(*d.get(1)?) * us / 2.0, Winding::Positive);
    } else if let Some(r) = name.strip_prefix("sq") {
        let s = f(r) * us / 2.0;
        b.add_rectangle(&Box2D::new(point(-s, -s), point(s, s)), Winding::Positive);
    } else if let Some(r) = name.strip_prefix('s').filter(|v| v.starts_with(|c: char| c.is_ascii_digit())) {
        let s = f(r) * us / 2.0;
        b.add_rectangle(&Box2D::new(point(-s, -s), point(s, s)), Winding::Positive);
    } else if let Some(r) = name.strip_prefix('r') {
        b.add_circle(point(0.0, 0.0), f(r) * us / 2.0, Winding::Positive);
    } else {
        return None;
    }
    Some(b.build())
}

/// Stroke width + cap for a symbol dragged along a line/arc.
// ponytail: rectangular apertures are stroked as square caps of their narrow side — exact
// rect-sweep outlines would need a Minkowski sum; revisit if fab-grade accuracy is wanted.
fn sym_stroke(name: &str, us: f32) -> (f32, LineCap) {
    if let Some(r) = name.strip_prefix("rect") {
        let d: Vec<&str> = r.split('x').collect();
        let w = f(d[0]);
        let h = d.get(1).map(|v| f(v)).unwrap_or(w);
        (w.min(h) * us, LineCap::Square)
    } else if let Some(r) = name.strip_prefix("oval") {
        let d: Vec<&str> = r.split('x').collect();
        let w = f(d[0]);
        let h = d.get(1).map(|v| f(v)).unwrap_or(w);
        (w.min(h) * us, LineCap::Round)
    } else if let Some(r) = name.strip_prefix("sq") {
        (f(r) * us, LineCap::Square)
    } else if let Some(r) = name.strip_prefix('r') {
        (f(r) * us, LineCap::Round)
    } else {
        (0.05, LineCap::Round) // unknown symbol on a line: hairline rather than nothing
    }
}

fn sym_mesh(name: &str, cs: f32, us: f32, syms: &HashMap<String, String>, depth: u32) -> Out {
    let mut o = Out::default();
    if let Some(p) = sym_path(name, us) {
        o.add(&fill(&p), None);
    } else if depth < 4 {
        if let Some(src) = syms.get(name) {
            build(src, cs, us, syms, &mut HashMap::new(), &mut o, depth + 1);
        }
    }
    o
}

fn build(
    src: &str,
    cs: f32,
    us: f32,
    syms: &HashMap<String, String>,
    cache: &mut HashMap<String, Out>,
    out: &mut Out,
    depth: u32,
) {
    let mut names: Vec<String> = Vec::new();
    let n = |s: &str| f(s) * cs;
    let mut it = src.lines();
    while let Some(line) = it.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('$') {
            let mut t = rest.splitn(2, ' ');
            let idx: usize = t.next().unwrap_or("").parse().unwrap_or(usize::MAX);
            if idx == usize::MAX {
                continue;
            }
            if names.len() <= idx {
                names.resize(idx + 1, String::new());
            }
            names[idx] = t.next().unwrap_or("").trim().to_string();
            continue;
        }
        // Attributes start at the first ';' and are sometimes glued to the last field.
        let head = line.split(';').next().unwrap_or("");
        let t: Vec<&str> = head.split_whitespace().collect();
        let sym = |i: usize| -> &str {
            t.get(i)
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|k| names.get(k))
                .map(|s| s.as_str())
                .unwrap_or("")
        };
        match t.first().copied().unwrap_or("") {
            "L" if t.len() >= 6 => {
                let (a, b) = (point(n(t[1]), n(t[2])), point(n(t[3]), n(t[4])));
                let (w, cap) = sym_stroke(sym(5), us);
                if (a - b).length() < 1e-7 {
                    let tf = Tf { dx: a.x, dy: a.y, c: 1.0, s: 0.0, mx: 1.0 };
                    out.merge(cached(sym(5), cs, us, syms, cache, depth), Some(&tf));
                    continue;
                }
                let mut pb = Path::builder();
                pb.begin(a);
                pb.line_to(b);
                pb.end(false);
                out.add(&stroke(&pb.build(), w, cap), None);
            }
            "A" if t.len() >= 10 => {
                let s = point(n(t[1]), n(t[2]));
                let e = point(n(t[3]), n(t[4]));
                let c = point(n(t[5]), n(t[6]));
                let (w, cap) = sym_stroke(sym(7), us);
                let mut pb = Path::builder();
                pb.begin(s);
                arc_to(&mut pb, s, e, c, t[10] == "Y");
                pb.end(false);
                out.add(&stroke(&pb.build(), w, cap), None);
            }
            "P" if t.len() >= 7 => {
                let o: u32 = t[6].parse().unwrap_or(0);
                let (rot, mirror) = match o {
                    0..=3 => (o as f32 * 90.0, false),
                    4..=7 => ((o - 4) as f32 * 90.0, true),
                    8 => (t.get(7).map(|v| f(v)).unwrap_or(0.0), false),
                    _ => (t.get(7).map(|v| f(v)).unwrap_or(0.0), true),
                };
                let a = rot.to_radians();
                let tf = Tf {
                    dx: n(t[1]),
                    dy: n(t[2]),
                    c: a.cos(),
                    s: a.sin(),
                    mx: if mirror { -1.0 } else { 1.0 },
                };
                let m = cached(sym(3), cs, us, syms, cache, depth);
                out.merge(m, Some(&tf));
            }
            "S" => {
                let mut pb = Path::builder();
                let mut cur = point(0.0, 0.0);
                let mut open = false;
                for l2 in it.by_ref() {
                    let h = l2.trim().split(';').next().unwrap_or("");
                    let u: Vec<&str> = h.split_whitespace().collect();
                    match u.first().copied().unwrap_or("") {
                        "OB" if u.len() >= 3 => {
                            cur = point(n(u[1]), n(u[2]));
                            pb.begin(cur);
                            open = true;
                        }
                        "OS" if u.len() >= 3 => {
                            cur = point(n(u[1]), n(u[2]));
                            pb.line_to(cur);
                        }
                        "OC" if u.len() >= 6 => {
                            let e = point(n(u[1]), n(u[2]));
                            arc_to(&mut pb, cur, e, point(n(u[3]), n(u[4])), u[5] == "Y");
                            cur = e;
                        }
                        "OE" => {
                            if open {
                                pb.end(true);
                                open = false;
                            }
                        }
                        "SE" => break,
                        _ => {}
                    }
                }
                if open {
                    pb.end(true);
                }
                out.add(&fill(&pb.build()), None);
            }
            _ => {}
        }
    }
}

/// Tessellate each distinct symbol once at the origin; pads are instanced copies.
fn cached<'a>(
    name: &str,
    cs: f32,
    us: f32,
    syms: &HashMap<String, String>,
    cache: &'a mut HashMap<String, Out>,
    depth: u32,
) -> &'a Out {
    if !cache.contains_key(name) {
        let m = sym_mesh(name, cs, us, syms, depth);
        cache.insert(name.to_string(), m);
    }
    &cache[name]
}

thread_local! {
    static SYMS: RefCell<HashMap<String,String>> = RefCell::new(HashMap::new());
    static RES: RefCell<Out> = RefCell::new(Out::default());
}

unsafe fn s<'a>(p: *const u8, l: usize) -> &'a str {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, l))
}

#[no_mangle]
pub extern "C" fn alloc(n: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(n);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[no_mangle]
pub unsafe extern "C" fn dealloc(p: *mut u8, n: usize) {
    drop(Vec::from_raw_parts(p, 0, n));
}

#[no_mangle]
pub unsafe extern "C" fn add_symbol(np: *const u8, nl: usize, sp: *const u8, sl: usize) {
    SYMS.with(|m| {
        m.borrow_mut().insert(s(np, nl).to_string(), s(sp, sl).to_string());
    });
}

/// mm != 0 => job units are mm; otherwise inch (symbol names are then in mils).
#[no_mangle]
pub unsafe extern "C" fn tessellate(p: *const u8, l: usize, mm: u32) {
    let (cs, us) = if mm != 0 { (1.0, 1.0) } else { (25.4, 0.0254) };
    let mut out = Out::default();
    SYMS.with(|syms| {
        build(s(p, l), cs, us, &syms.borrow(), &mut HashMap::new(), &mut out, 0);
    });
    RES.with(|r| *r.borrow_mut() = out);
}

#[no_mangle]
pub extern "C" fn vptr() -> *const f32 {
    RES.with(|r| r.borrow().v.as_ptr())
}
#[no_mangle]
pub extern "C" fn vlen() -> usize {
    RES.with(|r| r.borrow().v.len())
}
#[no_mangle]
pub extern "C" fn iptr() -> *const u32 {
    RES.with(|r| r.borrow().i.as_ptr())
}
#[no_mangle]
pub extern "C" fn ilen() -> usize {
    RES.with(|r| r.borrow().i.len())
}
