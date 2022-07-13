#![feature(string_remove_matches)]

use std::{collections::HashSet, net::TcpStream, thread, time::Duration};

use craftping::sync::ping;
use notify_rust::Notification;

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

    /// Players to notify for, UUID or Username
    #[opt(long, short = 'o')]
    players: Vec<String>,
) {
    let hostname2 = hostname.clone();

    // remove uuid `-`
    let players: HashSet<String> = players
        .into_iter()
        .map(|mut p| {
            p.remove_matches("-");
            p
        })
        .collect();

    #[cfg(target_os = "linux")]
    use tao::platform::linux::SystemTrayBuilderExtLinux;
    use tao::{
        event::Event,
        event_loop::{ControlFlow, EventLoop},
        menu::{ContextMenu as Menu, MenuItemAttributes, MenuType},
        system_tray::SystemTrayBuilder,
    };

    env_logger::init();
    let event_loop = EventLoop::new();

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png");

    let icon = load_icon(std::path::Path::new(path));

    let create_menu = move || {
        let mut tray_menu = Menu::new();

        let mut stream = TcpStream::connect((hostname.as_ref(), port)).unwrap();
        let pong = ping(&mut stream, &hostname, port).expect("Cannot ping server");

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
    };

    let (tray_menu, mut server_count_id) = create_menu();

    #[cfg(target_os = "linux")]
    let system_tray = SystemTrayBuilder::new(icon, Some(tray_menu))
        .with_temp_icon_dir(std::path::Path::new("/tmp/mctray"))
        .build(&event_loop)
        .unwrap();

    #[cfg(not(target_os = "linux"))]
    let mut system_tray = SystemTrayBuilder::new(icon, Some(tray_menu))
        .build(&event_loop)
        .unwrap();

    thread::spawn(move || {
        let mut current: HashSet<String> = HashSet::with_capacity(players.len());
        loop {
            thread::sleep(Duration::from_secs(refresh));

            let mut stream = TcpStream::connect((hostname2.as_ref(), port)).unwrap();
            let pong = ping(&mut stream, &hostname2, port).expect("Cannot ping server");

            if let Some(sample) = pong.sample {
                let players_that_matter = sample
                    .into_iter()
                    .filter(|p| players.contains(&p.id) || players.contains(&p.name));

                let ids: HashSet<String> = players_that_matter
                    .map(|p| {
                        let mut id = p.id;
                        id.remove_matches("-");
                        id
                    })
                    .collect();

                let diff = &ids - &current;

                if diff.contains("7a8084cd1f444a159bb1eef8d5b535a1") {
                    Notification::new()
                        .summary(&format!("brecert joined {hostname2}:{port}"))
                        .show()
                        .expect("Unable to create notification");
                }

                dbg!(&ids);

                current = ids;
            } else {
                current.clear();
            }
        }
    });

    event_loop.run(move |event, _event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::MenuEvent {
                menu_id,
                // specify only context menu's
                origin: MenuType::ContextMenu,
                ..
            } => {
                if menu_id == server_count_id {
                    let parts = create_menu();
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
