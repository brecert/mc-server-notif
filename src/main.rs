#![feature(drain_filter)]
#![feature(string_remove_matches)]
#![feature(iter_intersperse)]
#![feature(scoped_threads)]
use std::{collections::HashSet, net::TcpStream, thread, time::Duration};

use craftping::{sync::ping, Response};
use notify_rust::Notification;

#[cfg(target_os = "linux")]
use tao::platform::linux::SystemTrayBuilderExtLinux;
use tao::{
    event::{Event, TrayEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu as Menu, MenuId, MenuItemAttributes, MenuType},
    system_tray::SystemTrayBuilder,
};

fn ping_server(hostname: &str, port: u16) -> Response {
    let mut stream = TcpStream::connect((hostname.as_ref(), port)).unwrap();
    let pong = ping(&mut stream, &hostname, port).expect("Cannot ping server");
    pong
}

fn create_menu(pong: Response) -> (tao::menu::ContextMenu, MenuId) {
    let mut tray_menu = Menu::new();

    let server_count = tray_menu.add_item(MenuItemAttributes::new(&format!(
        "{} / {}",
        pong.online_players, pong.max_players
    )));

    if let Some(sample) = pong.sample {
        for player in sample {
            tray_menu.add_item(MenuItemAttributes::new(&player.name).with_enabled(false));
        }
    }

    tray_menu.add_native_item(tao::menu::MenuItem::Quit);

    return (tray_menu, server_count.id());
}

#[fncmd::fncmd]
fn main(
    /// The hostname for a server `example.com`
    #[opt(long, short)]
    hostname: String,

    /// The port for a server `25565`
    #[opt(long, short, default_value_t = 25565)]
    port: u16,

    /// How often to check and refresh the server for new players in seconds.
    #[opt(long, short, default_value_t = 30)]
    refresh: u64,
) {
    let hostname2 = hostname.clone();

    env_logger::init();
    let event_loop = EventLoop::new();

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png");

    let icon = load_icon(std::path::Path::new(path));

    let (tray_menu, mut server_count_id) = create_menu(ping_server(&hostname, port));

    #[cfg(target_os = "linux")]
    let mut system_tray = SystemTrayBuilder::new(icon, Some(tray_menu))
        .with_temp_icon_dir(std::path::Path::new("/tmp/minecraft-notifications"))
        .build(&event_loop)
        .unwrap();

    #[cfg(not(target_os = "linux"))]
    let mut system_tray = SystemTrayBuilder::new(icon, Some(tray_menu))
        .build(&event_loop)
        .unwrap();

    thread::spawn(move || {
        let mut player_count = 0;
        let mut current: HashSet<String> = HashSet::new();

        loop {
            thread::sleep(Duration::from_secs(refresh));

            let pong = ping_server(&hostname2, port);

            if let Some(mut sample) = pong.sample {
                // todo: remove clone
                let ids: HashSet<String> = sample.iter().map(|p| p.id.clone()).collect();

                let diff = &ids - &current;

                if diff.len() > 0 {
                    let names: String = sample
                        .drain_filter(|p| diff.contains(&p.id))
                        .map(|p| p.name)
                        .intersperse_with(|| ", ".into())
                        .collect();

                    Notification::new()
                        .summary(&format!("{names} joined {hostname2}:{port}"))
                        .appname("minecraft-notif")
                        .auto_icon()
                        .show()
                        .expect("Unable to create notification");
                } else if player_count > pong.online_players {
                    Notification::new()
                        .summary(&format!("An unknown player joined {hostname2}:{port}"))
                        .appname("minecraft-notif")
                        .auto_icon()
                        .show()
                        .expect("Unable to create notification");
                }

                current = ids;
                player_count = pong.online_players;
            } else {
                current.clear();
                player_count = 0;
            }
        }
    });

    event_loop.run(move |event, _event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::TrayEvent {
                event: TrayEvent::RightClick,
                ..
            } => {
                let parts = create_menu(ping_server(&hostname, port));
                system_tray.set_menu(&parts.0);
                server_count_id = parts.1;
            }
            Event::MenuEvent {
                menu_id,
                // specify only context menu's
                origin: MenuType::ContextMenu,
                ..
            } => {
                if menu_id == server_count_id {
                    let parts = create_menu(ping_server(&hostname, port));
                    system_tray.set_menu(&parts.0);
                    server_count_id = parts.1;
                }
            }
            _ => (),
        }
    });
}

fn load_icon(path: &std::path::Path) -> tao::system_tray::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tao::system_tray::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon")
}
