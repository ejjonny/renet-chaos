use common::*;
use eframe::Frame;

struct GameApp {
    animation: Arc<Mutex<Animated<f32, Instant>>>,
    last_game: Arc<Mutex<Game>>,
    game: Arc<Mutex<Game>>,
    render_fps: FpsCounter,
    updates_ps: Arc<Mutex<FpsCounter>>,
}

impl eframe::App for GameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.render_fps.update();
        egui::CentralPanel::default().show(ctx, |ui| {
            let fps = self.render_fps.fps;
            ui.heading(format!("fps: {fps}"));
            if let Ok(g) = self.updates_ps.try_lock() {
                let fps = g.fps;
                ui.heading(format!("updates per second: {fps}"));
            }

            let painter = ui.painter();

            if let Ok(g) = self.game.try_lock() {
                if let Ok(lg) = self.last_game.try_lock() {
                    if let Ok(anim) = self.animation.try_lock() {
                        let i = anim
                            .animate(
                                |f| {
                                    if f == 0. {
                                        IntpGame(lg.clone())
                                    } else {
                                        IntpGame(g.clone())
                                    }
                                },
                                Instant::now(),
                            )
                            .0;
                        for (_id, player) in i.players {
                            let player_pos =
                                egui::pos2(player.position.0 + 10., player.position.1 + 80.);
                            painter.circle_filled(player_pos, 3.0, egui::Color32::WHITE);

                            let direction =
                                egui::vec2(player.rotation.cos(), player.rotation.sin());
                            let line_end = player_pos + direction * 3.0;
                            painter.line_segment(
                                [player_pos, line_end],
                                egui::Stroke::new(2.0, egui::Color32::RED),
                            );

                            // painter.text(
                            //     player_pos + egui::vec2(0.0, -20.0),
                            //     egui::Align2::CENTER_CENTER,
                            //     id,
                            //     egui::FontId::default(),
                            //     egui::Color32::from_rgba_premultiplied(255, 255, 255, 10),
                            // );
                        }
                    }
                }
            }
        });
        ctx.request_repaint();
    }
}

struct IntpGame(Game);
impl Interpolable for IntpGame {
    fn interpolated(&self, other: Self, ratio: f32) -> Self {
        IntpGame(Game {
            players: self
                .0
                .players
                .iter()
                .map(|(key, player)| {
                    let other_player = other.0.players.get(key);

                    (
                        key.clone(),
                        match other_player {
                            Some(other_player) => Player {
                                position: (
                                    player.position.0
                                        + (other_player.position.0 - player.position.0) * ratio,
                                    player.position.1
                                        + (other_player.position.1 - player.position.1) * ratio,
                                ),
                                rotation: player.rotation
                                    + (other_player.rotation - player.rotation) * ratio,
                            },
                            None => player.clone(),
                        },
                    )
                })
                .collect(),
        })
    }
}

#[tokio::main]
async fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    let state = GameApp {
        animation: Arc::new(Mutex::new(Animated::new(0.))),
        last_game: Arc::new(Mutex::new(Game {
            players: HashMap::new(),
        })),
        game: Arc::new(Mutex::new(Game {
            players: HashMap::new(),
        })),
        render_fps: FpsCounter::new(),
        updates_ps: Arc::new(Mutex::new(FpsCounter::new())),
    };
    let last_state = state.last_game.clone();
    let game_updating = state.game.clone();
    let update = state.updates_ps.clone();
    let a = state.animation.clone();
    tokio::task::spawn(async { subscribe(last_state, game_updating, update, a).await });
    eframe::run_native(
        "Game Visualization",
        native_options,
        Box::new(|_cc| Ok(Box::new(state))),
    )
}

use bincode::config;
use lilt::{Animated, Interpolable};
use log::{error, info};
use renet::{
    transport::{ClientAuthentication, NetcodeClientTransport},
    ConnectionConfig, DefaultChannel, RenetClient,
};
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::Arc,
    thread,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::Mutex;

async fn subscribe(
    last_state: Arc<Mutex<Game>>,
    state: Arc<Mutex<Game>>,
    update: Arc<Mutex<FpsCounter>>,
    a: Arc<Mutex<Animated<f32, Instant>>>,
) {
    env_logger::init();
    let server_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let connection_config = ConnectionConfig::default();
    let mut client = RenetClient::new(connection_config);

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id: 333,
        user_data: None,
        protocol_id: 0,
    };

    let mut transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
    let world_local = state;

    let mut last_updated = Instant::now();

    loop {
        update.lock().await.update();
        let now = Instant::now();
        let duration = now - last_updated;
        last_updated = now;

        client.update(duration);
        transport.update(duration, &mut client).unwrap();

        if client.is_connected() {
            while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
                if let Ok((action, _)) =
                    bincode::decode_from_slice::<GameMessage, _>(&message, config::standard())
                {
                    let mut ls = last_state.lock().await;
                    let mut world_local = world_local.lock().await;
                    *ls = world_local.clone();
                    world_local.action(action);
                    let mut animation = a.lock().await;
                    *animation = Animated::new(0.).duration(10.).easing(lilt::Easing::Linear);
                    animation.transition(1., Instant::now());
                }
            }
        } else {
            error!("Not connected");
        }

        thread::sleep(Duration::from_millis(50));
    }
}
