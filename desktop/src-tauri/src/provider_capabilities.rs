//! Exact documented facts and explicit user declarations; no model substring guesses.
use super::{
    CapabilityDeclaration, ConfigError, ConnectionDraft, ProviderDialect, RequestResolutionView,
    ThinkingPreference,
};
const EFFORTS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh", "max"];
/// Validate raw nullable controls and the finite declaration without resolving keys.
pub fn validate(draft: &ConnectionDraft) -> Result<(), ConfigError> {
    if draft
        .temperature
        .is_some_and(|value| !value.is_finite() || !(0.0..=2.0).contains(&value))
        || draft
            .top_p
            .is_some_and(|value| !value.is_finite() || value <= 0.0 || value > 1.0)
        || draft.max_output_tokens.is_some_and(|value| {
            value == 0
                || value
                    > if draft.dialect == ProviderDialect::Deepseek {
                        393_216
                    } else {
                        1_000_000
                    }
        })
        || draft
            .effort
            .as_deref()
            .is_some_and(|effort| !EFFORTS.contains(&effort))
    {
        return Err(ConfigError("configuration-invalid-options"));
    }
    if let Some(declaration) = &draft.capability_declaration {
        validate_declaration(declaration)?;
    }
    Ok(())
}
fn validate_declaration(declaration: &CapabilityDeclaration) -> Result<(), ConfigError> {
    if !["non_reasoning", "optional", "mandatory"].contains(&declaration.kind.as_str())
        || declaration.supported_efforts.len() > 7
        || declaration
            .supported_efforts
            .iter()
            .any(|effort| !EFFORTS.contains(&effort.as_str()))
        || declaration
            .supported_efforts
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != declaration.supported_efforts.len()
    {
        return Err(ConfigError("configuration-capability-invalid"));
    }
    if declaration.kind == "mandatory"
        && (declaration.minimum_effort.as_ref().is_none_or(|minimum| {
            minimum == "none" || !declaration.supported_efforts.contains(minimum)
        }))
    {
        return Err(ConfigError("configuration-capability-invalid"));
    }
    if declaration.kind != "mandatory" && declaration.minimum_effort.is_some() {
        return Err(ConfigError("configuration-capability-invalid"));
    }
    Ok(())
}
struct Fact {
    kind: &'static str,
    minimum: Option<&'static str>,
    efforts: &'static [&'static str],
}
fn documented(draft: &ConnectionDraft) -> Option<Fact> {
    match draft.dialect {
        ProviderDialect::Deepseek => Some(Fact {
            kind: "optional",
            minimum: None,
            efforts: &["none", "low", "high", "max"],
        }),
        ProviderDialect::Gemini => match draft.model.as_str() {
            "gemini-3.8-flash" => Some(Fact {
                kind: "mandatory",
                minimum: Some("low"),
                efforts: &["low", "medium", "high"],
            }),
            "gemini-3.5-flash-lite" => Some(Fact {
                kind: "mandatory",
                minimum: Some("minimal"),
                efforts: &["minimal", "low", "medium", "high"],
            }),
            _ => None,
        },
        ProviderDialect::Openrouter => match draft.model.as_str() {
            "google/gemma-4-31b-it:free" | "google/gemma-4-26b-a4b-it:free" => Some(Fact {
                kind: "optional",
                minimum: None,
                efforts: &["low", "medium", "high"],
            }),
            _ => None,
        },
        ProviderDialect::OllamaOpenai => match draft.model.as_str() {
            "gpt-oss:20b" | "gpt-oss:120b" => Some(Fact {
                kind: "mandatory",
                minimum: Some("low"),
                efforts: &["low", "medium", "high"],
            }),
            _ => None,
        },
        ProviderDialect::Generic => None,
    }
}
/// Resolve exact facts or an explicitly unverified declaration. Unknown remains
/// unresolved and cannot become a silently enabled generation configuration.
pub fn resolve(draft: &ConnectionDraft) -> Result<RequestResolutionView, ConfigError> {
    validate(draft)?;
    let fact = documented(draft);
    let declaration = draft.capability_declaration.as_ref();
    let (kind, minimum, efforts, provenance, temperature_supported, top_p_supported) =
        if let Some(fact) = fact {
            (
                fact.kind.to_owned(),
                fact.minimum.map(str::to_owned),
                fact.efforts
                    .iter()
                    .map(|effort| (*effort).to_owned())
                    .collect::<Vec<_>>(),
                "documented",
                true,
                true,
            )
        } else if let Some(declaration) = declaration {
            (
                declaration.kind.clone(),
                declaration.minimum_effort.clone(),
                declaration.supported_efforts.clone(),
                "user_declared",
                declaration.temperature_supported,
                declaration.top_p_supported,
            )
        } else {
            ("unknown".into(), None, vec![], "unknown", true, true)
        };
    let off = draft.thinking_preference == ThinkingPreference::Off;
    let (thinking, effort, reason) = match kind.as_str() {
        "unknown" => (None, None, "unresolved"),
        "non_reasoning" => (Some(false), None, "non_reasoning"),
        "mandatory" => {
            let minimum = minimum.ok_or(ConfigError("configuration-capability-invalid"))?;
            let effort = if off {
                minimum
            } else {
                draft.effort.clone().unwrap_or(minimum)
            };
            if !efforts.contains(&effort) {
                return Err(ConfigError("configuration-effort-unsupported"));
            }
            (
                Some(true),
                Some(effort),
                if off { "mandatory_minimum" } else { "enabled" },
            )
        }
        "optional" if off => (
            Some(false),
            if matches!(draft.dialect, ProviderDialect::Openrouter) {
                None
            } else {
                Some("none".into())
            },
            "disabled",
        ),
        "optional" => {
            if draft
                .effort
                .as_ref()
                .is_some_and(|effort| effort == "none" || !efforts.contains(effort))
            {
                return Err(ConfigError("configuration-effort-unsupported"));
            }
            if draft.effort.is_none()
                && matches!(
                    draft.dialect,
                    ProviderDialect::Generic
                        | ProviderDialect::Gemini
                        | ProviderDialect::OllamaOpenai
                )
            {
                return Err(ConfigError("configuration-effort-required"));
            }
            (Some(true), draft.effort.clone(), "enabled")
        }
        _ => return Err(ConfigError("configuration-capability-invalid")),
    };
    if draft.temperature.is_some() && !temperature_supported
        || draft.top_p.is_some() && !top_p_supported
    {
        return Err(ConfigError("configuration-parameter-unsupported"));
    }
    let temperature_ignored = draft.dialect == ProviderDialect::Deepseek && thinking == Some(true);
    let top_p_ignored = draft.dialect == ProviderDialect::Deepseek && thinking == Some(false);
    Ok(RequestResolutionView {
        supported_efforts: efforts,
        capability: kind,
        provenance: provenance.into(),
        effective_thinking: thinking,
        effective_effort: effort,
        reason: reason.into(),
        temperature_wire: if temperature_ignored {
            None
        } else {
            draft.temperature
        },
        top_p_wire: if top_p_ignored { None } else { draft.top_p },
        temperature_resolution: if draft.temperature.is_none() {
            "omitted"
        } else if temperature_ignored {
            "ignored_by_provider"
        } else {
            "sent"
        }
        .into(),
        top_p_resolution: if draft.top_p.is_none() {
            "omitted"
        } else if top_p_ignored {
            "ignored_by_provider"
        } else {
            "sent"
        }
        .into(),
        documented_top_p: if draft.dialect == ProviderDialect::Deepseek && thinking == Some(true) {
            draft.top_p.map(|value| value.max(0.95))
        } else {
            None
        },
    })
}
