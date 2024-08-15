use bincode::config;
use common::*;
use log::info;
use quadtree::Point;
use renet::transport::NetcodeServerTransport;
use renet::transport::ServerAuthentication;
use renet::transport::ServerConfig;
use renet::*;
use std::{
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant, SystemTime},
};

#[derive(Debug, Clone, Copy)]
struct P {
    x: f64,
    y: f64,
}

impl Point for P {
    fn point(&self) -> nalgebra::Point2<f64> {
        nalgebra::Point2::new(self.x, self.y)
    }
}

fn main() {
    env_logger::init();
    let public_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    log::info!("Listening on {public_addr}");
    let connection_config = ConnectionConfig::default();
    let mut server: RenetServer = RenetServer::new(connection_config);

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server_config = ServerConfig {
        current_time,
        max_clients: 1024,
        protocol_id: 0,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    let socket: UdpSocket = UdpSocket::bind(public_addr).unwrap();

    let mut transport = NetcodeServerTransport::new(server_config, socket).unwrap();

    let mut last_updated = Instant::now();
    let mut world_local = Game::default();
    let mut fps = FpsCounter::new();
    let mut last_sync = Instant::now();
    loop {
        fps.update();
        let now = Instant::now();
        let duration = now - last_updated;
        last_updated = now;
        let sync_duration = now - last_sync;

        server.update(duration);
        transport.update(duration, &mut server).unwrap();

        while let Some(event) = server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!("Client {} connected.", client_id)
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!("Client {} disconnected: {}", client_id, reason);
                }
            }
        }

        let mut action_count = 0;
        for client_id in server.clients_id() {
            while let Some(message) = server.receive_message(client_id, DefaultChannel::Unreliable)
            {
                if let Ok((action, _)) =
                    bincode::decode_from_slice::<GameMessage, _>(&message, config::standard())
                {
                    action_count += 1;
                    // info!("recieved: {action:?}");
                    world_local.action(action);
                }
            }
        }

        let c =
            byte_unit::Byte::from_u64(server.bytes_sent_per_sec(ClientId::from_raw(333)) as u64);
        info!("sent screen {c:#}");
        info!("processed {action_count} actions");
        let fps = fps.fps;
        info!("fps: {fps}");
        if sync_duration.as_millis() >= 50 {
            last_sync = now;
            server.send_message(
                ClientId::from_raw(333),
                DefaultChannel::Unreliable,
                bincode::encode_to_vec(
                    GameMessage::Synchronize {
                        world: world_local.clone(),
                    },
                    config::standard(),
                )
                .unwrap(),
            );
        }

        transport.send_packets(&mut server);
    }
}
