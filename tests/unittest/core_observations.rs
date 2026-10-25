use notype_core::observations::{mixed_count, COUNT_RULE_VERSION};

#[test]
fn mixed_counts_follow_the_published_chinese_english_rule() {
    assert_eq!(COUNT_RULE_VERSION, "zh-en-v1");
    assert_eq!(mixed_count("我們用 API 測試。"), 6);
    assert_eq!(mixed_count("don't stop"), 2);
    assert_eq!(mixed_count("session_id"), 2);
    assert_eq!(mixed_count("API-v2 42"), 3);
    assert_eq!(mixed_count("𠀀 A\u{301} 👩‍💻\n"), 2);
}
