#![cfg(feature = "hyprland")]

mod monitor;
use monitor::*;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::activate::{
    calculate_relative, create_buttons, find_monitor_with_vec, get_monitors, notify_app_error,
    ButtonItem,
};
use crate::config::GroupConfig;
use gio::glib::idle_add_local_once;
use gtk::gdk::Monitor;
use gtk::glib;
use gtk::prelude::{GtkWindowExt, MonitorExt, WidgetExt};
use gtk::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use scopeguard::defer;

/// namespace for detect size of available working area
/// TL: Top Left
const NAMESPACE_TL: &str = "way-edges-detect-tl";

/// namespace for detect size of available working area
/// BR: Bottom Right
const NAMESPACE_BR: &str = "way-edges-detect-br";

/// reach max counter
type Counter = Rc<Cell<usize>>;
fn add_or_else(c: &Counter, max: usize) -> bool {
    if c.get() == max - 1 {
        true
    } else {
        c.set(c.get() + 1);
        false
    }
}

/// create window for detection on specific monitor
/// 2 window for positioning: one on top-left corner; one on bottom-right corner
fn window_for_detect(
    app: &Application,
    monitor: &Monitor,
    // layer: Layer,
) -> [ApplicationWindow; 2] {
    // left top
    let win_tl = gtk::ApplicationWindow::new(app);
    win_tl.init_layer_shell();
    // win_tl.set_layer(layer);
    win_tl.set_layer(Layer::Top);
    win_tl.set_anchor(Edge::Top, true);
    win_tl.set_anchor(Edge::Left, true);
    win_tl.set_width_request(1);
    win_tl.set_height_request(1);
    win_tl.set_namespace(NAMESPACE_TL);
    win_tl.set_monitor(monitor);

    // bottom left
    let win_br = gtk::ApplicationWindow::new(app);
    win_br.init_layer_shell();
    // win_br.set_layer(layer);
    win_br.set_layer(Layer::Top);
    win_br.set_anchor(Edge::Bottom, true);
    win_br.set_anchor(Edge::Right, true);
    win_br.set_width_request(1);
    win_br.set_height_request(1);
    win_br.set_namespace(NAMESPACE_BR);
    win_tl.set_monitor(monitor);

    [win_tl, win_br]
}

/// connect realize signal.
/// get layer info from hyprland after rendered.
/// calculate the available area size for each monitor.
/// calculate relative size.
/// render widgets.
fn connect(
    ws: Vec<ApplicationWindow>,
    app: &gtk::Application,
    cfgs: GroupConfig,
    monitor_connectors: HashMap<String, Monitor>,
) {
    let windows_count = ws.len();

    // as for why so many rc cells, `connect_realize` is not `FnOnce` but `Fn`
    // idk what i can do better for this
    let counter = Rc::new(Cell::new(0));
    let cfgs = Rc::new(Cell::new(cfgs));
    let ws = Rc::new(Cell::new(ws));
    let monitor_connectors = Rc::new(Cell::new(monitor_connectors));

    // used to setup realize event for each window
    let connect = gtk::glib::clone!(@strong counter, @strong ws, @weak app, @strong cfgs, @strong monitor_connectors => move |w: &ApplicationWindow| {
        w.connect_realize(gtk::glib::clone!(@strong counter, @strong ws, @weak app, @strong cfgs, @strong monitor_connectors => move |_| {
            // calculate after all window rendered(windows are not actually rendered when realize signaled)
            idle_add_local_once(
                gtk::glib::clone!(@weak counter, @weak ws, @weak app, @weak cfgs, @weak monitor_connectors  => move || {
                    // we need to get all layer info of windows
                    // we are going to do it after the last window rendered
                    // and we use counter to do it
                    if add_or_else(&counter, windows_count) {
                        // close window after calculation
                        let ws = ws.take();
                        defer!(
                            ws.into_iter().for_each(|w| {
                                w.close();
                            });
                        );
                        // get available area size for all needed monitor
                        let res = get_monitor_map(monitor_connectors.take()).and_then(|mm| {
                            log::debug!("Calculated layer map sizes: {mm:?}");
                            let cfgs = cfgs.take();
                            let monitors = mm.keys().collect::<Vec<&Monitor>>();
                            // create button items
                            let btis = cfgs.into_iter().map(|mut cfg| {
                                // get available area size for each monitor
                                let monitor = find_monitor_with_vec(&monitors, cfg.monitor.clone())?;
                                let size = *mm.get(&monitor).ok_or(format!("Did not find Calculated monitor size for {:?}", cfg.monitor))?;
                                calculate_relative(&mut cfg, size)?;
                                Ok(ButtonItem { cfg, monitor })
                            }).collect::<Result<Vec<ButtonItem>, String>>()?;
                            create_buttons(&app, btis);
                            Ok(())
                        });
                        if let Err(e) = res {
                            notify_app_error(format!("Failed to initialize app: get_monitor_map(): {e}"), &app);
                            return;
                        }
                    }
                })
            );
        }));
    });
    unsafe {
        ws.as_ptr().as_ref().unwrap().iter().for_each(|w| {
            connect(w);
        });
    }
}

pub struct Hyprland;
impl super::WindowInitializer for Hyprland {
    fn init_window(app: &Application, cfgs: GroupConfig) {
        let res = get_monitors().and_then(|monitors| {
            get_need_monitors(&cfgs, &monitors).and_then(|ml| {
                // initialize corner windows for eache monitor
                let ws = ml
                    .iter()
                    .flat_map(|m| window_for_detect(app, m))
                    .collect::<Vec<ApplicationWindow>>();

                // monitor name -> monitor
                let ml = ml
                    .into_iter()
                    .map(|m| {
                        let name = m
                            .connector()
                            .map(|v| v.to_string())
                            .ok_or(format!("Failed to get monitor name: {m:?}"))?;
                        Ok((name, m))
                    })
                    .collect::<Result<HashMap<String, Monitor>, String>>()?;

                // setup connect signal
                connect(ws.clone(), app, cfgs, ml);

                // show each window
                ws.iter().for_each(|w| {
                    w.present();
                });
                Ok(())
            })
        });
        if let Err(e) = res {
            notify_app_error(e, app)
        }
    }
}
