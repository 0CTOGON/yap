pub fn banner() {
    println!();
    println!("==============================");
    println!("          YAP");
    println!("==============================");
    println!();
}

pub fn chat_message(
    from: &str,
    message: &str,
) {
    println!();
    println!("{from}: {message}");
}

pub fn direct_message(
    from: &str,
    to: &str,
    message: &str,
) {
    println!();
    println!("{from} -> {to}: {message}");
}

pub fn connected(
    username: &str,
    address: &str,
) {
    println!();
    println!(
        "*** {username} connected ({address})"
    );
}

pub fn disconnected(
    username: &str,
) {
    println!();
    println!(
        "*** {username} disconnected"
    );
}