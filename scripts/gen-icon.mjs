// 生成 1024x1024 RGBA 占位图标 app-icon.png（纯 Node stdlib，无第三方依赖）。
// 用法：node scripts/gen-icon.mjs
// 之后运行：npx tauri icon app-icon.png 生成 src-tauri/icons/ 全套。
import { deflateSync, crc32 } from 'node:zlib';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const SIZE = 1024;
const CENTER = SIZE / 2;
const RADIUS = 380;

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])) >>> 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

// 每行前导 1 字节 filter=0，之后 RGBA 像素。
const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  const rowStart = y * (SIZE * 4 + 1);
  raw[rowStart] = 0;
  for (let x = 0; x < SIZE; x++) {
    const dx = x - CENTER;
    const dy = y - CENTER;
    const off = rowStart + 1 + x * 4;
    if (dx * dx + dy * dy <= RADIUS * RADIUS) {
      // 深蓝垂直渐变（占位即可）
      const t = y / SIZE;
      raw[off] = Math.round(90 - t * 30);
      raw[off + 1] = Math.round(107 - t * 30);
      raw[off + 2] = Math.round(254 - t * 40);
      raw[off + 3] = 255;
    } else {
      raw[off] = 0;
      raw[off + 1] = 0;
      raw[off + 2] = 0;
      raw[off + 3] = 0; // 透明背景，满足 tauri icon 的透明度要求
    }
  }
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type: RGBA
ihdr[10] = 0; // compression
ihdr[11] = 0; // filter
ihdr[12] = 0; // interlace

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw)),
  chunk('IEND', Buffer.alloc(0)),
]);

const out = fileURLToPath(new URL('../app-icon.png', import.meta.url));
writeFileSync(out, png);
console.log(`wrote ${out}`);
