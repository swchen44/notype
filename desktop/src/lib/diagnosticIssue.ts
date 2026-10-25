export interface DiagnosticPreview {
  report_id: string;
  text: string;
}

/** Only these two validated metadata values enter a browser URL. */
export function createDiagnosticIssueUrl(preview: DiagnosticPreview): string {
  const id = /^[a-f0-9]{16}-[a-f0-9]{8}$/.test(preview.report_id)
    ? preview.report_id
    : "unavailable";
  const version =
    /^version=([0-9.]{1,32})$/m.exec(preview.text)?.[1] ?? "unavailable";
  const url = new URL("https://github.com/swchen44/notype/issues/new");
  url.searchParams.set("title", "[Support] notype diagnostic report");
  url.searchParams.set(
    "body",
    [
      `notype version: ${version}`,
      `Diagnostic ID: ${id}`,
      "",
      "What happened, and what did you expect?",
      "",
      "Steps to reproduce:",
      "1. ",
      "",
      "After reviewing it, attach the exported notype-diagnostics .txt file using the GitHub attachment button or drag and drop. Attaching uploads the file to GitHub immediately. Do not attach raw logs or private content.",
    ].join("\n"),
  );
  return url.toString();
}
