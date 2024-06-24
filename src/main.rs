mod activate;
mod args;
mod config;
mod data;
mod ui;

use clap::Parser;
use gio::{prelude::*, ApplicationFlags};
use gtk::Application;

fn main() {
    // for cmd line help msg.
    // or else it will show help from `gtk` not `clap`
    args::Cli::parse();

    // set renderer explicitly to cairo instead of ngl
    std::env::set_var("GSK_RENDERER", "cairo");

    // that flag is for command line arguments
    let application = gtk::Application::new(None::<String>, ApplicationFlags::HANDLES_OPEN);

    // when args passed, `open` will be signaled instead of `activate`
    application.connect_open(|app, _, _| {
        init_app(app);
    });
    application.connect_activate(|app| {
        init_app(app);
    });

    application.run();
}

fn init_app(app: &Application) {
    let args = args::Cli::parse();
    println!("{:#?}", args);
    let group_map = config::get_config().unwrap();
    let cfgs = config::match_group_config(group_map, args.group);
    cfgs.iter().for_each(|c| {
        println!("{}", c.debug());
    });

    activate::activate(app, cfgs);
}
