use plurimus_core::ratatui_core::layout::Rect;

const FIELD_ROWS: u16 = 1;

/// The field row across the top, and whatever is left for the list.
pub(crate) fn split_area(area: Rect) -> (Rect, Rect) {
    let field = Rect {
        height: area.height.min(FIELD_ROWS),
        ..area
    };
    let list = Rect {
        y: area.y.saturating_add(field.height),
        height: area.height.saturating_sub(field.height),
        ..area
    };
    (field, list)
}

#[cfg(test)]
mod tests {
    use super::split_area;
    use plurimus_core::ratatui_core::layout::Rect;

    #[test]
    fn field_takes_the_first_row_and_the_list_the_rest() {
        let (field, list) = split_area(Rect::new(2, 3, 10, 5));
        assert_eq!(field, Rect::new(2, 3, 10, 1));
        assert_eq!(list, Rect::new(2, 4, 10, 4));
    }

    #[test]
    fn short_areas_starve_the_list_before_the_field() {
        assert_eq!(
            split_area(Rect::new(0, 0, 4, 1)),
            (Rect::new(0, 0, 4, 1), Rect::new(0, 1, 4, 0))
        );
        assert_eq!(
            split_area(Rect::new(0, 0, 4, 0)),
            (Rect::new(0, 0, 4, 0), Rect::new(0, 0, 4, 0))
        );
    }
}
