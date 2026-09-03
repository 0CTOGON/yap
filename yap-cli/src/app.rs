use std::env;

use tokio::sync::mpsc;

use yap_core::{IncomingMessage, LocalIdentity, Network};

use crate::{cli, commands, input::Input, output};

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let username = env::var("YAP_NAME").unwrap_or_else(|_| "Blyth".into());

    let requested_port = env::var("YAP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(7331);

    let identity = LocalIdentity::new(username)?;

    let (events_tx, mut events_rx) = mpsc::unbounded_channel();

    let network = match Network::bind(identity, requested_port, events_tx.clone()).await {
        Ok(network) => network,

        Err(error) if requested_port != 0 => {
            eprintln!("Port {requested_port} is unavailable: {error}");

            eprintln!("Trying an automatic port instead...");

            let fallback_identity =
                LocalIdentity::new(env::var("YAP_NAME").unwrap_or_else(|_| "Blyth".into()))?;

            Network::bind(fallback_identity, 0, events_tx).await?
        }

        Err(error) => {
            return Err(error);
        }
    };

    output::banner();

    println!("Listening on {}", network.local_addr()?);

    let event_task = tokio::spawn(async move {
        while let Some(event) = events_rx.recv().await {
            match event {
                IncomingMessage::Direct { from, to, message } => {
                    output::direct_message(&from, &to, &message);
                }

                IncomingMessage::Chat { from, message } => {
                    output::chat_message(&from, &message);
                }

                IncomingMessage::PeerConnected { username, address } => {
                    output::connected(&username, &address.to_string());
                }

                IncomingMessage::PeerDisconnected { username } => {
                    output::disconnected(&username);
                }
            }
        }
    });

    let mut input = Input::new();

    loop {
        match input.read_line() {
            Ok(Some(line)) => {
                let command = cli::parse(&line);

                if commands::execute(command, &network).await {
                    break;
                }
            }

            Ok(None) => {
                println!("Goodbye.");
                break;
            }

            Err(error) => {
                eprintln!("Input error: {error}");
                break;
            }
        }
    }

    event_task.abort();

    Ok(())
}
