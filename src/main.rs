use std::io::{self, Read, Result, Write};
use std::os::unix::io::AsRawFd;
use termion::terminal_size;
use termios::*;

struct TerminalRawMode {
    screen_rows: u16,
    screen_cols: u16,
    original_termios: Termios,
    version: String
}

impl TerminalRawMode {
    fn new() -> Result<Self> {
        let stdin_fd = io::stdin().as_raw_fd();
        let original_termios = Termios::from_fd(stdin_fd)?;
        let (screen_cols, screen_rows) = Self::get_window_size()?;
        let version = String::from("0.0.1");
        Ok(TerminalRawMode {
            screen_rows,
            screen_cols,
            original_termios,
            version
        })
    }

    fn enable(&self) -> Result<()> {
        let stdin_fd = io::stdin().as_raw_fd();
        let mut termios = self.original_termios;

        termios.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        termios.c_oflag &= !(OPOST);
        termios.c_cflag |= !(CS8);
        termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        termios.c_cc[VMIN] = 0;
        termios.c_cc[VTIME] = 1;

        tcsetattr(stdin_fd, TCSAFLUSH, &termios)?;
        Ok(())
    }

    fn disable(&self) -> Result<()> {
        let stdin_fd = io::stdin().as_raw_fd();
        tcsetattr(stdin_fd, TCSAFLUSH, &self.original_termios)?;
        Ok(())
    }

    fn crtl_key(&self, k: char) -> u8 {
        (k as u8) & 0x1f
    }

    fn editor_read_key() -> Result<char> {
        let mut buffer = [0; 1];
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        loop {
            match handle.read(&mut buffer) {
                Ok(0) => {
                    continue;
                }
                Ok(_) => {
                    return Ok(buffer[0] as char);
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    fn editor_process_key_pressed(&self) -> Result<bool> {
        match TerminalRawMode::editor_read_key() {
            Ok(c) if c as u8 == self.crtl_key('q') => {
                self.editor_refresh_screen()?;
                Ok(true)
            }
            Ok(_) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn editor_draw_rows(&self, buff: &mut String) -> Result<()> {
        let welcome_msg = format!("Spaghetti editor -- verison {}",self.version);
        for y in 0..self.screen_rows {
            if y == self.screen_rows / 3 {
                let welcome_len = welcome_msg.len() as u16;
                let mut padding = (self.screen_cols.saturating_sub(welcome_len)) / 2;

                if padding > 0{
                    buff.push_str("~");
                    padding -= 1;
                }
                while padding > 0 {
                    padding -= 1;
                    buff.push_str(" ")
                }
                buff.push_str(&welcome_msg);
            } else {
                buff.push_str("~");
            }

            buff.push_str("\x1b[K");
            if y < self.screen_rows - 1 {
                buff.push_str("\r\n");
            }
        }
        Ok(())
    }

    fn editor_refresh_screen(&self) -> Result<()> {
        let mut buff = String::new();

        buff.push_str("\x1b[?25l");
        buff.push_str("\x1b[H");

        self.editor_draw_rows(&mut buff);

        buff.push_str("\x1b[H");
        buff.push_str("\x1b[?25h");

        TerminalRawMode::write(&buff)?;
        Ok(())
    }

    fn write(seq: &str) -> Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(seq.as_bytes())?;
        stdout.flush()?;
        Ok(())
    }

    fn get_cursor_position() -> Result<(u16, u16)> {
        TerminalRawMode::write("\x1b[6n")?;

        let mut buffer = [0; 1];
        let mut position = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        loop {
            match handle.read(&mut buffer) {
                Ok(1) => {
                    let c = buffer[0] as char;
                    position.push(c);
                    if c == 'R' {
                        break;
                    }
                }
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }

        if position.starts_with("\x1b[") && position.ends_with('R') {
            let coords: Vec<&str> = position[2..position.len() - 1].split(';').collect();
            if coords.len() == 2 {
                if let (Ok(cols), Ok(rows)) = (coords[1].parse::<u16>(), coords[0].parse::<u16>()) {
                    return Ok((cols, rows));
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to get cursor position.",
        ))
    }

    fn get_window_size() -> Result<(u16, u16)> {
        match terminal_size() {
            Ok((cols, rows)) => Ok((cols, rows)),
            Err(_) => {
                TerminalRawMode::write("\x1b[999C\x1b[999B")?;
                TerminalRawMode::get_cursor_position()
            }
        }
    }
}

impl Drop for TerminalRawMode {
    fn drop(&mut self) {
        let _ = self.editor_refresh_screen();
        let _ = self.disable();
    }
}

fn main() -> Result<()> {
    let terminal_mode = TerminalRawMode::new().unwrap();
    terminal_mode.enable()?;

    loop {
        terminal_mode.editor_refresh_screen()?;
        if terminal_mode.editor_process_key_pressed()? {
            break;
        }
    }
    Ok(())
}
