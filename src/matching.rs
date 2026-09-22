use plurimus_core::ratatui_core::style::Style;
use plurimus_core::ratatui_core::text::{Line, Span};

const START_BONUS: i32 = 2;
const BOUNDARY_BONUS: i32 = 1;
const BOUNDARIES: [char; 4] = ['.', '-', '_', ' '];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Match {
    /// Higher ranks first: a start or boundary bonus, less the gaps between
    /// hits.
    pub score: i32,
    /// Char positions of the hits in the name, ascending.
    pub indices: Vec<usize>,
}

/// Case-insensitive subsequence match taking the leftmost hit for each
/// filter character; `None` when a character is missing.
pub(crate) fn find_match(filter: &str, name: &str) -> Option<Match> {
    let mut wanted = filter.chars().map(fold).peekable();
    let mut indices = Vec::with_capacity(filter.len());
    for (index, ch) in name.chars().enumerate() {
        let Some(&want) = wanted.peek() else { break };
        if fold(ch) == want {
            wanted.next();
            indices.push(index);
        }
    }
    wanted.peek().is_none().then(|| Match {
        score: score(name, &indices),
        indices,
    })
}

fn fold(ch: char) -> char {
    ch.to_lowercase().next().unwrap_or(ch)
}

fn score(name: &str, indices: &[usize]) -> i32 {
    let (Some(&first), Some(&last)) = (indices.first(), indices.last()) else {
        return 0;
    };
    let bonus = match first.checked_sub(1) {
        None => START_BONUS,
        Some(before)
            if name
                .chars()
                .nth(before)
                .is_some_and(|ch| BOUNDARIES.contains(&ch)) =>
        {
            BOUNDARY_BONUS
        }
        Some(_) => 0,
    };
    let gaps = i32::try_from(last - first + 1 - indices.len()).unwrap_or(i32::MAX);
    bonus.saturating_sub(gaps)
}

/// The name as a line with the hit characters in `lit`.
pub(crate) fn light_matches(name: &str, indices: &[usize], lit: Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut run_is_lit = false;
    for (index, ch) in name.chars().enumerate() {
        let is_lit = indices.binary_search(&index).is_ok();
        if is_lit != run_is_lit && !run.is_empty() {
            spans.push(span(std::mem::take(&mut run), run_is_lit, lit));
        }
        run_is_lit = is_lit;
        run.push(ch);
    }
    if !run.is_empty() {
        spans.push(span(run, run_is_lit, lit));
    }
    Line::from(spans)
}

fn span(text: String, is_lit: bool, lit: Style) -> Span<'static> {
    if is_lit {
        Span::styled(text, lit)
    } else {
        Span::raw(text)
    }
}

#[cfg(test)]
mod tests {
    use super::{Match, find_match, light_matches};
    use plurimus_core::ratatui_core::style::{Modifier, Style};
    use plurimus_core::ratatui_core::text::Span;

    #[test]
    fn empty_filter_matches_everything_with_no_hits() {
        assert_eq!(
            find_match("", "anything"),
            Some(Match {
                score: 0,
                indices: vec![]
            })
        );
    }

    #[test]
    fn missing_character_is_no_match() {
        assert_eq!(find_match("led", "tab-next"), None);
    }

    #[test]
    fn prefix_outranks_a_gapped_and_an_unstarted_match() {
        let ledger = find_match("led", "ledger").unwrap();
        let gapped = find_match("led", "lxexd").unwrap();
        let unstarted = find_match("led", "xled").unwrap();
        assert_eq!(ledger.indices, [0, 1, 2]);
        assert_eq!(gapped.indices, [0, 2, 4]);
        assert_eq!(unstarted.indices, [1, 2, 3]);
        assert!(ledger.score > gapped.score);
        assert!(ledger.score > unstarted.score);
    }

    #[test]
    fn boundary_start_outranks_an_inner_start() {
        let after_dot = find_match("rs", "main.rs").unwrap();
        let inner = find_match("rs", "cursor").unwrap();
        assert!(after_dot.score > inner.score);
    }

    #[test]
    fn matching_folds_case_both_ways() {
        assert_eq!(find_match("MR", "main.rs").unwrap().indices, [0, 5]);
        assert_eq!(find_match("mr", "MAIN.RS").unwrap().indices, [0, 5]);
    }

    #[test]
    fn hits_are_lit_in_runs() {
        let lit = Style::new().add_modifier(Modifier::BOLD);
        let line = light_matches("main.rs", &[0, 1, 5], lit);
        assert_eq!(
            line.spans,
            [
                Span::styled("ma", lit),
                Span::raw("in."),
                Span::styled("r", lit),
                Span::raw("s")
            ]
        );
    }
}
