use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Query, Res};
use plurimus_core::UiWidget;
use plurimus_ui::bevy_input_focus::InputFocus;
use plurimus_ui::{StateQuery, Stylable, StylistCache, UiTheme, observed};
use plurimus_widgets::ratatui_widgets::paragraph::Paragraph;

use crate::parts::FilePickerParts;
use crate::picker::{FilePicker, FilePickerLook};

type Pickers<'w, 's> = Query<
    'w,
    's,
    (
        StateQuery<'static>,
        Ref<'static, FilePickerLook>,
        &'static FilePickerParts,
        &'static mut StylistCache,
        &'static mut UiWidget,
    ),
    Stylable<FilePicker>,
>;

/// Draws the prompt at the start of the field row; the field and the list
/// draw themselves over the rest. Focused while the field is.
pub(crate) fn style_file_pickers(
    theme: Res<UiTheme>,
    focus: Res<InputFocus>,
    mut pickers: Pickers,
) {
    for (state, look, parts, mut cache, mut widget) in &mut pickers {
        let is_focused = focus.get() == Some(parts.field);
        let next = observed(state, &focus, 0).with_focused(is_focused);
        if !cache.redraws(next, theme.is_changed() || look.is_changed()) {
            continue;
        }
        let paragraph = Paragraph::new(look.prompt.clone()).style(next.style(&theme));
        *widget = UiWidget::new(paragraph);
    }
}
