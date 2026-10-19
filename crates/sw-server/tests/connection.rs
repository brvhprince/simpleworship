//! A real WebSocket client against the server: it is welcomed with the
//! engine's state, its role is enforced, events reach it, and removing the
//! device disconnects it.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use sw_core::{Command, Engine, EngineSnapshot, Id};
use sw_server::{
    Device, Host, MediaFolder, PairRequest, RemoteServer, Role, ServerConfig, pairing,
};
use tokio_tungstenite::tungstenite::Message;

/// An engine with no app around it, recording what it was asked to do.
#[derive(Default)]
struct FakeHost {
    engine: Mutex<Engine>,
    commands: Mutex<Vec<String>>,
}

impl Host for FakeHost {
    fn snapshot(&self) -> EngineSnapshot {
        self.engine.lock().unwrap().snapshot()
    }
    fn dispatch(&self, command: Command) -> Result<(), String> {
        self.commands.lock().unwrap().push(format!("{command:?}"));
        self.engine
            .lock()
            .unwrap()
            .dispatch(command)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    fn call(&self, method: &str, _params: Value) -> Result<Value, String> {
        Ok(json!({ "method": method }))
    }
    fn media_path(&self, _folder: MediaFolder, _file: &str) -> Option<PathBuf> {
        None
    }
    fn pairing_requested(&self, _request: PairRequest) {}
    fn devices_changed(&self, _devices: &[Device]) {}
    fn connections_changed(&self, _connected: Vec<Id>) {}
}

async fn next_json(
    socket: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Value {
    loop {
        match socket.next().await.expect("a message").expect("no error") {
            Message::Text(text) => return serde_json::from_str(&text).unwrap(),
            Message::Close(_) => return json!({ "type": "closed" }),
            _ => {}
        }
    }
}

#[tokio::test]
async fn a_presenter_phone_connects_and_is_kept_to_its_role() {
    let host = Arc::new(FakeHost::default());
    let (device, token) = pairing::new_device("Pastor's phone", Role::Presenter);
    let device_id = device.id;
    let server = RemoteServer::start(
        ServerConfig {
            port: 0,
            static_dir: PathBuf::from("does-not-exist"),
        },
        host.clone(),
        vec![device],
    )
    .await
    .unwrap();

    let url = format!("ws://127.0.0.1:{}/ws?token={token}", server.port());
    let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();

    let welcome = next_json(&mut socket).await;
    assert_eq!(welcome["type"], "welcome");
    assert_eq!(welcome["role"], "presenter");
    assert!(welcome["snapshot"]["schedule"].is_object());

    // Allowed: black the screen.
    let black =
        json!({ "type": "dispatch", "id": 1, "command": { "type": "setBlack", "on": true } });
    socket
        .send(Message::Text(black.to_string().into()))
        .await
        .unwrap();
    let answer = next_json(&mut socket).await;
    assert_eq!(answer["id"], 1);
    assert_eq!(answer["error"], Value::Null);

    // Not allowed: the logo is the operator's.
    let logo = json!({ "type": "dispatch", "id": 2, "command": { "type": "setLogo", "on": true } });
    socket
        .send(Message::Text(logo.to_string().into()))
        .await
        .unwrap();
    let answer = next_json(&mut socket).await;
    assert_eq!(answer["id"], 2);
    assert!(answer["error"].as_str().unwrap().contains("not allowed"));
    assert_eq!(
        host.commands.lock().unwrap().len(),
        1,
        "the logo never reached the engine"
    );

    // Events from the engine reach the phone.
    let events = host
        .engine
        .lock()
        .unwrap()
        .dispatch(Command::SetHideText { on: true })
        .unwrap();
    for event in &events {
        server.publish(event);
    }
    let event = next_json(&mut socket).await;
    assert_eq!(event["type"], "event");

    // Removing the device disconnects it.
    server.remove(device_id);
    assert_eq!(next_json(&mut socket).await["type"], "removed");
    server.stop();
}

#[tokio::test]
async fn an_unknown_token_is_refused() {
    let server = RemoteServer::start(
        ServerConfig {
            port: 0,
            static_dir: PathBuf::from("does-not-exist"),
        },
        Arc::new(FakeHost::default()),
        Vec::new(),
    )
    .await
    .unwrap();
    let url = format!("ws://127.0.0.1:{}/ws?token=guess", server.port());
    assert!(tokio_tungstenite::connect_async(url).await.is_err());
    server.stop();
}
