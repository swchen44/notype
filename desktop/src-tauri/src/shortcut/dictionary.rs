//! Validation boundary shared by single-word edits and bulk imports.
pub fn validated_words(words: Vec<String>) -> Result<Vec<String>, &'static str> {
    if words.len() > 50_000
        || words
            .iter()
            .try_fold(0usize, |size, word| size.checked_add(word.len()))
            .is_none_or(|size| size > 1_048_576)
    {
        return Err("dictionary-words-invalid");
    }
    let mut identities = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for word in words {
        let word = word.trim();
        if word.is_empty() || word.chars().count() > 50 || word.chars().any(char::is_control) {
            return Err("dictionary-words-invalid");
        }
        if identities.insert(word.to_lowercase()) {
            normalized.push(word.to_owned());
        }
    }
    Ok(normalized)
}
