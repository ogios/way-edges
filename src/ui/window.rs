use crate::config::{self, Config};

use gtk::{
    gdk::Monitor, prelude::*, Application, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::LayerShell;

use super::widgets;

pub fn new_window(
    app: &Application,
    mut config: Config,
    monitor: &Monitor,
) -> Result<gtk::ApplicationWindow, String> {
    let window = gtk::ApplicationWindow::new(app);

    // init layer
    window.init_layer_shell();
    window.set_monitor(monitor);
    window.set_layer(config.layer);

    // edge and position
    window.set_anchor(config.edge, true);
    if let Some(pos) = config.position {
        window.set_anchor(pos, true);
    }

    // set something after show
    window.connect_show(|w: &gtk::ApplicationWindow| {
        // transparency background !! may not work for some gtk4 theme, and idk what to do with it !!
        let provider = CssProvider::new();
        provider
            // .load_from_string("window.background { background: unset; border: 1px solid white; }");
            .load_from_string("window.background { background: unset; }");
        gtk::style_context_add_provider_for_display(
            &WidgetExt::display(w),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    // TEST:
    widgets::wrapbox::init_widget(&window);
    return Ok(window);

    // margin
    std::mem::take(&mut config.margins)
        .into_iter()
        .try_for_each(|(e, m)| -> Result<(), String> {
            window.set_margin(e, m.get_num_into()? as i32);
            Ok(())
        })
        .and_then(|_| match config.widget.take().ok_or("Widget is None")? {
            config::Widget::Btn(c) => widgets::button::init_widget(&window, config, *c).map(|_| ()),
            config::Widget::Slider(c) => {
                widgets::slide::init_widget(&window, config, *c).map(|_| ())
            }
            config::Widget::PulseAudio(c) => {
                widgets::pulseaudio::init_widget(&window, config, *c).map(|_| ())
            }
            config::Widget::Backlight(c) => {
                widgets::backlight::init_widget(&window, config, *c).map(|_| ())
            }
            _ => unreachable!(),
        })?;

    Ok(window)
}
