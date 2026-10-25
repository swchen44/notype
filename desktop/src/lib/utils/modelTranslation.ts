import type { i18n } from "i18next";
import type { ModelInfo } from "@/bindings";

type ModelTranslations = Pick<i18n, "t" | "exists">;

/** Only translate existing catalog entries; missing-key handlers must not hide metadata. */
export function getTranslatedModelName(
  model: ModelInfo,
  translations: ModelTranslations,
): string {
  const key = `onboarding.models.${model.id}.name`;
  return translations.exists(key)
    ? translations.t(key) || model.name
    : model.name;
}

/** Present the catalog purpose when this model has no localized description. */
export function getTranslatedModelDescription(
  model: ModelInfo,
  translations: ModelTranslations,
): string {
  const key = model.is_custom
    ? "onboarding.customModelDescription"
    : `onboarding.models.${model.id}.description`;
  return translations.exists(key)
    ? translations.t(key) || model.description
    : model.description;
}
