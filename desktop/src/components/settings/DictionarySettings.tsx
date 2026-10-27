import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import {
  dictionaryCsv,
  mergeWords,
  normalizeWord,
  parseDictionaryCsv,
  validWord,
  wordIdentity,
} from "@/lib/dictionary";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export function DictionarySettings() {
  const { t } = useTranslation();
  const { getSetting, refreshSettings, isLoading } = useSettings();
  const words = getSetting("custom_words") ?? [];
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(new Set<string>());
  const [editor, setEditor] = useState<{
    original: string | null;
    value: string;
  } | null>(null);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const file = useRef<HTMLInputElement>(null);
  const visible = words.filter((word) =>
    wordIdentity(word).includes(query.trim().toLowerCase()),
  );
  const persist = async (next: string[]) => {
    const result = await commands.updateCustomWords(next);
    if (result.status !== "ok") throw new Error("dictionary-save-failed");
    await refreshSettings();
    setSelected(new Set());
    return true;
  };
  const mutate = async (operation: () => Promise<boolean>) => {
    if (lock.current) return false;
    lock.current = true;
    setBusy(true);
    setMessage("");
    try {
      return await operation();
    } catch {
      setMessage(t("notype.usability.saveFailed"));
      return false;
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  const save = (next: string[]) => mutate(() => persist(next));
  const edit = async () => {
    if (!editor) return;
    const word = normalizeWord(editor.value);
    if (!validWord(word)) {
      setMessage(t("notype.usability.wordInvalid"));
      return;
    }
    const others = words.filter((value) => value !== editor.original);
    if (others.some((value) => wordIdentity(value) === wordIdentity(word))) {
      setMessage(t("notype.usability.duplicate"));
      return;
    }
    const next =
      editor.original === null
        ? [...words, word]
        : words.map((value) => (value === editor.original ? word : value));
    if (await save(next)) setEditor(null);
  };
  const importCsv = (input: File) =>
    mutate(async () => {
      let merged: ReturnType<typeof mergeWords>;
      try {
        if (input.size > 1024 * 1024) throw new Error("dictionary-csv-invalid");
        const csv = new TextDecoder("utf-8", { fatal: true }).decode(
          await input.arrayBuffer(),
        );
        merged = mergeWords(words, parseDictionaryCsv(csv));
      } catch {
        setMessage(t("notype.usability.csvInvalid"));
        return false;
      }
      await persist(merged.words);
      setEditor(null);
      setMessage(
        t("notype.usability.imported", {
          count: merged.added,
          duplicates: merged.duplicates,
        }),
      );
      return true;
    });
  const exportCsv = () => {
    const blob = new Blob([dictionaryCsv(words)], {
      type: "text/csv;charset=utf-8",
    });
    const url = URL.createObjectURL(blob),
      anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "notype-dictionary.csv";
    anchor.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
  return (
    <section
      className="max-w-4xl w-full mx-auto space-y-4"
      aria-label={t("notype.usability.dictionary")}
    >
      <div className="flex flex-wrap justify-between gap-3 items-center">
        <h2 className="text-2xl font-semibold">
          {t("notype.usability.dictionary")}
        </h2>
        <div className="flex gap-2">
          <Button
            disabled={busy || isLoading}
            onClick={() => {
              setEditor({ original: null, value: "" });
              setMessage("");
            }}
          >
            {t("notype.usability.addWord")}
          </Button>
          <Button
            variant="secondary"
            disabled={busy || isLoading}
            onClick={() => file.current?.click()}
          >
            {t("notype.usability.importCsv")}
          </Button>
          <Button variant="secondary" disabled={busy} onClick={exportCsv}>
            {t("notype.usability.exportCsv")}
          </Button>
        </div>
      </div>
      <p className="text-sm text-text/60">{t("notype.usability.csvFormat")}</p>
      <input
        ref={file}
        type="file"
        accept=".csv,text/csv"
        className="hidden"
        aria-label={t("notype.usability.importCsv")}
        onChange={(event) => {
          const chosen = event.target.files?.[0];
          event.target.value = "";
          if (chosen) void importCsv(chosen);
        }}
      />
      <Input
        type="search"
        value={query}
        aria-label={t("notype.usability.searchWords")}
        placeholder={t("notype.usability.searchWords")}
        onChange={(event) => setQuery(event.target.value)}
      />
      {selected.size > 0 && (
        <Button
          variant="danger-ghost"
          disabled={busy}
          onClick={() => void save(words.filter((word) => !selected.has(word)))}
        >
          {t("notype.usability.deleteSelected", { count: selected.size })}
        </Button>
      )}
      <p role="status">{message}</p>
      <table className="w-full text-left border-collapse">
        <thead>
          <tr className="border-b border-mid-gray/30">
            <th className="p-2">
              <input
                type="checkbox"
                aria-label={t("notype.usability.selectVisible")}
                checked={
                  visible.length > 0 &&
                  visible.every((word) => selected.has(word))
                }
                disabled={busy || !visible.length}
                onChange={(event) =>
                  setSelected((current) => {
                    const next = new Set(current);
                    for (const word of visible) {
                      if (event.target.checked) next.add(word);
                      else next.delete(word);
                    }
                    return next;
                  })
                }
              />
            </th>
            <th className="p-2">{t("notype.usability.word")}</th>
            <th className="p-2">{t("notype.usability.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {visible.map((word) => (
            <tr key={word} className="border-b border-mid-gray/20">
              <td className="p-2">
                <input
                  type="checkbox"
                  aria-label={t("notype.usability.selectWord", { word })}
                  checked={selected.has(word)}
                  disabled={busy}
                  onChange={(event) =>
                    setSelected((current) => {
                      const next = new Set(current);
                      if (event.target.checked) next.add(word);
                      else next.delete(word);
                      return next;
                    })
                  }
                />
              </td>
              <td className="p-2 break-all">{word}</td>
              <td className="p-2">
                <div className="flex gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() => {
                      setEditor({ original: word, value: word });
                      setMessage("");
                    }}
                  >
                    {t("notype.connections.edit")}
                  </Button>
                  <Button
                    variant="danger-ghost"
                    size="sm"
                    disabled={busy}
                    onClick={() =>
                      void save(words.filter((value) => value !== word))
                    }
                  >
                    {t("notype.history.delete")}
                  </Button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {!visible.length && (
        <p>
          {t(
            query
              ? "notype.usability.noMatchingWords"
              : "notype.usability.noWords",
          )}
        </p>
      )}
      {editor && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <section
            role="dialog"
            aria-modal="true"
            aria-label={t(
              editor.original === null
                ? "notype.usability.addWord"
                : "notype.usability.editWord",
            )}
            className="bg-background border border-mid-gray/40 rounded-2xl p-6 space-y-4 w-full max-w-lg"
          >
            <h3 className="text-xl">
              {t(
                editor.original === null
                  ? "notype.usability.addWord"
                  : "notype.usability.editWord",
              )}
            </h3>
            <Input
              autoFocus
              disabled={busy}
              value={editor.value}
              aria-label={t("notype.usability.word")}
              onChange={(event) =>
                setEditor({ ...editor, value: event.target.value })
              }
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.nativeEvent.isComposing)
                  void edit();
                if (event.key === "Escape" && !busy) setEditor(null);
              }}
            />
            <p role="status">{message}</p>
            <div className="flex justify-between gap-2">
              <Button
                variant="secondary"
                disabled={busy}
                onClick={() => file.current?.click()}
              >
                {t("notype.usability.importCsv")}
              </Button>
              <div className="flex gap-2">
                <Button
                  variant="secondary"
                  disabled={busy}
                  onClick={() => setEditor(null)}
                >
                  {t("notype.usability.cancel")}
                </Button>
                <Button
                  disabled={busy || !validWord(normalizeWord(editor.value))}
                  onClick={() => void edit()}
                >
                  {t("notype.usability.save")}
                </Button>
              </div>
            </div>
          </section>
        </div>
      )}
    </section>
  );
}
