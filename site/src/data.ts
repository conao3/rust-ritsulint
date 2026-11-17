import raw from "../data/warnings.json";

export type Provision = { Article: string; Paragraph: number };
export type Confidence = "high" | "medium" | "low";
export type Status = "absent" | "other-paragraph" | "no-paragraph" | "no-article";

export type Warning = {
  Id: string;
  Status: Status;
  LawId: string;
  LawTitle: string;
  Source: Provision;
  Scope: string;
  Targets: Provision[];
  Quoted: string;
  Replacement: string;
  Nearest: string;
  Similarity: number;
  Confidence: Confidence;
};

type Report = {
  AsOf: string;
  Laws: number;
  Counts: { Clauses: number; Checked: number; Warnings: number; External: number; Unsupported: number };
  Warnings: Warning[];
};

export const report = raw as Report;

export const confidenceLabel: Record<Confidence, string> = { high: "高", medium: "中", low: "低" };

export const confidenceNote: Record<Confidence, string> = {
  high: "条番号を直接書いた参照。全件を条文で読んで確かめた",
  medium: "「同項」「前条」などの相対の参照。参照先の解決違いを含む",
  low: "別の法令の条を指している疑いが強い",
};

export const statusLabel: Record<Status, string> = {
  absent: "引用先に文言がない",
  "other-paragraph": "同じ条の別の項にある",
  "no-paragraph": "引用先の項がない",
  "no-article": "引用先の条がない",
};

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

export function provision(p: Provision): string {
  const [head, ...branches] = p.Article.split("_");
  const article = `第${head ?? ""}条${branches.map((b) => `の${b}`).join("")}`;
  return p.Paragraph > 0 ? `${article}第${p.Paragraph}項` : article;
}
