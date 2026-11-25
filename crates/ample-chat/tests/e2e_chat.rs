use std::time::Duration;

use ample_chat::client::{authenticate, DecryptedContent};
use ample_chat_proto::ServerWsMessage;
use ample_chat_server::{run_server_with_listener, ServerConfig};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::test]
async fn test_e2e_multi_client_chat_and_wrong_password() {
    let room_password = "correct-room-password-42";

    // 1. Bind to ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let base_url = format!("http://127.0.0.1:{port}");
    let ws_url = format!("ws://127.0.0.1:{port}");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let config = ServerConfig::new("127.0.0.1", port, room_password);

    // Spawn server in background
    let server_handle = tokio::spawn(async move {
        run_server_with_listener(listener, config, Some(shutdown_rx))
            .await
            .expect("server runs");
    });

    // Give server a moment to start accepting
    tokio::time::sleep(Duration::from_millis(50)).await;

    let http_client = reqwest::Client::new();

    // 2. Test wrong password rejection
    let wrong_auth = authenticate(&http_client, &base_url, "carol", "wrong-password").await;
    assert!(
        wrong_auth.is_err(),
        "Authentication with wrong password must be rejected"
    );

    // 3. Authenticate Alice with correct password
    let alice_session = authenticate(&http_client, &base_url, "alice", room_password)
        .await
        .expect("Alice authenticates");

    // 4. Authenticate Bob with correct password
    let bob_session = authenticate(&http_client, &base_url, "bob", room_password)
        .await
        .expect("Bob authenticates");

    // Both should derive the exact same room key!
    assert_eq!(
        *alice_session.room_key, *bob_session.room_key,
        "Clients with same password must derive identical room keys"
    );

    // 5. Connect Alice to WebSocket
    let alice_ws_url = format!("{ws_url}/ws/chat?user_id={}", alice_session.user_id);
    let (alice_ws, _) = connect_async(&alice_ws_url)
        .await
        .expect("Alice connects ws");
    let (mut alice_write, mut alice_read) = alice_ws.split();

    // Alice receives initial state
    let msg1 = alice_read.next().await.unwrap().unwrap();
    let raw1 = msg1.to_text().unwrap();
    let parsed1: ServerWsMessage = serde_json::from_str(raw1).unwrap();
    assert!(matches!(parsed1, ServerWsMessage::Init { .. }));

    // 6. Connect Bob to WebSocket
    let bob_ws_url = format!("{ws_url}/ws/chat?user_id={}", bob_session.user_id);
    let (bob_ws, _) = connect_async(&bob_ws_url).await.expect("Bob connects ws");
    let (mut bob_write, mut bob_read) = bob_ws.split();

    // Bob receives initial state
    let _ = bob_read.next().await.unwrap().unwrap();

    // Alice receives Bob's user_joined notice
    let mut got_bob_joined = false;
    for _ in 0..2 {
        let join_notice = alice_read.next().await.unwrap().unwrap();
        let raw_join = join_notice.to_text().unwrap();
        let parsed_join: ServerWsMessage = serde_json::from_str(raw_join).unwrap();
        if let ServerWsMessage::UserJoined { username, .. } = parsed_join {
            if username == "bob" {
                got_bob_joined = true;
                break;
            }
        }
    }
    assert!(got_bob_joined, "Alice must receive UserJoined for bob");

    // 7. Alice encrypts and sends a message to the room
    let encrypted_alice_msg = alice_session
        .encrypt_message("Hello from Alice!")
        .expect("Alice encrypts message");
    alice_write
        .send(Message::Text(encrypted_alice_msg))
        .await
        .expect("Alice sends message");

    // 8. Bob receives the broadcasted message from Alice
    let mut got_alice_msg = false;
    for _ in 0..3 {
        let bob_received = bob_read.next().await.unwrap().unwrap();
        let raw_bob_msg = bob_received.to_text().unwrap();
        let parsed_bob_msg: ServerWsMessage = serde_json::from_str(raw_bob_msg).unwrap();

        if let ServerWsMessage::Message { data } = parsed_bob_msg {
            assert_eq!(data.username, "alice");
            // Bob decrypts Alice's message using Bob's room key
            match bob_session.decrypt_message(&data.text) {
                DecryptedContent::Success(payload) => {
                    assert_eq!(payload.sender, "alice");
                    assert_eq!(payload.counter, 0);
                    assert_eq!(payload.content, "Hello from Alice!");
                    got_alice_msg = true;
                    break;
                }
                DecryptedContent::Failed(err) => panic!("Bob failed to decrypt: {err}"),
            }
        }
    }
    assert!(
        got_alice_msg,
        "Bob must receive and decrypt Alice's message"
    );

    // 9. Bob encrypts and sends a reply
    let encrypted_bob_msg = bob_session
        .encrypt_message("Hey Alice, received loud and clear!")
        .expect("Bob encrypts");
    bob_write
        .send(Message::Text(encrypted_bob_msg))
        .await
        .expect("Bob sends");

    // 10. Alice receives Bob's message
    // Alice might first receive her own message echoed if broadcasted to all, so let's find Bob's message
    let mut got_bob_msg = false;
    for _ in 0..2 {
        let alice_rcv = alice_read.next().await.unwrap().unwrap();
        let raw_rcv = alice_rcv.to_text().unwrap();
        let parsed_rcv: ServerWsMessage = serde_json::from_str(raw_rcv).unwrap();
        if let ServerWsMessage::Message { data } = parsed_rcv {
            if data.username == "bob" {
                match alice_session.decrypt_message(&data.text) {
                    DecryptedContent::Success(payload) => {
                        assert_eq!(payload.sender, "bob");
                        assert_eq!(payload.content, "Hey Alice, received loud and clear!");
                        got_bob_msg = true;
                        break;
                    }
                    DecryptedContent::Failed(err) => panic!("Alice failed to decrypt: {err}"),
                }
            }
        }
    }
    assert!(got_bob_msg, "Alice must receive and decrypt Bob's reply");

    // 11. Alice disconnects
    drop(alice_write);
    drop(alice_read);

    // 12. Bob should receive UserLeft notice for Alice
    let mut got_alice_left = false;
    for _ in 0..3 {
        let left_notice = bob_read.next().await.unwrap().unwrap();
        let raw_left = left_notice.to_text().unwrap();
        let parsed_left: ServerWsMessage = serde_json::from_str(raw_left).unwrap();
        if let ServerWsMessage::UserLeft { username, .. } = parsed_left {
            if username == "alice" {
                got_alice_left = true;
                break;
            }
        }
    }
    assert!(got_alice_left, "Bob must receive UserLeft for alice");

    // 13. Graceful shutdown
    drop(bob_write);
    drop(bob_read);
    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
}
