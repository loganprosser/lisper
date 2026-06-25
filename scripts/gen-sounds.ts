import { writeFileSync } from "fs";

const SR = 44100;

function wav(samples: Float32Array): Buffer {
  const n = samples.length;
  const buf = Buffer.alloc(44 + n * 2);
  buf.write("RIFF", 0);
  buf.writeUInt32LE(36 + n * 2, 4);
  buf.write("WAVE", 8);
  buf.write("fmt ", 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(1, 22);
  buf.writeUInt32LE(SR, 24);
  buf.writeUInt32LE(SR * 2, 28);
  buf.writeUInt16LE(2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write("data", 36);
  buf.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    buf.writeInt16LE((s * 32767) | 0, 44 + i * 2);
  }
  return buf;
}

// Deep click: low sine (~120Hz) + short noise transient, fast exponential decay
function deepClick(durSec: number, freq: number): Float32Array {
  const n = Math.floor(SR * durSec);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const env = Math.exp(-t * 32);
    const body = Math.sin(2 * Math.PI * freq * t) * env;
    const transient =
      ((Math.sin(i * 12.9898) * 43758.5453) % 1) * Math.exp(-t * 220) * 0.4;
    out[i] = (body * 0.8 + transient) * 0.9;
  }
  return out;
}

writeFileSync(
  "src-tauri/resources/lisper_start.wav",
  wav(deepClick(0.18, 120)),
);
writeFileSync("src-tauri/resources/lisper_stop.wav", wav(deepClick(0.12, 90)));
console.log("sounds generated");
