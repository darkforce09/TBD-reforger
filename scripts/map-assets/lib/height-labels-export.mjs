/**
 * T-152.7 — height label gate helpers (G2–G6).
 */

export const PEAK_LABEL_MAX = 48;

export function readUint16BE(data, byteOffset) {
  return (data[byteOffset] << 8) | data[byteOffset + 1];
}

export function demMetersFromPng(png, minM, maxM) {
  const { width, height, data } = png;
  const meters = new Float32Array(width * height);
  for (let i = 0; i < width * height; i++) {
    const u16 = readUint16BE(data, i * 2);
    meters[i] = minM + (u16 / 65535) * (maxM - minM);
  }
  return { width, height, meters };
}

export function heightLabelMinSepM(deckZoom) {
  return 80 * 2 ** -deckZoom;
}

/**
 * @param {Array<{x:number,y:number,value_m:number,kind?:string}>} labels
 * @param {number} deckZoom
 */
export function verifyHeightLabelGates(labels, deckZoom = 0) {
  const errors = [];
  if (!Array.isArray(labels)) {
    return ["labels not an array"];
  }
  if (labels.length > PEAK_LABEL_MAX) {
    errors.push(`G5 cap: count ${labels.length} > ${PEAK_LABEL_MAX}`);
  }
  const maxVal = labels.reduce((m, l) => Math.max(m, l.value_m ?? 0), -Infinity);
  if (maxVal < 350) {
    errors.push(`G6 coverage: max(value_m)=${maxVal} < 350`);
  }
  const sep = heightLabelMinSepM(deckZoom);
  for (let i = 0; i < labels.length; i++) {
    const a = labels[i];
    if (!(a.value_m > 0)) {
      errors.push(`G3 sea: label ${i} value_m=${a.value_m}`);
    }
    for (let j = i + 1; j < labels.length; j++) {
      const b = labels[j];
      const dx = a.x - b.x;
      const dy = a.y - b.y;
      const d = Math.hypot(dx, dy);
      if (d < sep) {
        errors.push(`G4 declutter: pair (${i},${j}) dist=${d.toFixed(1)} < ${sep}`);
      }
    }
  }
  return errors;
}
