use cursive::{
    direction::Orientation,
    event::{self, Callback},
    theme::Style,
    view::{self, Nameable, Resizable},
    views,
};

use crate::{events::KeyEvt, render_server::RendererEvent};

use super::CursiveFrontendUserData;

pub trait ViewBuilder {
    type ViewType: cursive::View;
    fn view_name() -> &'static str;
    fn build() -> Self::ViewType;

    fn new() -> views::NamedView<Self::ViewType> {
        Self::build().with_name(Self::view_name())
    }

    fn get(ctx: &mut cursive::Cursive) -> cursive::views::ViewRef<Self::ViewType> {
        ctx.find_name::<Self::ViewType>(Self::view_name()).unwrap()
    }
}

pub struct RootView;

impl ViewBuilder for RootView {
    type ViewType = views::NamedView<<RootStackView as ViewBuilder>::ViewType>;

    fn view_name() -> &'static str {
        "root"
    }

    fn build() -> Self::ViewType {
        RootStackView::new()
    }
}

pub struct RootStackView;

impl ViewBuilder for RootStackView {
    type ViewType = views::StackView;

    fn view_name() -> &'static str {
        "root_stack"
    }

    fn build() -> Self::ViewType {
        views::StackView::new().fullscreen_layer(EditorView::new())
    }
}

pub struct EditorView {
    inner_view: views::LinearLayout,
}

impl ViewBuilder for EditorView {
    type ViewType = Self;

    fn view_name() -> &'static str {
        "editor"
    }

    fn build() -> Self::ViewType {
        let inner_view = views::LinearLayout::new(Orientation::Vertical)
            .child(EditorTextView::new().full_screen())
            .child(CmdBarView::new())
            .child(LogView::new());
        EditorView { inner_view }
    }
}

impl view::ViewWrapper for EditorView {
    cursive::wrap_impl!(self.inner_view: views::LinearLayout);

    fn wrap_on_event(&mut self, evt: event::Event) -> event::EventResult {
        event::EventResult::Consumed(Some(Callback::from_fn_once(|ctx| {
            ctx.with_user_data(|user_data: &mut CursiveFrontendUserData| {
                KeyEvt::try_from_cursive_evt(evt).map(|evt| {
                    user_data
                        .evt_chan
                        .send(RendererEvent::KeyEvent(evt))
                        .unwrap();
                })
            });
        })))
    }
}

pub struct EditorTextView;

impl ViewBuilder for EditorTextView {
    type ViewType = views::TextView;

    fn view_name() -> &'static str {
        "editor_text"
    }

    fn build() -> Self::ViewType {
        let mut v = views::TextView::new("[error]");
        v.set_style(Style::terminal_default());
        v.set_content_wrap(false);
        v
    }
}

pub struct LogView;

impl ViewBuilder for LogView {
    type ViewType = views::TextView;

    fn view_name() -> &'static str {
        "log"
    }

    fn build() -> Self::ViewType {
        views::TextView::new("logs")
    }
}

pub struct CmdBarView;

impl ViewBuilder for CmdBarView {
    type ViewType = views::TextView;

    fn view_name() -> &'static str {
        "cmd_bar"
    }

    fn build() -> Self::ViewType {
        views::TextView::new("cmd")
    }
}
