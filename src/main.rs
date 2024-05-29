use std::io::{self, stderr, Read, Result, Write};
use std::os::unix::io::AsRawFd;
use ffi::tcgetattr;
use termios::*;

struct TerminalRawMode {
    original_termios: Termios,
}

impl TerminalRawMode {
    fn new() -> Result<Self> {
        let stdin_fd = io::stdin().as_raw_fd();
        let original_termios = Termios::from_fd(stdin_fd)?;
        Ok(TerminalRawMode { original_termios })
    }

    fn enable(&self) {
        let stdin_fd = io::stdin().as_raw_fd();
        let mut termios = self.original_termios;
        tcgetattr(stdin_fd, self.original_termios);
        termios.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        termios.c_oflag &= !(OPOST);
        termios.c_cflag |= !(CS8);
        termios.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        termios.c_cc[VMIN] = 0;
        termios.c_cc[VTIME] = 1;

        tcsetattr(stdin_fd, TCSAFLUSH, &termios);
    }

    fn disable(&self) {
        let stdin_fd = io::stdin().as_raw_fd();
        tcsetattr(stdin_fd, TCSAFLUSH, &self.original_termios);
    }
}

impl Drop for TerminalRawMode {
    fn drop(&mut self) {
        let _ = self.disable();
    }
}

fn main() -> Result<()> {
    let terminal_mode = TerminalRawMode::new().unwrap();
    terminal_mode.enable();

    loop {
        let mut buffer = [0; 1];
        io::stdin().read_exact(&mut buffer).ok();
        if buffer[0].is_ascii_control() {
            println!("{}\r", buffer[0])
        } else {
            println!("{}\r", buffer[0] as char)
        }
        if buffer[0] == b'q' {
            break;
        }
    }

    Ok(())
}
