use std::sync::mpsc;

use cursive::{
    direction::Orientation,
    event,
    theme::Style,
    view::{self, Nameable, Resizable},
    views, View,
};

use crate::{events::KeyEvt, render_server::RendererEvent};

pub trait ViewBuilder {
    type ViewType: cursive::View;
    fn view_name() -> &'static str;
    fn build(evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType;

    fn new(evt_chan: mpsc::Sender<RendererEvent>) -> views::NamedView<Self::ViewType> {
        Self::build(evt_chan).with_name(Self::view_name())
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

    fn build(evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType {
        RootStackView::new(evt_chan)
    }
}

pub struct RootStackView;

impl ViewBuilder for RootStackView {
    type ViewType = views::StackView;

    fn view_name() -> &'static str {
        "root_stack"
    }

    fn build(evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType {
        views::StackView::new().fullscreen_layer(EditorView::new(evt_chan))
    }
}

pub struct EditorView {
    inner_view: views::LinearLayout,
    evt_chan: mpsc::Sender<RendererEvent>,
}

impl ViewBuilder for EditorView {
    type ViewType = Self;

    fn view_name() -> &'static str {
        "editor"
    }

    fn build(evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType {
        let inner_view = views::LinearLayout::new(Orientation::Vertical)
            .child(EditorTextView::new(evt_chan.clone()).full_screen())
            .child(CmdBarView::new(evt_chan.clone()))
            .child(LogView::new(evt_chan.clone()));
        EditorView {
            inner_view,
            evt_chan,
        }
    }
}

impl view::ViewWrapper for EditorView {
    cursive::wrap_impl!(self.inner_view: views::LinearLayout);

    fn wrap_on_event(&mut self, evt: event::Event) -> event::EventResult {
        KeyEvt::try_from_cursive_evt(evt).map(|evt| {
            self.evt_chan.send(RendererEvent::KeyEvent(evt)).unwrap();
        });
        event::EventResult::Consumed(None)
    }

    fn wrap_layout(&mut self, size: cursive::Vec2) {
        self.evt_chan
            .send(RendererEvent::Resized(size.x, size.y))
            .unwrap();
        self.inner_view.layout(size);
    }
}

pub struct EditorTextView;

impl ViewBuilder for EditorTextView {
    type ViewType = views::ScrollView<views::TextView>;

    fn view_name() -> &'static str {
        "editor_text"
    }

    fn build(_evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType {
        let mut v = views::TextView::new("loading...");
        v.set_style(Style::terminal_default());
        v.set_content_wrap(false);
        let mut v = views::ScrollView::new(v);
        v.set_scroll_x(true);
        v
    }
}

pub struct LogView;

impl ViewBuilder for LogView {
    type ViewType = views::TextView;

    fn view_name() -> &'static str {
        "log"
    }

    fn build(_evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType {
        views::TextView::new("logs")
    }
}

pub struct CmdBarView;

impl ViewBuilder for CmdBarView {
    type ViewType = views::TextView;

    fn view_name() -> &'static str {
        "cmd_bar"
    }

    fn build(_evt_chan: mpsc::Sender<RendererEvent>) -> Self::ViewType {
        views::TextView::new("cmd")
    }
}
