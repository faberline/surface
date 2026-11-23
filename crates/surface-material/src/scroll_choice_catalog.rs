//! Finite ordered text choices and locale-specific direct-input matching.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollChoiceItem<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub detail: Option<&'a str>,
    pub aliases: &'a [&'a str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollChoice<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub detail: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScrollChoiceCatalogError {
    TooFewChoices,
    TooManyChoices,
    BlankId { index: usize },
    DuplicateId { id: String },
    BlankLabel { id: String },
    NoMatchKeys { id: String },
    UnknownCommittedId { id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollChoiceMatchKind {
    Exact,
    Prefix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollChoiceMatch<'a> {
    pub kind: ScrollChoiceMatchKind,
    pub choice: ScrollChoice<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollChoiceQueryError {
    Empty,
    NoMatch,
    Ambiguous {
        kind: ScrollChoiceMatchKind,
        count: usize,
    },
}

type MatchKey = dyn Fn(&str) -> String;

/// Owns the choice data. The caller holds the committed ID and any input draft.
pub struct ScrollChoiceCatalog {
    choices: Vec<OwnedChoice>,
    match_key: Box<MatchKey>,
}

struct OwnedChoice {
    id: String,
    label: String,
    detail: Option<String>,
    keys: Vec<String>,
}

impl OwnedChoice {
    fn as_choice(&self) -> ScrollChoice<'_> {
        ScrollChoice {
            id: &self.id,
            label: &self.label,
            detail: self.detail.as_deref(),
        }
    }
}

impl ScrollChoiceCatalog {
    /// Use trimmed, case-sensitive Unicode matching by default.
    pub fn new<'a>(
        items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
        committed_id: &str,
    ) -> Result<Self, ScrollChoiceCatalogError> {
        Self::with_match_key(items, committed_id, str::to_owned)
    }

    /// Use a deterministic caller-supplied key for labels, aliases and queries.
    /// The catalog trims each input and key. Choice IDs and visible copy remain
    /// as authored. Configure locale-specific case, accent or width folding here.
    pub fn with_match_key<'a>(
        items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
        committed_id: &str,
        match_key: impl Fn(&str) -> String + 'static,
    ) -> Result<Self, ScrollChoiceCatalogError> {
        let choices = validated_choices(items, committed_id, &match_key)?;
        Ok(Self {
            choices,
            match_key: Box::new(match_key),
        })
    }

    /// Publish new data only after checking all choices and the supplied ID.
    /// Uses the current key function. A locale policy change can replace the
    /// entire catalog with a newly validated one.
    pub fn replace_items<'a>(
        &mut self,
        items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
        committed_id: &str,
    ) -> Result<(), ScrollChoiceCatalogError> {
        self.choices = validated_choices(items, committed_id, self.match_key.as_ref())?;
        Ok(())
    }

    /// Current authored order. The editor can use some of these labels and
    /// details for localized no-match help without accepting any candidate.
    pub fn items(&self) -> impl ExactSizeIterator<Item = ScrollChoice<'_>> {
        self.choices.iter().map(OwnedChoice::as_choice)
    }

    /// Resolve an ordinal against the currently published catalog.
    pub fn at(&self, index: usize) -> Option<ScrollChoice<'_>> {
        self.choices.get(index).map(OwnedChoice::as_choice)
    }

    /// Find a current index by application-owned identity.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.choices.iter().position(|choice| choice.id == id)
    }

    /// Exact matches take precedence, even when more than one ID matches.
    /// A unique prefix returns a suggestion for the caller to accept explicitly.
    pub fn match_query(&self, raw: &str) -> Result<ScrollChoiceMatch<'_>, ScrollChoiceQueryError> {
        if raw.trim().is_empty() {
            return Err(ScrollChoiceQueryError::Empty);
        }
        let query = normalized_key(self.match_key.as_ref(), raw);
        if query.is_empty() {
            return Err(ScrollChoiceQueryError::NoMatch);
        }
        if let Some(exact) = self.find_match(&query, ScrollChoiceMatchKind::Exact)? {
            return Ok(exact);
        }
        self.find_match(&query, ScrollChoiceMatchKind::Prefix)?
            .ok_or(ScrollChoiceQueryError::NoMatch)
    }

    fn find_match(
        &self,
        query: &str,
        kind: ScrollChoiceMatchKind,
    ) -> Result<Option<ScrollChoiceMatch<'_>>, ScrollChoiceQueryError> {
        let mut matches = self.choices.iter().filter(|choice| {
            choice.keys.iter().any(|key| match kind {
                ScrollChoiceMatchKind::Exact => key == query,
                ScrollChoiceMatchKind::Prefix => key.starts_with(query),
            })
        });
        let Some(first) = matches.next() else {
            return Ok(None);
        };
        let count = 1 + matches.count();
        if count > 1 {
            return Err(ScrollChoiceQueryError::Ambiguous { kind, count });
        }
        Ok(Some(ScrollChoiceMatch {
            kind,
            choice: first.as_choice(),
        }))
    }
}

fn validated_choices<'a>(
    items: impl IntoIterator<Item = ScrollChoiceItem<'a>>,
    committed_id: &str,
    match_key: &MatchKey,
) -> Result<Vec<OwnedChoice>, ScrollChoiceCatalogError> {
    use ScrollChoiceCatalogError::*;

    let mut ids = HashSet::new();
    let mut choices = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        if i32::try_from(index).is_err() {
            return Err(TooManyChoices);
        }
        if item.id.trim().is_empty() {
            return Err(BlankId { index });
        }
        if !ids.insert(item.id) {
            return Err(DuplicateId {
                id: item.id.to_owned(),
            });
        }
        if item.label.trim().is_empty() {
            return Err(BlankLabel {
                id: item.id.to_owned(),
            });
        }
        let mut keys = Vec::new();
        for name in std::iter::once(item.label).chain(item.aliases.iter().copied()) {
            let key = normalized_key(match_key, name);
            if !key.is_empty() && !keys.contains(&key) {
                keys.push(key);
            }
        }
        if keys.is_empty() {
            return Err(NoMatchKeys {
                id: item.id.to_owned(),
            });
        }
        choices.push(OwnedChoice {
            id: item.id.to_owned(),
            label: item.label.to_owned(),
            detail: item.detail.map(str::to_owned),
            keys,
        });
    }
    if choices.len() < 2 {
        return Err(TooFewChoices);
    }
    if !ids.contains(committed_id) {
        return Err(UnknownCommittedId {
            id: committed_id.to_owned(),
        });
    }
    Ok(choices)
}

fn normalized_key(match_key: &MatchKey, raw: &str) -> String {
    let key = match_key(raw.trim());
    if key.trim().len() == key.len() {
        key
    } else {
        key.trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ScrollChoiceCatalogError as CatalogError;
    use ScrollChoiceMatchKind::{Exact, Prefix};
    use ScrollChoiceQueryError as QueryError;

    fn item<'a>(id: &'a str, label: &'a str, aliases: &'a [&'a str]) -> ScrollChoiceItem<'a> {
        ScrollChoiceItem {
            id,
            label,
            detail: None,
            aliases,
        }
    }

    #[test]
    fn validates_nonblank_unique_ids_and_the_committed_id() {
        let north = item("north", "North", &[]);
        let south = item("south", "South", &[]);
        assert!(matches!(
            ScrollChoiceCatalog::new([north], "north"),
            Err(CatalogError::TooFewChoices)
        ));
        assert!(matches!(
            ScrollChoiceCatalog::new([north, north], "north"),
            Err(CatalogError::DuplicateId { id }) if id == "north"
        ));
        assert!(matches!(
            ScrollChoiceCatalog::new([north, item(" \t", "South", &[])], "north"),
            Err(CatalogError::BlankId { index: 1 })
        ));
        assert!(matches!(
            ScrollChoiceCatalog::new([north, item("south", " ", &[])], "north"),
            Err(CatalogError::BlankLabel { id }) if id == "south"
        ));
        assert!(matches!(
            ScrollChoiceCatalog::new([north, south], "North"),
            Err(CatalogError::UnknownCommittedId { id }) if id == "North"
        ));
    }

    #[test]
    fn exact_labels_and_aliases_take_priority_over_longer_prefixes() {
        let catalog = ScrollChoiceCatalog::new(
            [
                item("north-large", "Northbound", &["Northeast"]),
                item("north", "North", &["N"]),
                item("south", "South", &[]),
            ],
            "north",
        )
        .unwrap();
        for exact in [" North ", "N"] {
            let matched = catalog.match_query(exact).unwrap();
            assert_eq!((matched.kind, matched.choice.id), (Exact, "north"));
        }
        assert_eq!(catalog.match_query("S").unwrap().kind, Prefix);
        assert_eq!(catalog.match_query("n"), Err(QueryError::NoMatch));
    }

    #[test]
    fn one_cjk_character_can_offer_a_unique_suggestion() {
        let catalog = ScrollChoiceCatalog::new(
            [
                item("near", "近距離", &[]),
                item("far", "遠距離", &[]),
                item("high", "高處", &[]),
            ],
            "near",
        )
        .unwrap();
        let suggestion = catalog.match_query("遠").unwrap();
        assert_eq!(suggestion.kind, Prefix);
        assert_eq!(
            (suggestion.choice.id, suggestion.choice.label),
            ("far", "遠距離")
        );
        assert_eq!(catalog.index_of(suggestion.choice.id), Some(1));
    }

    #[test]
    fn ambiguity_counts_ids_instead_of_aliases_and_names() {
        let catalog = ScrollChoiceCatalog::with_match_key(
            [
                ScrollChoiceItem {
                    detail: Some("Maine"),
                    ..item(
                        "portland-me",
                        "Portland",
                        &["Portland", "PORTLAND MAINE", "Portland ME"],
                    )
                },
                ScrollChoiceItem {
                    detail: Some("Oregon"),
                    ..item(
                        "portland-or",
                        "Portland",
                        &["Portland Oregon", "Portland OR"],
                    )
                },
                item("port", "Portland Harbor", &[]),
            ],
            "portland-or",
            str::to_lowercase,
        )
        .unwrap();
        assert_eq!(
            catalog.match_query("Portland"),
            Err(QueryError::Ambiguous {
                kind: Exact,
                count: 2
            })
        );
        assert_eq!(
            catalog.match_query("Por"),
            Err(QueryError::Ambiguous {
                kind: Prefix,
                count: 3
            })
        );
        let exact = catalog.match_query("portland OR").unwrap();
        assert_eq!(
            (exact.kind, exact.choice.id, exact.choice.detail),
            (Exact, "portland-or", Some("Oregon"))
        );
        let prefix = catalog.match_query("portland M").unwrap();
        assert_eq!((prefix.kind, prefix.choice.id), (Prefix, "portland-me"));
    }

    #[test]
    fn caller_match_keys_allow_locale_rules_without_changing_identity_or_copy() {
        fn turkish(s: &str) -> String {
            s.replace('I', "ı").replace('İ', "i").to_lowercase()
        }
        let catalog = ScrollChoiceCatalog::with_match_key(
            [
                item("ankara", "Işık", &["Işıklar"]),
                item("istanbul", "İstanbul", &[]),
            ],
            "ankara",
            turkish,
        )
        .unwrap();
        assert_eq!(catalog.match_query(" i ").unwrap().choice.id, "istanbul");
        assert_eq!(catalog.match_query("ı").unwrap().choice.label, "Işık");
        assert_eq!(catalog.match_query("ışıklar").unwrap().kind, Exact);
    }

    #[test]
    fn no_match_is_distinct_from_blank_input_and_empty_aliases_do_not_match() {
        let items = [
            item("near", "Near", &["", "    ", "!"]),
            item("far", "Far", &[]),
        ];
        let catalog =
            ScrollChoiceCatalog::with_match_key(items, "near", |value| value.replace('!', ""))
                .unwrap();
        assert_eq!(catalog.match_query("\t　"), Err(QueryError::Empty));
        assert_eq!(catalog.match_query("!"), Err(QueryError::NoMatch));
        assert_eq!(catalog.match_query("Other"), Err(QueryError::NoMatch));
        assert_eq!(
            catalog
                .items()
                .take(2)
                .map(|choice| choice.label)
                .collect::<Vec<_>>(),
            ["Near", "Far"]
        );
        assert!(matches!(
            ScrollChoiceCatalog::with_match_key(items, "near", |_| String::new()),
            Err(CatalogError::NoMatchKeys { id }) if id == "near"
        ));
    }

    #[test]
    fn replacement_is_atomic_and_resolves_the_same_id_after_reorder() {
        let mut catalog = ScrollChoiceCatalog::new(
            [item("north", "North", &[]), item("south", "South", &[])],
            "north",
        )
        .unwrap();
        let renamed = item("south", "Sud", &[]);
        assert!(catalog.replace_items([renamed, renamed], "south").is_err());
        assert_eq!(catalog.at(0).unwrap().id, "north");
        assert_eq!(catalog.match_query("South").unwrap().choice.id, "south");

        catalog
            .replace_items([renamed, item("north", "Nord", &[])], "north")
            .unwrap();
        assert_eq!(catalog.index_of("north"), Some(1));
        assert_eq!(catalog.at(1).unwrap().label, "Nord");
        assert_eq!(catalog.match_query("North"), Err(QueryError::NoMatch));
        assert_eq!(catalog.match_query("No").unwrap().choice.id, "north");
    }
}
