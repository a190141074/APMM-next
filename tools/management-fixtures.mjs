// 生成大小固定且 CRC 完整的 PNG，供同大小编辑和真实解码验收使用。
import { deflateSync } from "node:zlib";
const crc = (bytes) => {
  let n = 0xffffffff;
  for (const b of bytes) {
    n ^= b;
    for (let i = 0; i < 8; i++) n = (n >>> 1) ^ (n & 1 ? 0xedb88320 : 0);
  }
  return (n ^ 0xffffffff) >>> 0;
};
const chunk = (name, data) => {
  const type = Buffer.from(name),
    length = Buffer.alloc(4),
    sum = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  sum.writeUInt32BE(crc(Buffer.concat([type, data])));
  return Buffer.concat([length, type, data, sum]);
};
export const png = (color) => {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(2, 0);
  ihdr.writeUInt32BE(2, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const row = Buffer.from([0, color, 30, 40, 255, color, 30, 40, 255]);
  return Buffer.concat([
    Buffer.from("89504e470d0a1a0a", "hex"),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(Buffer.concat([row, row]), { level: 0 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
};
