'use strict';
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const ROOT = path.join(__dirname, '..');
const ASSETS = path.join(ROOT, 'assets');

// слои раскопа в 44-юнитовом дизайн-пространстве, по центру (22,22)
const BARS = [
  { x: 12, y: 7,  w: 20, h: 5, r: 2.5 },
  { x: 7,  y: 15, w: 30, h: 5, r: 2.5 },
  { x: 3,  y: 23, w: 38, h: 5, r: 2.5 },
  { x: 9,  y: 31, w: 26, h: 5, r: 2.5 },
  { x: 16, y: 38, w: 12, h: 4, r: 2 }
];
const C1 = [240, 200, 120];
const C2 = [206, 120, 46];
const BG = [14, 11, 10];
const GLOW_BG = [48, 32, 18];

const clamp = (v, a, b) => (v < a ? a : v > b ? b : v);
const lerp = (a, b, t) => a + (b - a) * t;

function rrectDist(px, py, rc) {
  const cx = rc.x + rc.w / 2, cy = rc.y + rc.h / 2;
  const qx = Math.abs(px - cx) - (rc.w / 2 - rc.r);
  const qy = Math.abs(py - cy) - (rc.h / 2 - rc.r);
  const ax = Math.max(qx, 0), ay = Math.max(qy, 0);
  return Math.min(Math.max(qx, qy), 0) + Math.hypot(ax, ay) - rc.r;
}

function render(size) {
  const SS = 4, N = size * SS;
  const out = new Uint8Array(size * size * 4);
  const contentScale = size <= 32 ? 0.74 : size <= 48 ? 0.68 : 0.62;
  const scale = contentScale * N / 44;
  const maskR = N * 0.21;

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let R = 0, G = 0, B = 0, A = 0;
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const u = x * SS + sx + 0.5, v = y * SS + sy + 0.5;
          const nx = u / N, ny = v / N;
          const lu = (u - N / 2) / scale + 22, lv = (v - N / 2) / scale + 22;

          const d1 = Math.hypot(nx - 0.75, ny - 0.18) / 0.9;
          const f1 = Math.pow(clamp(1 - d1, 0, 1), 1.5);
          let r = lerp(BG[0], GLOW_BG[0], f1);
          let g = lerp(BG[1], GLOW_BG[1], f1);
          let b = lerp(BG[2], GLOW_BG[2], f1);
          const d2 = Math.hypot(nx - 0.5, ny - 0.52) / 0.55;
          const f2 = Math.pow(clamp(1 - d2, 0, 1), 2);
          r = clamp(r + C1[0] * f2 * 0.08, 0, 255);
          g = clamp(g + C1[1] * f2 * 0.08, 0, 255);
          b = clamp(b + C1[2] * f2 * 0.08, 0, 255);
          let a = 1;

          for (const rc of BARS) {
            if (rrectDist(lu, lv, rc) <= 0) {
              const t = clamp((lv - 5) / 34, 0, 1);
              r = lerp(C1[0], C2[0], t);
              g = lerp(C1[1], C2[1], t);
              b = lerp(C1[2], C2[2], t);
              break;
            }
          }

          if (rrectDist(u, v, { x: 0, y: 0, w: N, h: N, r: maskR }) > 0) a = 0;

          R += r * a; G += g * a; B += b * a; A += a;
        }
      }
      const n = SS * SS, i = (y * size + x) * 4;
      const alpha = A / n;
      out[i] = alpha > 0 ? Math.round(R / A) : 0;
      out[i + 1] = alpha > 0 ? Math.round(G / A) : 0;
      out[i + 2] = alpha > 0 ? Math.round(B / A) : 0;
      out[i + 3] = Math.round(alpha * 255);
    }
  }
  return out;
}

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const b = Buffer.alloc(12 + data.length);
  b.writeUInt32BE(data.length, 0);
  b.write(type, 4, 'ascii');
  data.copy(b, 8);
  b.writeUInt32BE(crc32(b.subarray(4, 8 + data.length)), 8 + data.length);
  return b;
}

function encodePng(size, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    Buffer.from(rgba.buffer, y * size * 4, size * 4).copy(raw, y * (size * 4 + 1) + 1);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ]);
}

function writePng(file, size) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, encodePng(size, render(size)));
  console.log(path.relative(ROOT, file), size + 'px');
}

function writeIco(file, sizes) {
  const pngs = sizes.map(s => encodePng(s, render(s)));
  const head = Buffer.alloc(6 + sizes.length * 16);
  head.writeUInt16LE(1, 2);
  head.writeUInt16LE(sizes.length, 4);
  let off = head.length;
  sizes.forEach((s, i) => {
    const e = 6 + i * 16;
    head[e] = s >= 256 ? 0 : s;
    head[e + 1] = s >= 256 ? 0 : s;
    head.writeUInt16LE(1, e + 4);
    head.writeUInt16LE(32, e + 6);
    head.writeUInt32LE(pngs[i].length, e + 8);
    head.writeUInt32LE(off, e + 12);
    off += pngs[i].length;
  });
  fs.writeFileSync(file, Buffer.concat([head, ...pngs]));
  console.log(path.relative(ROOT, file), sizes.join('/'));
}

writePng(path.join(ASSETS, 'icon.png'), 512);
writeIco(path.join(ASSETS, 'icon.ico'), [256, 128, 64, 48, 32, 16]);
console.log('Готово.');
