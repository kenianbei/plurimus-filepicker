use std::ops::Range;

use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Query, Res};
use plurimus_core::UiWidget;
use plurimus_core::ratatui_core::style::Style;
use plurimus_core::ratatui_core::text::{Line, Span};
use plurimus_ui::bevy_input_focus::InputFocus;
use plurimus_ui::{
    ComputedWidgetArea, StateQuery, Stylable, StylistCache, UiTheme, hashed_bits, observed,
};
use plurimus_widgets::TextInput;
use plurimus_widgets::ratatui_widgets::paragraph::Paragraph;

use crate::parts::PickerList;
use crate::picker::{FilePicker, FilePickerLook};

const CARET_CELLS: usize = 1;

type Pickers<'w, 's> = Query<
    'w,
    's,
    (
        StateQuery<'static>,
        &'static FilePicker,
        Ref<'static, FilePickerLook>,
        &'static ComputedWidgetArea,
        &'static PickerList,
        &'static mut StylistCache,
        &'static mut UiWidget,
    ),
    Stylable<FilePicker>,
>;

/// Draws the field row: prompt, text, and the selection or, with none and
/// the picker's list focused, a caret; scrolled so the cursor stays in view.
pub(crate) fn style_file_pickers(
    theme: Res<UiTheme>,
    focus: Res<InputFocus>,
    mut pickers: Pickers,
) {
    for (state, picker, look, area, list, mut cache, mut widget) in &mut pickers {
        let is_focused = focus.get() == Some(list.0);
        let next = observed(state, &focus, hashed_bits((picker.field(), area.0.width)))
            .with_focused(is_focused);
        if !cache.redraws(next, theme.is_changed() || look.is_changed()) {
            continue;
        }
        let field = picker.field();
        let caret = field.cursor()..field.cursor() + 1;
        let mark = field
            .selection()
            .map(|selected| (selected, theme.selection))
            .or_else(|| is_focused.then_some((caret, theme.caret)));
        let (line, caret_end) = field_line(&look.prompt, field, mark);
        let scroll = caret_end.saturating_sub(usize::from(area.0.width));
        let paragraph = Paragraph::new(line)
            .style(next.style(&theme))
            .scroll((0, u16::try_from(scroll).unwrap_or(u16::MAX)));
        *widget = UiWidget::new(paragraph);
    }
}

/// The row's line, the chars of `mark` in its style, and the column just
/// past the cursor, for scrolling. A mark past the text's end is one blank.
fn field_line(
    prompt: &Line<'static>,
    field: &TextInput,
    mark: Option<(Range<usize>, Style)>,
) -> (Line<'static>, usize) {
    let value = field.value();
    let byte_at = |index: usize| {
        value
            .char_indices()
            .nth(index)
            .map_or(value.len(), |(byte, _)| byte)
    };
    let mut line = prompt.clone();
    let before_cursor = Span::raw(&value[..byte_at(field.cursor())]).width();
    let caret_end = line.width() + before_cursor + CARET_CELLS;
    let Some((chars, style)) = mark else {
        line.push_span(Span::raw(value.to_owned()));
        return (line, caret_end);
    };
    let (start, end) = (byte_at(chars.start), byte_at(chars.end));
    let marked = if start == end {
        " "
    } else {
        &value[start..end]
    };
    line.push_span(Span::raw(value[..start].to_owned()));
    line.push_span(Span::styled(marked.to_owned(), style));
    line.push_span(Span::raw(value[end..].to_owned()));
    (line, caret_end)
}
