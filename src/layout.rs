use plurimus_core::ratatui_core::layout::Rect;

const FIELD_ROWS: u16 = 1;

/// The field across the first row after `prompt` cells, and whatever is
/// left for the list.
pub(crate) fn split_area(area: Rect, prompt: u16) -> (Rect, Rect) {
    let prompt = prompt.min(area.width);
    let field = Rect {
        x: area.x.saturating_add(prompt),
        width: area.width - prompt,
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
    fn field_takes_the_first_row_after_the_prompt_and_the_list_the_rest() {
        let (field, list) = split_area(Rect::new(2, 3, 10, 5), 2);
        assert_eq!(field, Rect::new(4, 3, 8, 1));
        assert_eq!(list, Rect::new(2, 4, 10, 4));
    }

    #[test]
    fn short_areas_starve_the_list_before_the_field() {
        assert_eq!(
            split_area(Rect::new(0, 0, 4, 1), 2),
            (Rect::new(2, 0, 2, 1), Rect::new(0, 1, 4, 0))
        );
        assert_eq!(
            split_area(Rect::new(0, 0, 4, 0), 2),
            (Rect::new(2, 0, 2, 0), Rect::new(0, 0, 4, 0))
        );
    }

    #[test]
    fn a_prompt_wider_than_the_area_leaves_the_field_no_cells() {
        let (field, _) = split_area(Rect::new(0, 0, 4, 2), 9);
        assert_eq!(field, Rect::new(4, 0, 0, 1));
    }
}
