use std::io::{self, Read, Result, Write};
use std::os::unix::io::AsRawFd;
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

    fn editor_read_key(&self) -> char {
        let mut buffer = [0; 1];
        while io::stdin().read_exact(&mut buffer).is_ok() {}
        buffer[0] as char
    }

    fn editor_process_key_pressed(&self) -> Result<bool> {
        let c: char = self.editor_read_key();
        if c as u8 == Self::crtl_key(&self,'q') {
            self.editor_refresh_screen();
            return Ok(true)
        }
        Ok(false)
    }

    fn editor_draw_rows(&self) {
        let mut stdout = io::stdout().lock();
        for y in 0..24 {
            stdout.write_all(b"~\r\n");
        }
    }

    fn editor_refresh_screen(&self) -> Result<()>{
        let mut stdout = io::stdout().lock();
        stdout.write_all(b"\x1b[2J")?;
        stdout.write_all(b"\x1b[H")?;

        self.editor_draw_rows();
        stdout.write_all(b"\x1b[H");
        stdout.flush()?;
        Ok(())
    }
}

impl Drop for TerminalRawMode {
    fn drop(&mut self) {
        let _ =self.editor_refresh_screen();
        let _ = self.disable();
    }
}

fn main() -> Result<()> {
    let terminal_mode = TerminalRawMode::new().unwrap();
    terminal_mode.enable()?;

    loop {
        terminal_mode.editor_refresh_screen();
        if terminal_mode.editor_process_key_pressed()? {
            break;
        }
    }

    Ok(())
}
