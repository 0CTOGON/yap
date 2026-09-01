use yap_core::Network;

use crate::cli::Command;

pub async fn execute(
    command: Command,
    network: &Network,
) -> bool {
    match command {
        Command::Empty => {}

        Command::Help => {
            println!();
            println!("Commands:");
            println!("  connect <host:port>");
            println!("  disconnect <username>");
            println!("  peers");
            println!("  yap <message>");
            println!("  to <username> <message>");
            println!("  name <username>");
            println!("  help");
            println!("  quit");
            println!();
        }

        Command::Connect { address } => {
            if address.is_empty() {
                println!(
                    "Usage: connect <host:port>"
                );
                return false;
            }

            println!(
                "Connecting to {address}..."
            );

            match network
                .connect(&address)
                .await
            {
                Ok(actual) => {
                    println!(
                        "Connected to {actual}"
                    );
                }

                Err(error) => {
                    println!(
                        "Connection failed: {error}"
                    );
                }
            }
        }

        Command::Disconnect {
            username,
        } => {
            if username.is_empty() {
                println!(
                    "Usage: disconnect <username>"
                );
                return false;
            }

            match network
                .disconnect(&username)
                .await
            {
                Ok(()) => {
                    println!(
                        "Disconnected from {username}."
                    );
                }

                Err(error) => {
                    println!("Error: {error}");
                }
            }
        }

        Command::Peers => {
            let peers =
                network.peers().await;

            if peers.is_empty() {
                println!(
                    "No directly connected peers."
                );
            } else {
                println!("Peers:");

                for peer in peers {
                    println!("  {peer}");
                }
            }
        }

        Command::Yap { message } => {
            if message.trim().is_empty() {
                println!(
                    "Usage: yap <message>"
                );
                return false;
            }

            match network
                .broadcast(&message)
                .await
            {
                Ok(()) => {
                    println!(
                        "{}: {}",
                        network.username().await,
                        message
                    );
                }

                Err(error) => {
                    println!(
                        "Yap failed: {error}"
                    );
                }
            }
        }

        Command::To {
            username,
            message,
        } => {
            if username.trim().is_empty()
                || message.trim().is_empty()
            {
                println!(
                    "Usage: to <username> <message>"
                );
                return false;
            }

            match network
                .send_direct(
                    &username,
                    &message,
                )
                .await
            {
                Ok(()) => {
                    println!(
                        "{} -> {}: {}",
                        network.username().await,
                        username,
                        message
                    );
                }

                Err(error) => {
                    println!(
                        "Send failed: {error}"
                    );
                }
            }
        }

        Command::Name { username } => {
            if username.is_empty() {
                println!(
                    "Usage: name <username>"
                );
                return false;
            }

            match network
                .set_username(&username)
                .await
            {
                Ok(()) => {
                    println!(
                        "Username changed to {username}."
                    );
                }

                Err(error) => {
                    println!(
                        "Name change failed: {error}"
                    );
                }
            }
        }

        Command::Quit => {
            return true;
        }

        Command::Unknown { command } => {
            println!(
                "Unknown command: {command}"
            );

            println!(
                "Try 'help'."
            );
        }
    }

    false
}