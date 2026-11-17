export type Piece = { text: string; kind: "same" | "differs" | "outside" };

const LIMIT = 400;

function pieces(text: string, kept: boolean[], margin: { before: number; after: number } | null): Piece[] {
  const first = kept.indexOf(true);
  const last = kept.lastIndexOf(true);
  const out: Piece[] = [];
  for (let i = 0; i < text.length; i++) {
    const outside = margin !== null && (first === -1 || i < first - margin.before || i > last + margin.after);
    const kind = outside ? "outside" : kept[i] === true ? "same" : "differs";
    const tail = out.at(-1);
    if (tail !== undefined && tail.kind === kind) {
      tail.text += text.charAt(i);
    } else {
      out.push({ text: text.charAt(i), kind });
    }
  }
  return out;
}

export function compare(quoted: string, nearest: string): { quoted: Piece[]; nearest: Piece[] } {
  if (nearest.length === 0 || quoted.length > LIMIT || nearest.length > LIMIT) {
    return { quoted: [{ text: quoted, kind: "same" }], nearest: [{ text: nearest, kind: "same" }] };
  }
  const n = quoted.length;
  const m = nearest.length;
  const width = m + 1;
  const table = new Uint16Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      table[i * width + j] =
        quoted.charAt(i) === nearest.charAt(j)
          ? (table[(i + 1) * width + j + 1] ?? 0) + 1
          : Math.max(table[(i + 1) * width + j] ?? 0, table[i * width + j + 1] ?? 0);
    }
  }
  const keptQuoted = Array.from({ length: n }, () => false);
  const keptNearest = Array.from({ length: m }, () => false);
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (quoted.charAt(i) === nearest.charAt(j)) {
      keptQuoted[i] = true;
      keptNearest[j] = true;
      i++;
      j++;
    } else if ((table[(i + 1) * width + j] ?? 0) >= (table[i * width + j + 1] ?? 0)) {
      i++;
    } else {
      j++;
    }
  }
  const before = Math.max(keptQuoted.indexOf(true), 0);
  const after = n - 1 - keptQuoted.lastIndexOf(true);
  return {
    quoted: pieces(quoted, keptQuoted, null),
    nearest: pieces(nearest, keptNearest, { before, after }),
  };
}
