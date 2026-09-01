#[derive(Debug)]
pub enum Command {
    Help,

    Connect {
        address: String,
    },

    Disconnect {
        username: String,
    },

    Peers,

    Yap {
        message: String,
    },

    To {
        username: String,
        message: String,
    },

    Name {
        username: String,
    },

    Quit,

    Empty,

    Unknown {
        command: String,
    },
}

pub fn parse(line: &str) -> Command {
    let line = line.trim();

    if line.is_empty() {
        return Command::Empty;
    }

    let mut parts =
        line.splitn(2, ' ');

    let command =
        parts.next().unwrap_or("");

    let rest =
        parts.next().unwrap_or("").trim();

    match command.to_lowercase().as_str() {
        "help" | "?" => {
            Command::Help
        }

        "connect" => {
            Command::Connect {
                address: rest.to_string(),
            }
        }

        "disconnect" => {
            Command::Disconnect {
                username: rest.to_string(),
            }
        }

        "peers" => {
            Command::Peers
        }

        "yap" => {
            Command::Yap {
                message: rest.to_string(),
            }
        }

        "to" => {
            let mut args =
                rest.splitn(2, ' ');

            let username =
                args.next()
                    .unwrap_or("")
                    .to_string();

            let message =
                args.next()
                    .unwrap_or("")
                    .trim()
                    .to_string();

            Command::To {
                username,
                message,
            }
        }

        "name" => {
            Command::Name {
                username: rest.to_string(),
            }
        }

        "quit" | "exit" => {
            Command::Quit
        }

        _ => {
            Command::Unknown {
                command: command.to_string(),
            }
        }
    }
}