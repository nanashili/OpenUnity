//! Deterministic search over caller-supplied labels.
//!
//! This service does not read the filesystem or interpret Unity search syntax.

/// Returns the original indices of labels matching `query`, in ranked order.
///
/// Matching uses Unicode lowercase conversion (`str::to_lowercase`), not full
/// Unicode case folding or normalization. An exact match ranks before a prefix
/// match, which ranks before a substring match. Ties preserve input order.
/// Whitespace and punctuation are literal query characters; an empty query
/// returns every index in input order.
///
/// The caller supplies the labels and controls their bounds. All matches are
/// returned without truncation. Each label is lowercased once for a nonempty
/// query, and only matching indices are retained.
pub fn ranked_indices<'a>(query: &str, labels: impl IntoIterator<Item = &'a str>) -> Vec<usize> {
    if query.is_empty() {
        return labels
            .into_iter()
            .enumerate()
            .map(|(index, _)| index)
            .collect();
    }

    let query = query.to_lowercase();
    let mut exact = Vec::new();
    let mut prefix = Vec::new();
    let mut substring = Vec::new();
    for (index, label) in labels.into_iter().enumerate() {
        let label = label.to_lowercase();
        if label == query {
            exact.push(index);
        } else if label.starts_with(&query) {
            prefix.push(index);
        } else if label.contains(&query) {
            substring.push(index);
        }
    }
    exact.extend(prefix);
    exact.extend(substring);
    exact
}

#[cfg(test)]
mod tests {
    use super::ranked_indices;

    #[test]
    fn ranks_match_strength_and_preserves_ties() {
        let labels = [
            "Main Camera",
            "Camera Rig",
            "CAMERA",
            "Side Camera",
            "camera",
            "camera two",
            "Cube",
        ];
        assert_eq!(ranked_indices("Camera", labels), [2, 4, 1, 5, 0, 3]);
    }

    #[test]
    fn lowercases_unicode_without_case_folding_or_normalization() {
        assert_eq!(
            ranked_indices("ÄPFEL", ["Grüne Äpfel", "äpfel", "Äpfelbaum"]),
            [1, 2, 0]
        );
        assert_eq!(ranked_indices("STRASSE", ["Straße", "Strasse"]), [1]);
        assert_eq!(ranked_indices("é", ["e\u{301}", "É"]), [1]);
    }

    #[test]
    fn empty_query_preserves_all_indices_including_empty_labels() {
        assert_eq!(ranked_indices("", ["z", "", "A", "A"]), [0, 1, 2, 3]);
        assert!(ranked_indices("", std::iter::empty()).is_empty());
    }

    #[test]
    fn filters_nonmatches_and_treats_query_syntax_literally() {
        assert!(ranked_indices("missing", ["Camera", "", "Cube"]).is_empty());
        assert_eq!(
            ranked_indices("t:Cube", ["Cube", "t:Cube", " t:Cube "]),
            [1, 2]
        );
        assert_eq!(ranked_indices(" ", ["Cube", "Camera Rig", ""]), [1]);
        assert!(ranked_indices("Camera", std::iter::empty()).is_empty());
    }
}
