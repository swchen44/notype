import { useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  AddressMode,
  CapabilityDeclaration,
  ConnectionDraft,
  ConnectionKind,
  ConnectionView,
  KeyAction,
  ProviderDialect,
} from "@/bindings";
import {
  connectionDraft,
  endpointPreview,
  optionalNumber,
  maskApiKey,
} from "@/lib/providerForm";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { ModelIdList } from "./ModelIdList";

export function ConnectionEditor({
  connection,
  revision,
  busy,
  onSave,
}: {
  connection: ConnectionView;
  revision: number;
  busy: boolean;
  onSave: (draft: ConnectionDraft, keyAction: KeyAction) => Promise<boolean>;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(() => connectionDraft(connection));
  const [keyMode, setKeyMode] = useState<KeyAction["action"]>("keep");
  const [replacement, setReplacement] = useState("");
  const [temperature, setTemperature] = useState(
    connection.temperature?.toString() ?? "",
  );
  const [topP, setTopP] = useState(connection.top_p?.toString() ?? "");
  const [output, setOutput] = useState(
    connection.max_output_tokens?.toString() ?? "",
  );
  const [timeout, setTimeout] = useState(
    (connection.timeout_ms / 1000).toString(),
  );
  const [efforts, setEfforts] = useState(
    connection.capability_declaration?.supported_efforts.join(", ") ?? "",
  );
  const [minimum, setMinimum] = useState(
    connection.capability_declaration?.minimum_effort ?? "",
  );

  const edit = (value: Partial<ConnectionDraft>) =>
    setDraft((current) => ({ ...current, ...value }));
  const parsedTemperature = optionalNumber(temperature, { min: 0, max: 2 });
  const parsedTopP = optionalNumber(topP, {
    min: 0,
    max: 1,
    exclusiveMin: true,
  });
  const parsedOutput = optionalNumber(output, {
    min: 1,
    max: 1000000,
    integer: true,
  });
  const parsedTimeout = optionalNumber(timeout, { min: 0.001, max: 120 });
  const numbersValid =
    parsedTemperature.valid &&
    parsedTopP.valid &&
    parsedOutput.valid &&
    parsedTimeout.valid &&
    parsedTimeout.value !== null;
  const declaredEfforts = efforts
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
  const declarationValid =
    !draft.capability_declaration ||
    (declaredEfforts.every((value) => /^[a-z0-9_-]{1,32}$/.test(value)) &&
      (draft.capability_declaration.kind !== "mandatory" ||
        (minimum.length > 0 && declaredEfforts.includes(minimum))));
  const declaration: CapabilityDeclaration | null = draft.capability_declaration
    ? {
        ...draft.capability_declaration,
        supported_efforts: declaredEfforts,
        minimum_effort:
          draft.capability_declaration.kind === "mandatory" ? minimum : null,
      }
    : null;
  const candidate: ConnectionDraft = {
    ...draft,
    capability_declaration: declaration,
    temperature: parsedTemperature.value,
    top_p: parsedTopP.value,
    max_output_tokens: parsedOutput.value,
    timeout_ms:
      parsedTimeout.value === null ? 0 : Math.round(parsedTimeout.value * 1000),
  };
  const endpoint = endpointPreview(draft);
  const routeUnchanged =
    draft.kind === connection.kind &&
    draft.dialect === connection.dialect &&
    draft.authentication === connection.authentication &&
    endpoint === connection.effective_endpoint &&
    draft.models_endpoint === connection.models_endpoint;
  const bindingInvalid =
    connection.app_key_present && keyMode === "keep" && !routeUnchanged;
  const replacementValid =
    keyMode !== "replace" ||
    (replacement.trim().length > 0 && !/[\r\n\0]/.test(replacement));
  const nameValid =
    draft.display_name.trim().length > 0 &&
    Array.from(draft.display_name.trim()).length <= 128 &&
    !/[\x00-\x1f\x7f]/.test(draft.display_name);
  const unsupported =
    (declaration &&
      !declaration.temperature_supported &&
      parsedTemperature.value !== null) ||
    (declaration && !declaration.top_p_supported && parsedTopP.value !== null);
  const effortValid =
    draft.effort === null || /^[a-z0-9_-]{1,32}$/.test(draft.effort);
  const valid =
    numbersValid &&
    declarationValid &&
    replacementValid &&
    nameValid &&
    !bindingInvalid &&
    !unsupported &&
    effortValid &&
    endpoint !== null;
  const sameModel =
    draft.model === connection.model &&
    draft.dialect === connection.dialect &&
    draft.kind === connection.kind &&
    endpoint === connection.effective_endpoint;
  const supportedEfforts =
    declaration?.supported_efforts ??
    (sameModel ? (connection.resolution?.supported_efforts ?? []) : []);
  const resolutionCurrent =
    sameModel &&
    numbersValid &&
    draft.thinking_preference === connection.thinking_preference &&
    draft.effort === connection.effort &&
    candidate.temperature === connection.temperature &&
    candidate.top_p === connection.top_p &&
    JSON.stringify(declaration) ===
      JSON.stringify(connection.capability_declaration);
  const resolution = resolutionCurrent ? connection.resolution : null;
  const keyAction: KeyAction =
    keyMode === "replace"
      ? { action: "replace", value: replacement }
      : { action: keyMode };

  return (
    <section
      aria-label={t("notype.connections.editor")}
      className="space-y-4 border-t border-mid-gray/30 pt-4"
    >
      <h3 className="font-semibold">{t("notype.connections.editor")}</h3>
      <p className="text-sm text-text/70">
        {t("notype.connections.editorHint")}
      </p>
      <fieldset disabled={busy} className="space-y-3 min-w-0">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.name")}
            <Input
              aria-label={t("notype.connections.name")}
              value={draft.display_name}
              onChange={(event) => edit({ display_name: event.target.value })}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.kind")}
            <select
              aria-label={t("notype.connections.kind")}
              className="border rounded p-2 bg-background"
              value={draft.kind}
              onChange={(event) =>
                edit({ kind: event.target.value as ConnectionKind })
              }
            >
              {(["provider", "custom", "local"] as const).map((value) => (
                <option key={value} value={value}>
                  {t("notype.connections.kinds." + value)}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.dialect")}
            <select
              aria-label={t("notype.connections.dialect")}
              className="border rounded p-2 bg-background"
              value={draft.dialect}
              onChange={(event) =>
                edit({ dialect: event.target.value as ProviderDialect })
              }
            >
              {(
                [
                  "generic",
                  "deepseek",
                  "openrouter",
                  "gemini",
                  "ollama_openai",
                ] as const
              ).map((value) => (
                <option key={value} value={value}>
                  {t("notype.connections.dialects." + value)}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.addressMode")}
            <select
              aria-label={t("notype.connections.addressMode")}
              className="border rounded p-2 bg-background"
              value={draft.address_mode}
              onChange={(event) =>
                edit({ address_mode: event.target.value as AddressMode })
              }
            >
              <option value="base_url">
                {t("notype.connections.baseUrl")}
              </option>
              <option value="full_chat_endpoint">
                {t("notype.connections.fullEndpoint")}
              </option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.address")}
            <Input
              aria-label={t("notype.connections.address")}
              value={draft.api_address}
              placeholder={
                draft.address_mode === "base_url"
                  ? "https://example.com/v1"
                  : "https://example.com/v1/chat/completions"
              }
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => edit({ api_address: event.target.value })}
            />
          </label>
        </div>
        <p className="text-sm text-text/70">
          {t("notype.connections.endpointPreview")}:{" "}
          {endpoint ?? t("notype.connections.invalidEndpoint")}
        </p>
        <p className="text-sm text-text/70">
          {t("notype.connections.addressHint")}
        </p>
        {draft.address_mode === "full_chat_endpoint" && (
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.modelsEndpoint")}
            <Input
              aria-label={t("notype.connections.modelsEndpoint")}
              value={draft.models_endpoint ?? ""}
              autoComplete="off"
              spellCheck={false}
              onChange={(event) =>
                edit({ models_endpoint: event.target.value || null })
              }
            />
          </label>
        )}
        {draft.kind === "local" && (
          <p role="note" className="text-sm text-text/70">
            {t("notype.connections.localHint")}
          </p>
        )}
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.authentication")}
          <select
            aria-label={t("notype.text.authentication")}
            className="border rounded p-2 bg-background"
            value={draft.authentication}
            onChange={(event) => {
              edit({
                authentication: event.target
                  .value as ConnectionDraft["authentication"],
              });
              if (event.target.value === "none") {
                setKeyMode("clear");
                setReplacement("");
              } else {
                setKeyMode("keep");
              }
            }}
          >
            <option value="none">{t("notype.text.noAuth")}</option>
            <option value="bearer">{t("notype.text.bearer")}</option>
          </select>
        </label>
        <p className="text-sm" role="note">
          {t("notype.connections.savedKeySource")}:{" "}
          {t("notype.connections.sources." + connection.credential_source)}
        </p>
        {draft.authentication === "bearer" && (
          <div className="space-y-3">
            <label className="flex flex-col gap-1 text-sm">
              {t("notype.connections.apiKey")}
              <Input
                aria-label={t("notype.connections.apiKey")}
                type="password"
                autoComplete="new-password"
                placeholder={t("notype.connections.keyPlaceholder")}
                disabled={keyMode === "clear"}
                value={replacement}
                onChange={(event) => {
                  setReplacement(event.target.value);
                  setKeyMode(event.target.value.length ? "replace" : "keep");
                }}
              />
              <span
                className="font-mono break-all"
                aria-label={t("notype.usability.keyPreview")}
              >
                {replacement ? maskApiKey(replacement) : ""}
              </span>
            </label>
            <p className="text-sm text-text/70">
              {t("notype.connections.keyEntryHint")}
            </p>
            <p className="text-sm text-text/70">
              {t("notype.connections.keyPriority")}
            </p>
            <label className="flex flex-col gap-1 text-sm">
              {t("notype.connections.keyAction")}
              <select
                aria-label={t("notype.connections.keyAction")}
                className="border rounded p-2 bg-background"
                value={keyMode}
                onChange={(event) => {
                  setKeyMode(event.target.value as KeyAction["action"]);
                  if (event.target.value !== "replace") setReplacement("");
                }}
              >
                <option value="keep">{t("notype.connections.keepKey")}</option>
                <option value="clear">
                  {t("notype.connections.clearKey")}
                </option>
                <option value="replace">
                  {t("notype.connections.replaceKey")}
                </option>
              </select>
            </label>
            {bindingInvalid && (
              <p role="alert" className="text-sm">
                {t("notype.connections.bindingChanged")}
              </p>
            )}
            {keyMode === "keep" && connection.masked_api_key && (
              <p className="font-mono break-all">
                {t("notype.usability.savedKey")}: {connection.masked_api_key}
              </p>
            )}
          </div>
        )}
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.model")}
          <Input
            aria-label={t("notype.text.model")}
            value={draft.model}
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => edit({ model: event.target.value })}
          />
        </label>
        <ModelIdList
          connection={candidate}
          keyAction={keyAction}
          revision={revision}
          disabled={
            busy ||
            !numbersValid ||
            !replacementValid ||
            bindingInvalid ||
            endpoint === null
          }
        />
        <label className="flex gap-2 text-sm">
          <input
            type="checkbox"
            aria-label={t("notype.connections.thinking")}
            checked={draft.thinking_preference === "on"}
            onChange={(event) =>
              edit({ thinking_preference: event.target.checked ? "on" : "off" })
            }
          />
          {t("notype.connections.thinking")}
        </label>
        {(declaration?.kind === "mandatory" ||
          (sameModel && connection.resolution?.capability === "mandatory")) && (
          <p role="note" className="text-sm">
            {t("notype.connections.mandatoryCapability")}
          </p>
        )}
        {resolution?.reason === "mandatory_minimum" && (
          <p role="note" className="text-sm">
            {t("notype.connections.mandatoryHint", {
              effort:
                resolution.effective_effort ?? t("notype.metrics.unknown"),
            })}
          </p>
        )}
        {resolution?.provenance === "user_declared" && (
          <p role="note" className="text-sm">
            {t("notype.connections.declaredHint")}
          </p>
        )}
        {(!resolution || resolution.capability === "unknown") && (
          <p role="note" className="text-sm">
            {t("notype.connections.unresolvedHint")}
          </p>
        )}
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.effort")}
          {supportedEfforts.length ? (
            <select
              aria-label={t("notype.text.effort")}
              className="border rounded p-2 bg-background"
              disabled={draft.thinking_preference === "off"}
              value={draft.effort ?? ""}
              onChange={(event) => edit({ effort: event.target.value || null })}
            >
              <option value="">{t("notype.metrics.providerDefault")}</option>
              {supportedEfforts
                .filter((value) => value !== "none")
                .map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
            </select>
          ) : (
            <Input
              aria-label={t("notype.text.effort")}
              disabled={draft.thinking_preference === "off"}
              value={draft.effort ?? ""}
              onChange={(event) => edit({ effort: event.target.value || null })}
            />
          )}
        </label>
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.connections.temperature")}
          <Input
            aria-label={t("notype.connections.temperature")}
            inputMode="decimal"
            aria-invalid={!parsedTemperature.valid}
            value={temperature}
            onChange={(event) => setTemperature(event.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.connections.topP")}
          <Input
            aria-label={t("notype.connections.topP")}
            inputMode="decimal"
            aria-invalid={!parsedTopP.valid}
            value={topP}
            onChange={(event) => setTopP(event.target.value)}
          />
        </label>
        <p className="text-sm text-text/70">
          {t("notype.connections.samplingHint")}
        </p>
        {(resolution?.temperature_resolution === "ignored_by_provider" ||
          resolution?.top_p_resolution === "ignored_by_provider") && (
          <p role="note" className="text-sm">
            {t("notype.connections.ignoredHint")}
          </p>
        )}
        {resolution?.documented_top_p !== null &&
          resolution?.documented_top_p !== undefined && (
            <p role="note" className="text-sm">
              {t("notype.connections.normalizedHint", {
                value: resolution.documented_top_p,
              })}
            </p>
          )}
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.maxOutput")}
          <Input
            aria-label={t("notype.text.maxOutput")}
            inputMode="numeric"
            aria-invalid={!parsedOutput.valid}
            value={output}
            onChange={(event) => setOutput(event.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.timeout")}
          <Input
            aria-label={t("notype.text.timeout")}
            inputMode="decimal"
            aria-invalid={!parsedTimeout.valid || parsedTimeout.value === null}
            value={timeout}
            onChange={(event) => setTimeout(event.target.value)}
          />
        </label>
        {!numbersValid && (
          <p role="alert" className="text-sm">
            {t("notype.connections.numberInvalid")}
          </p>
        )}
        {unsupported && (
          <p role="alert" className="text-sm">
            {t("notype.connections.unsupportedOption")}
          </p>
        )}
        <details className="space-y-3">
          <summary className="cursor-pointer">
            {t("notype.connections.capabilityTitle")}
          </summary>
          <p className="text-sm text-text/70">
            {t("notype.connections.capabilityHint")}
          </p>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.connections.capability")}
            <select
              aria-label={t("notype.connections.capability")}
              className="border rounded p-2 bg-background"
              value={draft.capability_declaration?.kind ?? "known"}
              onChange={(event) =>
                edit({
                  capability_declaration:
                    event.target.value === "known"
                      ? null
                      : {
                          kind: event.target
                            .value as CapabilityDeclaration["kind"],
                          minimum_effort: null,
                          supported_efforts: [],
                          temperature_supported: true,
                          top_p_supported: true,
                        },
                })
              }
            >
              <option value="known">
                {t("notype.connections.capabilities.known")}
              </option>
              <option value="non_reasoning">
                {t("notype.connections.capabilities.non_reasoning")}
              </option>
              <option value="optional">
                {t("notype.connections.capabilities.optional")}
              </option>
              <option value="mandatory">
                {t("notype.connections.capabilities.mandatory")}
              </option>
            </select>
          </label>
          {draft.capability_declaration && (
            <>
              <label className="flex flex-col gap-1 text-sm">
                {t("notype.connections.supportedEfforts")}
                <Input
                  aria-label={t("notype.connections.supportedEfforts")}
                  value={efforts}
                  onChange={(event) => setEfforts(event.target.value)}
                />
              </label>
              {draft.capability_declaration.kind === "mandatory" && (
                <label className="flex flex-col gap-1 text-sm">
                  {t("notype.connections.minimumEffort")}
                  <Input
                    aria-label={t("notype.connections.minimumEffort")}
                    value={minimum}
                    onChange={(event) => setMinimum(event.target.value)}
                  />
                </label>
              )}
              <label className="flex gap-2 text-sm">
                <input
                  type="checkbox"
                  aria-label={t("notype.connections.supportsTemperature")}
                  checked={draft.capability_declaration.temperature_supported}
                  onChange={(event) =>
                    edit({
                      capability_declaration: {
                        ...draft.capability_declaration!,
                        temperature_supported: event.target.checked,
                      },
                    })
                  }
                />
                {t("notype.connections.supportsTemperature")}
              </label>
              <label className="flex gap-2 text-sm">
                <input
                  type="checkbox"
                  aria-label={t("notype.connections.supportsTopP")}
                  checked={draft.capability_declaration.top_p_supported}
                  onChange={(event) =>
                    edit({
                      capability_declaration: {
                        ...draft.capability_declaration!,
                        top_p_supported: event.target.checked,
                      },
                    })
                  }
                />
                {t("notype.connections.supportsTopP")}
              </label>
            </>
          )}
          {!declarationValid && (
            <p role="alert" className="text-sm">
              {t("notype.connections.declarationInvalid")}
            </p>
          )}
        </details>
        <Button
          disabled={!valid}
          onClick={() => void onSave(candidate, keyAction)}
        >
          {t("notype.connections.save")}
        </Button>
      </fieldset>
    </section>
  );
}
