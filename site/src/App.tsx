import { useMemo, useState } from "react";
import {
  Input,
  Label,
  Link,
  ListBox,
  ListBoxItem,
  Popover,
  SearchField,
  Select,
  SelectValue,
  ToggleButton,
  ToggleButtonGroup,
  Button,
  type Key,
} from "react-aria-components";

import {
  type Confidence,
  type LawKind,
  type Rule,
  type Warning,
  article,
  confidenceLabel,
  confidenceNote,
  lawKind,
  originText,
  origins,
  provision,
  report,
  ruleLabel,
  ruleNote,
  rules,
  status,
} from "./data";
import { type Piece, compare } from "./diff";

const confidences: Confidence[] = ["high", "medium", "low"];
const kinds: ("すべて" | LawKind)[] = ["すべて", "法律", "政令・勅令", "府省令・規則"];
const order: Record<Confidence, number> = { high: 0, medium: 1, low: 2 };

const badge: Record<Confidence, string> = {
  high: "bg-red-100 text-red-900 dark:bg-red-950 dark:text-red-200",
  medium: "bg-amber-100 text-amber-900 dark:bg-amber-950 dark:text-amber-200",
  low: "bg-stone-200 text-stone-700 dark:bg-stone-700 dark:text-stone-200",
};

const focusRing = "outline-none data-[focus-visible]:ring-2 data-[focus-visible]:ring-blue-600";

function Pieces({ pieces }: { pieces: Piece[] }) {
  return (
    <>
      {pieces.map((p, i) =>
        p.kind === "differs" ? (
          <mark key={i} className="rounded-xs bg-yellow-200 text-inherit dark:bg-yellow-700/60">
            {p.text}
          </mark>
        ) : (
          <span key={i} className={p.kind === "outside" ? "text-stone-400 dark:text-stone-500" : undefined}>
            {p.text}
          </span>
        ),
      )}
    </>
  );
}

const lead = "mt-2 text-sm text-stone-700 dark:text-stone-300";
const term = "text-xs leading-6 text-stone-500 dark:text-stone-400";
const quote = "font-serif leading-6 break-all";

function Emphasized({ text, part }: { text: string; part: string }) {
  const at = text.indexOf(part);
  if (at === -1 || part === "") {
    return <>{text}</>;
  }
  return (
    <>
      {text.slice(0, at)}
      <mark className="rounded-xs bg-yellow-200 text-inherit dark:bg-yellow-700/60">{part}</mark>
      {text.slice(at + part.length)}
    </>
  );
}

function actual(text: string): string {
  const items = /^(\S+)\/(\d+): (\d+) items(?:, item (\d+) of them)?$/.exec(text);
  if (items !== null) {
    const [, num, paragraph, count, position] = items;
    const where = `${article(num ?? "")}第${paragraph}項は${count}号まで`;
    return position === undefined ? where : `${where}。この参照は${position}番目の号にある`;
  }
  const paragraphs = /^(\S+): (\d+) paragraphs$/.exec(text);
  if (paragraphs !== null) {
    return `${article(paragraphs[1] ?? "")}は${paragraphs[2]}項まで`;
  }
  return text;
}

function ReadAs({ warning }: { warning: Warning }) {
  const diff = useMemo(() => compare(warning.Quoted, warning.Nearest), [warning]);
  const target =
    warning.TargetLawTitle === undefined
      ? warning.Targets.map(provision).join("・")
      : `${warning.TargetLawTitle} ${warning.Targets.map(provision).join("・")}`;
  return (
    <>
      <p className={lead}>
        <span className="font-medium">{provision(warning.Source)}</span> の読替えが、
        <span className="font-medium">{target}</span> の文言として次を引用している。
        <span className="ml-1 text-stone-500 dark:text-stone-400">({status(warning)})</span>
      </p>
      <dl className="mt-3 grid gap-x-3 gap-y-2 text-sm sm:grid-cols-[7rem_1fr]">
        <dt className={term}>読替えの引用</dt>
        <dd className={quote}>
          「<Pieces pieces={diff.quoted} />」
        </dd>
        {warning.Nearest.length > 0 && (
          <>
            <dt className={term}>引用先の近い文</dt>
            <dd className={quote}>
              <Pieces pieces={diff.nearest} />
            </dd>
          </>
        )}
      </dl>
    </>
  );
}

function Reference({ warning }: { warning: Warning }) {
  const unit = warning.Rule === "missing-unit";
  return (
    <>
      <p className={lead}>
        <span className="font-medium">{provision(warning.Source)}</span> の「{warning.Quoted}」
        {unit ? "は、条番号のあとの単位の字がない。" : "が指す先がない。"}
      </p>
      <dl className="mt-3 grid gap-x-3 gap-y-2 text-sm sm:grid-cols-[7rem_1fr]">
        <dt className={term}>条文</dt>
        <dd className={quote}>
          …<Emphasized text={warning.Context ?? ""} part={warning.Quoted} />…
        </dd>
        {!unit && (
          <>
            <dt className={term}>実際の構造</dt>
            <dd className="leading-6">{actual(warning.Actual ?? "")}</dd>
          </>
        )}
      </dl>
    </>
  );
}

function Numbering({ warning }: { warning: Warning }) {
  const where =
    warning.Status === "duplicate-paragraph" ? article(warning.Source.Article) : provision(warning.Source);
  const what =
    warning.Status === "duplicate-paragraph"
      ? `第${warning.Quoted}項が 2 つある。`
      : warning.Status === "duplicate-item"
        ? `第${warning.Quoted.replaceAll("_", "の")}号が 2 つある。`
        : `号の番号が第${(warning.Actual ?? "").replace("after item ", "")}号の次に第${warning.Quoted}号へ飛ぶ。`;
  return (
    <>
      <p className={lead}>
        <span className="font-medium">{where}</span> に{what}
      </p>
      <dl className="mt-3 grid gap-x-3 gap-y-2 text-sm sm:grid-cols-[7rem_1fr]">
        <dt className={term}>2 つ目の文</dt>
        <dd className={quote}>{warning.Context}…</dd>
      </dl>
    </>
  );
}

function Toc({ warning }: { warning: Warning }) {
  const [first, last] = (warning.Actual ?? "").split("..");
  return (
    <>
      <p className={lead}>
        {warning.Status === "toc-malformed"
          ? "目次の条の範囲に「第」のない条番号がある。"
          : "目次が書く条の範囲の外に、この章・節の条がある。"}
      </p>
      <dl className="mt-3 grid gap-x-3 gap-y-2 text-sm sm:grid-cols-[7rem_1fr]">
        <dt className={term}>目次</dt>
        <dd className={quote}>
          {warning.Context}
          {warning.Quoted}
        </dd>
        <dt className={term}>本則の条</dt>
        <dd className="leading-6">
          {article(first ?? "")} から {article(last ?? "")} まで
        </dd>
      </dl>
    </>
  );
}

function bracket(text: string): string {
  const open = /^(.) not closed$/.exec(text);
  if (open !== null) {
    return `「${open[1]}」が閉じていない`;
  }
  const close = /^(.) without (.)$/.exec(text);
  return close === null ? text : `「${close[1]}」に対応する「${close[2]}」がない`;
}

function Words({ warning }: { warning: Warning }) {
  const here = <span className="font-medium">{provision(warning.Source)}</span>;
  const actual = warning.Actual ?? "";
  const sentence = (() => {
    switch (warning.Rule) {
      case "referenced-term":
        return (
          <>
            {here} の「{warning.Quoted}」が指す第{actual}項に、この語がない。
          </>
        );
      case "brackets":
        return (
          <>
            {here} で、{bracket(actual)}。
          </>
        );
      case "spelling":
        return (
          <>
            {here} の「{warning.Quoted}」は、同じ法令のほかの箇所と表記が違う。
          </>
        );
      case "unused-definition":
        return (
          <>
            {here} が定義する「{warning.Quoted}」が、定義のほかに出てこない。
          </>
        );
      case "near-term":
        return (
          <>
            {here} の「{warning.Quoted}」は、定義された語「{actual}」と 1 字違う。
          </>
        );
      default:
        return (
          <>
            {article(actual)} が「この条において」と定義する「{warning.Quoted}」が、{here} で使われている。
          </>
        );
    }
  })();
  const spelling = warning.Rule === "spelling";
  return (
    <>
      <p className={lead}>{sentence}</p>
      <dl className="mt-3 grid gap-x-3 gap-y-2 text-sm sm:grid-cols-[7rem_1fr]">
        {(warning.Context ?? "") !== "" && (
          <>
            <dt className={term}>条文</dt>
            <dd className={quote}>
              …<Emphasized text={warning.Context ?? ""} part={warning.Quoted} />…
            </dd>
          </>
        )}
        {spelling && (
          <>
            <dt className={term}>出現</dt>
            <dd className="leading-6">{actual.replaceAll(" times", " 回")}</dd>
          </>
        )}
      </dl>
    </>
  );
}

function Body({ warning }: { warning: Warning }) {
  switch (warning.Rule) {
    case "read-as-quote":
    case "read-as-quote-external":
      return <ReadAs warning={warning} />;
    case "relative-ref":
    case "missing-unit":
      return <Reference warning={warning} />;
    case "numbering":
      return <Numbering warning={warning} />;
    case "toc-range":
      return <Toc warning={warning} />;
    default:
      return <Words warning={warning} />;
  }
}

function Card({ warning }: { warning: Warning }) {
  const origin = origins.get(warning.Id);
  return (
    <li
      id={warning.Id}
      className="rounded-lg border border-stone-200 bg-white p-4 dark:border-stone-700 dark:bg-stone-800"
    >
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <span className={`rounded px-1.5 py-0.5 text-xs font-semibold ${badge[warning.Confidence]}`}>
          確度 {confidenceLabel[warning.Confidence]}
        </span>
        <span className="rounded border border-stone-300 px-1.5 py-0.5 text-xs text-stone-600 dark:border-stone-600 dark:text-stone-300">
          {ruleLabel[warning.Rule]}
        </span>
        <Link
          href={`https://laws.e-gov.go.jp/law/${warning.LawId}`}
          target="_blank"
          rel="noreferrer"
          className={`font-semibold text-blue-800 underline decoration-blue-800/30 underline-offset-2 data-[hovered]:decoration-blue-800 dark:text-blue-300 ${focusRing}`}
        >
          {warning.LawTitle}
        </Link>
      </div>
      <Body warning={warning} />
      {origin !== undefined && (
        <p className="mt-3 text-xs leading-5 text-stone-600 dark:text-stone-300">{originText(origin)}</p>
      )}
      <p className="mt-3 text-xs text-stone-500 dark:text-stone-400">
        <a href={`#${warning.Id}`} className="font-mono">
          {warning.Id}
        </a>
      </p>
    </li>
  );
}

export function App() {
  const [query, setQuery] = useState("");
  const [shown, setShown] = useState<Set<Key>>(new Set(["high", "medium"]));
  const [kind, setKind] = useState<Key>("すべて");
  const [rule, setRule] = useState<Key>("すべて");

  const counts = useMemo(() => {
    const out: Record<Confidence, number> = { high: 0, medium: 0, low: 0 };
    for (const w of report.Warnings) {
      out[w.Confidence] += 1;
    }
    return out;
  }, []);

  const rows = useMemo(() => {
    const q = query.trim();
    return report.Warnings.filter(
      (w) =>
        shown.has(w.Confidence) &&
        (rule === "すべて" || w.Rule === rule) &&
        (kind === "すべて" || lawKind(w.LawId) === kind) &&
        (q === "" || w.LawTitle.includes(q) || w.Quoted.includes(q) || w.Id.startsWith(q)),
    ).sort(
      (a, b) =>
        order[a.Confidence] - order[b.Confidence] ||
        rules.indexOf(a.Rule) - rules.indexOf(b.Rule) ||
        a.LawTitle.localeCompare(b.LawTitle, "ja") ||
        a.Id.localeCompare(b.Id),
    );
  }, [query, shown, kind, rule]);

  const byRule = useMemo(() => {
    const out = new Map<Rule, number>();
    for (const w of report.Warnings) {
      if (shown.has(w.Confidence)) {
        out.set(w.Rule, (out.get(w.Rule) ?? 0) + 1);
      }
    }
    return out;
  }, [shown]);

  const laws = useMemo(() => new Set(rows.map((w) => w.LawId)).size, [rows]);

  return (
    <div className="mx-auto max-w-4xl px-4 py-10">
      <header>
        <h1 className="text-3xl font-bold tracking-tight">ritsulint</h1>
        <p className="mt-1 text-stone-600 dark:text-stone-300">日本の法令の linter</p>
        <div className="mt-6 space-y-3 text-sm leading-7 text-stone-700 dark:text-stone-300">
          <p>
            法令は、ほかの規定を名前でなく位置 (第○条第○項)
            と文言の引用で指します。規定の一方だけが改正されると、指す先や引用が合わなくなります。ritsulint は{" "}
            <Link
              href="https://laws.e-gov.go.jp/"
              target="_blank"
              rel="noreferrer"
              className={`text-blue-800 underline underline-offset-2 dark:text-blue-300 ${focusRing}`}
            >
              e-Gov 法令検索
            </Link>{" "}
            の法令 XML を機械で読み、そのような箇所を警告として出します。
          </p>
          <p>
            {report.AsOf} 時点で施行されている {report.Laws.toLocaleString()} 法令の本則が対象で、警告は{" "}
            {report.Warnings.length.toLocaleString()} 件です。読替えは、句{" "}
            {report.Counts.Clauses.toLocaleString()} 件のうち同じ法令の条・項に解決できた{" "}
            {report.Counts.Checked.toLocaleString()} 件と、別の法令を特定できた{" "}
            {report.ExternalChecked.toLocaleString()} 件を照合しています。
          </p>
          <p className="rounded-md border border-stone-300 bg-stone-100 px-3 py-2 dark:border-stone-600 dark:bg-stone-800">
            警告は e-Gov
            のデータの上の不一致です。官報に載った法令そのものの誤りかどうかは、改正法令の原文との照合が要ります。確度「中」「低」は誤検出を含みます。
          </p>
        </div>
      </header>

      <section aria-label="絞り込み" className="mt-8 flex flex-wrap items-end gap-x-6 gap-y-4">
        <div>
          <span id="confidence-label" className="block text-xs text-stone-600 dark:text-stone-400">
            確度
          </span>
          <ToggleButtonGroup
            aria-labelledby="confidence-label"
            selectionMode="multiple"
            selectedKeys={shown}
            onSelectionChange={setShown}
            className="mt-1 flex"
          >
            {confidences.map((c) => (
              <ToggleButton
                key={c}
                id={c}
                className={`-ml-px h-8 border border-stone-300 bg-white px-3 text-sm first:ml-0 first:rounded-l-md last:rounded-r-md data-[hovered]:bg-stone-100 data-[selected]:border-stone-800 data-[selected]:bg-stone-800 data-[selected]:text-white dark:border-stone-600 dark:bg-stone-800 dark:data-[hovered]:bg-stone-700 dark:data-[selected]:border-stone-200 dark:data-[selected]:bg-stone-200 dark:data-[selected]:text-stone-900 ${focusRing} data-[focus-visible]:z-10`}
              >
                {confidenceLabel[c]} <span className="tabular-nums opacity-70">{counts[c]}</span>
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </div>
        <Select name="rule" selectedKey={rule} onSelectionChange={(k) => setRule(k ?? "すべて")}>
          <Label className="block text-xs text-stone-600 dark:text-stone-400">検査</Label>
          <Button
            className={`mt-1 flex h-8 w-56 items-center justify-between rounded-md border border-stone-300 bg-white px-3 text-sm data-[hovered]:bg-stone-100 dark:border-stone-600 dark:bg-stone-800 dark:data-[hovered]:bg-stone-700 ${focusRing}`}
          >
            <SelectValue />
            <span aria-hidden="true" className="text-xs">
              ▾
            </span>
          </Button>
          <Popover className="w-(--trigger-width) rounded-md border border-stone-300 bg-white p-1 shadow-lg dark:border-stone-600 dark:bg-stone-800">
            <ListBox className="outline-none">
              {(["すべて", ...rules] as const).map((r) => (
                <ListBoxItem
                  key={r}
                  id={r}
                  textValue={r === "すべて" ? r : ruleLabel[r]}
                  className="flex cursor-default justify-between rounded px-2 py-1.5 text-sm text-stone-900 outline-none data-[focused]:bg-stone-100 data-[selected]:font-semibold dark:text-stone-100 dark:data-[focused]:bg-stone-700"
                >
                  {r === "すべて" ? r : ruleLabel[r]}
                  {r !== "すべて" && <span className="tabular-nums opacity-70">{byRule.get(r) ?? 0}</span>}
                </ListBoxItem>
              ))}
            </ListBox>
          </Popover>
        </Select>
        <Select name="kind" selectedKey={kind} onSelectionChange={(k) => setKind(k ?? "すべて")}>
          <Label className="block text-xs text-stone-600 dark:text-stone-400">法令の種類</Label>
          <Button
            className={`mt-1 flex h-8 w-40 items-center justify-between rounded-md border border-stone-300 bg-white px-3 text-sm data-[hovered]:bg-stone-100 dark:border-stone-600 dark:bg-stone-800 dark:data-[hovered]:bg-stone-700 ${focusRing}`}
          >
            <SelectValue />
            <span aria-hidden="true" className="text-xs">
              ▾
            </span>
          </Button>
          <Popover className="w-(--trigger-width) rounded-md border border-stone-300 bg-white p-1 shadow-lg dark:border-stone-600 dark:bg-stone-800">
            <ListBox className="outline-none">
              {kinds.map((k) => (
                <ListBoxItem
                  key={k}
                  id={k}
                  className="cursor-default rounded px-2 py-1.5 text-sm text-stone-900 outline-none data-[focused]:bg-stone-100 data-[selected]:font-semibold dark:text-stone-100 dark:data-[focused]:bg-stone-700"
                >
                  {k}
                </ListBoxItem>
              ))}
            </ListBox>
          </Popover>
        </Select>
        <SearchField value={query} onChange={setQuery} className="min-w-48 flex-1">
          <Label className="block text-xs text-stone-600 dark:text-stone-400">法令名・引用の文言</Label>
          <Input
            name="q"
            placeholder="例: 施行規則"
            className={`mt-1 h-8 w-full rounded-md border border-stone-300 bg-white px-3 text-sm placeholder:text-stone-400 dark:border-stone-600 dark:bg-stone-800 [&::-webkit-search-cancel-button]:hidden ${focusRing}`}
          />
        </SearchField>
      </section>

      <ul className="mt-3 space-y-0.5 text-xs text-stone-600 dark:text-stone-400">
        {confidences.map((c) => (
          <li key={c}>
            確度 {confidenceLabel[c]}: {confidenceNote[c]}
          </li>
        ))}
      </ul>

      {rule !== "すべて" && (
        <p className="mt-4 rounded-md bg-stone-100 px-3 py-2 text-sm leading-6 text-stone-700 dark:bg-stone-800 dark:text-stone-300">
          {ruleLabel[rule as Rule]}: {ruleNote[rule as Rule]}
        </p>
      )}

      <p className="mt-6 text-sm text-stone-700 dark:text-stone-300" aria-live="polite">
        {rows.length.toLocaleString()} 件・{laws.toLocaleString()} 法令
      </p>
      <ol className="mt-3 space-y-3">
        {rows.map((w) => (
          <Card key={w.Id} warning={w} />
        ))}
      </ol>

      <footer className="mt-12 border-t border-stone-200 pt-6 text-xs leading-6 text-stone-500 dark:border-stone-700 dark:text-stone-400">
        <p>
          法令のデータは e-Gov 法令検索の一括ダウンロード ({report.AsOf} 取得)
          によります。条文の文言は各法令のリンク先で確かめられます。
        </p>
        <p>
          作成:{" "}
          <Link href="https://s.conao3.com/" className={`underline underline-offset-2 ${focusRing}`}>
            conao3
          </Link>
        </p>
      </footer>
    </div>
  );
}
