use ely_gpui_component::buttons::Button;
use gpui::prelude::*;
use gpui::{div, px, App, AppContext, Context, IntoElement, Render, Window, WindowOptions};

struct Hello;

impl Render for Hello {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p(px(24.0))
            .child(Button::new("ok", "Hello"))
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(ely_gpui_component::Assets::before(gpui_kit::assets::Assets))
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            ely_gpui_component::init(cx).expect("ely");
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Hello))
                .expect("window");
        });
}
