use super::{get_tray_translations, TRANSLATIONS};

#[test]
fn tray_locale_resolves_both_chinese_scripts_and_fallbacks() {
    for (locale, expected) in [
        ("zh-Hant-TW", "zh-TW"),
        ("zh-Hant-HK", "zh-TW"),
        ("zh-HK", "zh-TW"),
        ("zh-MO", "zh-TW"),
        ("ZH-TW", "zh-TW"),
        ("zh_Hant_TW", "zh-TW"),
        ("zh-Hans-CN", "zh-CN"),
        ("yue-Hant-HK", "zh-TW"),
        ("yue-Hans-CN", "zh-CN"),
        ("fr-FR", "fr"),
        ("xx-YY", "zh-TW"),
    ] {
        assert_eq!(
            format!("{:?}", get_tray_translations(Some(locale.into()))),
            format!("{:?}", TRANSLATIONS[expected]),
            "{locale}"
        );
    }
}
