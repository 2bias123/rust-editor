use std::io::{self, Read, Result, Write};
use std::os::unix::io::AsRawFd;
use termios::*;
use termion::terminal_size;

struct TerminalRawMode {
    screen_rows: u16,
    screen_cols: u16,
    original_termios: Termios,
}

impl TerminalRawMode {
    fn new() -> Result<Self> {
        let stdin_fd = io::stdin().as_raw_fd();
        let original_termios = Termios::from_fd(stdin_fd)?;
        let (screen_cols, screen_rows) = Self::get_window_size();
        Ok(TerminalRawMode { screen_rows, screen_cols, original_termios })
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
        for y in 0..self.screen_rows{
            self.write_escape_seq("~\r\n");
        }
    }

    fn editor_refresh_screen(&self) -> Result<()>{
        self.write_escape_seq("\x1b[2J");
        self.write_escape_seq("\x1b[H");
        self.editor_draw_rows();
        self.write_escape_seq("\x1b[H");
        Ok(())
    }

    fn write_escape_seq(&self, seq: &str) {
        let mut stdout = io::stdout().lock();
        stdout.write_all(seq.as_bytes());
        stdout.flush();
    }

    fn get_window_size() -> (u16, u16) {
        let terminal_size = match terminal_size() {
            Ok(term_size) => term_size,
            Err(_) => {}
        };
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
