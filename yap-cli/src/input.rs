use std::io::{self, Write};

pub struct Input;

impl Input {
    pub fn new() -> Self {
        Self
    }

    pub fn read_line(&mut self) -> io::Result<Option<String>> {
        print!("yap> ");
        io::stdout().flush()?;

        let mut line = String::new();

        let bytes = io::stdin().read_line(&mut line)?;

        if bytes == 0 {
            return Ok(None);
        }

        Ok(Some(line.trim_end().to_string()))
    }
}
