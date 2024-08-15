use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use bincode::{Decode, Encode};

#[derive(Encode, Decode, PartialEq, Debug, Default, Clone)]
pub struct Game {
    pub players: HashMap<String, Player>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum GameMessage {
    Synchronize { world: Game },
    Rotate { player_id: String, angle: f32 },
    MoveForward { player_id: String, distance: f32 },
    MoveBackward { player_id: String, distance: f32 },
    RandomMove { player_id: String },
    PlayerAdded { player_id: String },
    PlayerRemoved { player_id: String },
}

impl Game {
    pub fn action(&mut self, message: GameMessage) {
        match message {
            GameMessage::Rotate { player_id, angle } => {
                if let Some(player) = self.players.get_mut(&player_id) {
                    player.rotation += angle;
                }
            }
            GameMessage::MoveForward {
                player_id,
                distance,
            } => {
                if let Some(player) = self.players.get_mut(&player_id) {
                    player.position.0 =
                        (player.position.0 + player.rotation.cos() * distance).clamp(0., 500.);
                    player.position.1 =
                        (player.position.1 + player.rotation.sin() * distance).clamp(0., 500.);
                }
            }
            GameMessage::MoveBackward {
                player_id,
                distance,
            } => {
                if let Some(player) = self.players.get_mut(&player_id) {
                    player.position.0 =
                        (player.position.0 - player.rotation.cos() * distance).clamp(0., 500.);
                    player.position.1 =
                        (player.position.1 - player.rotation.sin() * distance).clamp(0., 500.);
                }
            }
            GameMessage::RandomMove { player_id } => {
                if let Some(player) = self.players.get(&player_id) {}
            }
            GameMessage::Synchronize { world } => *self = world,
            GameMessage::PlayerAdded { player_id } => {
                _ = self.players.insert(
                    player_id,
                    Player {
                        position: (0., 0.),
                        rotation: 0.,
                    },
                )
            }
            GameMessage::PlayerRemoved { player_id } => _ = self.players.remove(&player_id),
        }
    }
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Player {
    pub position: (f32, f32),
    pub rotation: f32,
}

pub struct FpsCounter {
    pub frame_count: u32,
    pub last_update: Instant,
    pub fps: f64,
}

impl FpsCounter {
    pub fn new() -> Self {
        FpsCounter {
            frame_count: 0,
            last_update: Instant::now(),
            fps: 0.0,
        }
    }

    pub fn update(&mut self) -> f64 {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);

        if elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.last_update = now;
        }

        self.fps
    }
}
