use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Query, Res};
use bevy_input_focus::InputFocus;
use plurimus_core::UiWidget;
use plurimus_core::ratatui_core::style::Style;
use plurimus_core::ratatui_core::text::{Line, Span};
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

/// Draws the field row: prompt, text, and a caret while the picker's list
/// holds focus, scrolled so the caret stays in view.
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
        let caret = is_focused.then_some(theme.caret);
        let (line, caret_end) = field_line(&look.prompt, picker.field(), caret);
        let scroll = caret_end.saturating_sub(usize::from(area.0.width));
        let paragraph = Paragraph::new(line)
            .style(next.style(&theme))
            .scroll((0, u16::try_from(scroll).unwrap_or(u16::MAX)));
        *widget = UiWidget::new(paragraph);
    }
}

/// The row's line and the column just past the caret, for scrolling.
fn field_line(
    prompt: &Line<'static>,
    field: &TextInput,
    caret: Option<Style>,
) -> (Line<'static>, usize) {
    let value = field.value();
    let at = value
        .char_indices()
        .nth(field.cursor())
        .map_or(value.len(), |(index, _)| index);
    let (before, rest) = value.split_at(at);
    let mut line = prompt.clone();
    line.push_span(Span::raw(before.to_owned()));
    let caret_end = line.width() + CARET_CELLS;
    match caret {
        Some(caret) => {
            let mut after = rest.chars();
            let under = after.next().unwrap_or(' ');
            line.push_span(Span::styled(under.to_string(), caret));
            line.push_span(Span::raw(after.as_str().to_owned()));
        }
        None => line.push_span(Span::raw(rest.to_owned())),
    }
    (line, caret_end)
}
