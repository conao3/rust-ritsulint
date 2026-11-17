import rawOrigins from "../data/origins.json";
import raw from "../data/warnings.json";

export type Provision = { Article: string; Paragraph: number };
export type Confidence = "high" | "medium" | "low";
export type Rule =
  | "read-as-quote"
  | "read-as-quote-external"
  | "relative-ref"
  | "missing-unit"
  | "numbering"
  | "toc-range"
  | "referenced-term"
  | "brackets"
  | "spelling"
  | "unused-definition"
  | "near-term"
  | "definition-scope";

export type Warning = {
  Id: string;
  Rule: Rule;
  Status: string;
  LawId: string;
  LawTitle: string;
  Source: Provision;
  Scope: string;
  Targets: Provision[];
  Quoted: string;
  Nearest: string;
  Confidence: Confidence;
  Context?: string;
  Actual?: string;
  TargetLawId?: string;
  TargetLawTitle?: string;
};

type Report = {
  AsOf: string;
  Laws: number;
  LawsWithoutArticles: number;
  Counts: { Clauses: number; Checked: number };
  ExternalChecked: number;
  Warnings: Warning[];
};

export type Origin = { Id: string; FirstRevisionId: string; Kind: string };

export const report = raw as Report;

export const origins = new Map((rawOrigins as { Origins: Origin[] }).Origins.map((o) => [o.Id, o]));

export const rules: Rule[] = [
  "read-as-quote",
  "read-as-quote-external",
  "relative-ref",
  "missing-unit",
  "numbering",
  "toc-range",
  "referenced-term",
  "brackets",
  "spelling",
  "unused-definition",
  "near-term",
  "definition-scope",
];

export const ruleLabel: Record<Rule, string> = {
  "read-as-quote": "読替えの引用",
  "read-as-quote-external": "読替えの引用 (別の法令)",
  "relative-ref": "相対参照",
  "missing-unit": "単位の脱字",
  numbering: "番号",
  "toc-range": "目次",
  "referenced-term": "参照先にない語",
  brackets: "括弧の対応",
  spelling: "表記の不統一",
  "unused-definition": "使われない定義",
  "near-term": "定義語と 1 字違い",
  "definition-scope": "定義の範囲の外",
};

export const ruleNote: Record<Rule, string> = {
  "read-as-quote":
    "「第○条中「A」とあるのは「B」と読み替える」の A が、同じ法令の引用先の条・項にない。引用先だけが改正されると生じる",
  "read-as-quote-external":
    "別の法令の条を引用する読替えの A が、その法令の条・項にない。法令名・法令番号・定義された略称で法令を特定できた句が対象",
  "relative-ref":
    "「前条第二項」「次項」「前三号」のように位置で指す参照の先に、項や号がない。間に条を足す改正で指す先がずれると生じる",
  "missing-unit": "「第九十八第二項」のように、条番号のあとの「条」がない参照",
  numbering: "同じ条の中に同じ番号の項、同じ項の中に同じ番号の号が 2 つある。または号の番号が飛んでいる",
  "toc-range": "目次が章・節に書く条の範囲「（第 A 条―第 B 条）」の外に、その章・節の条がある",
  "referenced-term":
    "「前項に規定する X」「第 N 項に規定する X」の X が、指している項の文にない。X が 5 字以上の語のときだけ見る",
  brackets: "1 つの項の中で、鉤括弧か丸括弧の開きと閉じが対応していない。読替え規定の項は見ない",
  spelling:
    "同じ法令の中で、同じ片仮名の語を小書きの字 (ッ、ャ、ァ) と大きい字 (ツ、ヤ、ア) の両方で書いている。ほかの法令の題名を元の表記で引いている場合を含む",
  "unused-definition": "「以下「X」という。」で定義した語が、その法令の定義のほかに出てこない",
  "near-term":
    "定義した語と 1 字だけ違う語が、まれに使われている。別の語であることが多く、誤検出が大半を占める",
  "definition-scope":
    "「以下この条において「X」という。」と定義した語が、定義のない別の条で使われている。一般の語であることが多く、誤検出が大半を占める",
};

export const confidenceLabel: Record<Confidence, string> = { high: "高", medium: "中", low: "低" };

export const confidenceNote: Record<Confidence, string> = {
  high: "条文を読んで確かめたもの。番号・目次・脱字は条文の構造だけで決まる",
  medium: "誤検出を含む。参照先や法令の解決違い、同じ意味の言い換えなど",
  low: "誤検出が大半を占める見込みのもの",
};

const statusLabel: Record<string, string> = {
  absent: "引用先に文言がない",
  "other-paragraph": "同じ条の別の項にある",
  "no-paragraph": "引用先の項がない",
  "no-article": "引用先の条がない",
};

export function status(w: Warning): string {
  return statusLabel[w.Status] ?? w.Status;
}

export type LawKind = "法律" | "政令・勅令" | "府省令・規則";

export function lawKind(lawId: string): LawKind {
  const code = /^\d{3}([A-Z]+)/.exec(lawId)?.[1] ?? "";
  if (code.startsWith("AC")) {
    return "法律";
  }
  if (code.startsWith("CO") || code.startsWith("IO")) {
    return "政令・勅令";
  }
  return "府省令・規則";
}

export function article(num: string): string {
  if (num === "") {
    return "本則";
  }
  const [head, ...branches] = num.split("_");
  return `第${head ?? ""}条${branches.map((b) => `の${b}`).join("")}`;
}

export function provision(p: Provision): string {
  if (p.Article === "") {
    return p.Paragraph > 0 ? `第${p.Paragraph}項` : "本則";
  }
  return p.Paragraph > 0 ? `${article(p.Article)}第${p.Paragraph}項` : article(p.Article);
}

const eras: Record<string, string> = { "1": "明治", "2": "大正", "3": "昭和", "4": "平成", "5": "令和" };

function amendment(id: string): string {
  const m = /^(\d)(\d{2})([A-Z]+)(\d+)$/.exec(id);
  if (m === null) {
    return id;
  }
  const [, era, year, code, digits] = m;
  const law = code?.startsWith("AC") || code?.startsWith("CO") || code?.startsWith("IO");
  const kind = code?.startsWith("AC")
    ? "法律"
    : code?.startsWith("CO")
      ? "政令"
      : code?.startsWith("IO")
        ? "勅令"
        : "省令・規則";
  const number = Number(law ? digits : digits?.slice(-3));
  return `${eras[era ?? ""] ?? ""}${Number(year) === 1 ? "元" : Number(year)}年${kind}第${number}号`;
}

const originLabel: Record<string, string> = {
  "target-changed": "この改正で引用先の文言が変わり、読替えの引用が残った",
  "clause-new": "読替えの句がこの版で書かれた",
  renumbered: "この版で引用先の条・項の番号が変わった",
  "was-unchecked": "この版から照合できる書き方になった",
};

export function originText(o: Origin): string {
  const [, date, amend] = o.FirstRevisionId.split("_");
  const day = date === undefined ? "" : `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)}`;
  if (o.Kind === "enacted") {
    return `制定時 (${day} 施行) から`;
  }
  if (o.Kind === "oldest") {
    return `e-Gov が持つ最古の版 (${day} 施行) から。起点の改正は版の履歴では決まらない`;
  }
  const since = `${day} 施行の版 (${amendment(amend ?? "")}) から`;
  const label = originLabel[o.Kind];
  return label === undefined ? since : `${since}。${label}`;
}
