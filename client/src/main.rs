use bincode::config;
use common::*;
use log::info;
use rand::Rng;
use renet::{
    transport::{ClientAuthentication, NetcodeClientTransport},
    ConnectionConfig, DefaultChannel, RenetClient,
};
use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
    time::{Duration, Instant, SystemTime},
};

#[tokio::main]
async fn main() {
    env_logger::init();
    let n = 1023;
    let mut rng = rand::thread_rng();
    for i in 0..n {
        let client_id = rng.gen_range(0..1000000);
        info!("Spawning {i}");
        create_and_connect_client(client_id, 60000 + i);
    }
    sleep(Duration::from_secs(1000))
}

fn create_and_connect_client(id: u64, socket: usize) {
    tokio::task::spawn(async move {
        let server_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let connection_config = ConnectionConfig::default();
        let mut client = RenetClient::new(connection_config);

        let socket = UdpSocket::bind(format!("127.0.0.1:{socket}")).unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: id,
            user_data: None,
            protocol_id: 0,
        };

        let mut transport =
            NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
        let mut world_local = Game::default();

        let mut last_updated = Instant::now();

        let mut joined = false;
        loop {
            let now = Instant::now();
            let duration = now - last_updated;
            last_updated = now;

            client.update(duration);
            transport.update(duration, &mut client).unwrap();

            if client.is_connected() {
                if !joined {
                    joined = true;
                    client.send_message(
                        DefaultChannel::Unreliable,
                        bincode::encode_to_vec(
                            GameMessage::PlayerAdded {
                                player_id: id.to_string(),
                            },
                            config::standard(),
                        )
                        .unwrap(),
                    );
                }
                while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
                    if let Ok((action, _)) =
                        bincode::decode_from_slice::<GameMessage, _>(&message, config::standard())
                    {
                        world_local.action(action);
                    }
                }
                if let Some(action) = random_player_action(id.to_string()) {
                    world_local.action(action.clone());
                    client.send_message(
                        DefaultChannel::Unreliable,
                        bincode::encode_to_vec(action, config::standard()).unwrap(),
                    );
                }
            } else {
                // error!("Not connected");
            }

            transport.send_packets(&mut client).unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
}

fn random_player_action(id: String) -> Option<GameMessage> {
    let mut rng = rand::thread_rng();
    let action = rng.gen_range(0..3);

    match action {
        0 => GameMessage::Rotate {
            player_id: id,
            angle: rng.gen_range(-0.3..0.3),
        }
        .into(),
        1 => GameMessage::MoveForward {
            player_id: id,
            distance: rng.gen_range(0.0..30.0),
        }
        .into(),
        2 => GameMessage::MoveBackward {
            player_id: id,
            distance: rng.gen_range(0.0..30.0),
        }
        .into(),
        _ => None,
    }
}
