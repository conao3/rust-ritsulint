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
  type Warning,
  confidenceLabel,
  confidenceNote,
  lawKind,
  provision,
  report,
  statusLabel,
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

function Card({ warning }: { warning: Warning }) {
  const diff = useMemo(() => compare(warning.Quoted, warning.Nearest), [warning]);
  return (
    <li
      id={warning.Id}
      className="rounded-lg border border-stone-200 bg-white p-4 dark:border-stone-700 dark:bg-stone-800"
    >
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <span className={`rounded px-1.5 py-0.5 text-xs font-semibold ${badge[warning.Confidence]}`}>
          確度 {confidenceLabel[warning.Confidence]}
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
      <p className="mt-2 text-sm text-stone-700 dark:text-stone-300">
        <span className="font-medium">{provision(warning.Source)}</span> の読替えが、
        <span className="font-medium">{warning.Targets.map(provision).join("・")}</span>{" "}
        の文言として次を引用している。
        <span className="ml-1 text-stone-500 dark:text-stone-400">({statusLabel[warning.Status]})</span>
      </p>
      <dl className="mt-3 grid gap-x-3 gap-y-2 text-sm sm:grid-cols-[7rem_1fr]">
        <dt className="text-xs leading-6 text-stone-500 dark:text-stone-400">読替えの引用</dt>
        <dd className="font-serif leading-6 break-all">
          「<Pieces pieces={diff.quoted} />」
        </dd>
        {warning.Nearest.length > 0 && (
          <>
            <dt className="text-xs leading-6 text-stone-500 dark:text-stone-400">引用先の近い文</dt>
            <dd className="font-serif leading-6 break-all">
              <Pieces pieces={diff.nearest} />
            </dd>
          </>
        )}
      </dl>
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
        (kind === "すべて" || lawKind(w.LawId) === kind) &&
        (q === "" || w.LawTitle.includes(q) || w.Quoted.includes(q) || w.Id.startsWith(q)),
    ).sort(
      (a, b) =>
        order[a.Confidence] - order[b.Confidence] ||
        a.LawTitle.localeCompare(b.LawTitle, "ja") ||
        a.Id.localeCompare(b.Id),
    );
  }, [query, shown, kind]);

  const laws = useMemo(() => new Set(rows.map((w) => w.LawId)).size, [rows]);

  return (
    <div className="mx-auto max-w-4xl px-4 py-10">
      <header>
        <h1 className="text-3xl font-bold tracking-tight">ritsulint</h1>
        <p className="mt-1 text-stone-600 dark:text-stone-300">日本の法令の linter</p>
        <div className="mt-6 space-y-3 text-sm leading-7 text-stone-700 dark:text-stone-300">
          <p>
            法令には「第○条中「A」とあるのは「B」と読み替える」という読替え規定があります。A
            は引用先の条文の文言をそのまま写したものなので、引用先だけが改正されると、A
            が引用先のどこにもない状態になります。ritsulint は{" "}
            <Link
              href="https://laws.e-gov.go.jp/"
              target="_blank"
              rel="noreferrer"
              className={`text-blue-800 underline underline-offset-2 dark:text-blue-300 ${focusRing}`}
            >
              e-Gov 法令検索
            </Link>{" "}
            の法令 XML を読み、そのような箇所を警告として出します。
          </p>
          <p>
            {report.AsOf} 時点で施行されている {report.Laws.toLocaleString()} 法令の本則が対象です。読替えの句{" "}
            {report.Counts.Clauses.toLocaleString()} 件のうち、同じ法令の条・項に解決できた{" "}
            {report.Counts.Checked.toLocaleString()} 件を照合し、{report.Counts.Warnings.toLocaleString()}{" "}
            件が警告になりました。別の法令を指す句と、解決できない書き方の句は照合していません。
          </p>
          <p className="rounded-md border border-stone-300 bg-stone-100 px-3 py-2 dark:border-stone-600 dark:bg-stone-800">
            警告は e-Gov
            のデータの上の不一致です。官報に載った法令そのものの誤りかどうかは、改正法令の原文との照合が要ります。確度「中」「低」は、参照先の解決違いによる誤検出を含みます。
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
