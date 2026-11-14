//! Terminal facts and bounded capability detection. Redirected output never probes.
use crate::config::{Backend, Display};
#[derive(Debug, Clone, Copy)]
pub struct CellPixels {
    pub width: u16,
    pub height: u16,
}
#[derive(Debug, Clone, Copy)]
pub struct Terminal {
    pub width: u16,
    pub height: u16,
    pub cell: CellPixels,
}
impl Terminal {
    pub fn detect(display: &Display) -> Self {
        if let Ok(size) = crossterm::terminal::window_size()
            && size.columns > 0
            && size.rows > 0
        {
            return Self {
                width: size.columns,
                height: size.rows,
                cell: CellPixels {
                    width: if size.width > 0 {
                        (size.width / size.columns).clamp(1, 256)
                    } else {
                        display.cell_width
                    },
                    height: if size.height > 0 {
                        (size.height / size.rows).clamp(1, 256)
                    } else {
                        display.cell_height
                    },
                },
            };
        }
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        Self {
            width,
            height,
            cell: CellPixels {
                width: display.cell_width,
                height: display.cell_height,
            },
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Capabilities {
    pub kitty: bool,
    pub iterm2: bool,
    pub sixel: bool,
}
impl Capabilities {
    pub fn features(&mut self, features: &str) {
        let mut token = String::new();
        for c in features
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .chain(std::iter::once('!'))
        {
            if c.is_ascii_uppercase() || c == '!' {
                match token.as_str() {
                    "F" => self.iterm2 = true,
                    "Sx" => self.sixel = true,
                    _ => {}
                }
                token.clear();
            }
            token.push(c);
        }
    }
    pub fn replies(&mut self, bytes: &[u8]) {
        let reply = String::from_utf8_lossy(bytes);
        if reply.contains("\x1b_Gi=314159;OK\x1b\\") {
            self.kitty = true;
        }
        if let Some(start) = reply.find("Capabilities=") {
            self.features(&reply[start + "Capabilities=".len()..]);
        }
        for part in reply.split("\x1b[?").skip(1) {
            if let Some(end) = part.find('c') {
                let params = &part[..end];
                if params.split(';').any(|p| p == "4") {
                    self.sixel = true;
                }
            }
        }
    }
    pub fn select(self) -> Backend {
        if self.kitty {
            Backend::Kitty
        } else if self.iterm2 {
            Backend::Iterm2
        } else if self.sixel {
            Backend::Sixel
        } else {
            Backend::Ansi
        }
    }
}
pub fn backend(
    request: Backend,
    terminal: bool,
    has_images: bool,
    color: bool,
    display: &Display,
) -> std::io::Result<Backend> {
    if !terminal || !color {
        return Ok(Backend::Text);
    }
    if !has_images {
        return Ok(Backend::Text);
    }
    if matches!(request, Backend::Text | Backend::Ansi) {
        return Ok(request);
    }
    let screen = std::env::var_os("STY").is_some();
    let mux = std::env::var_os("TMUX").is_some() || screen;
    if screen || mux && !display.multiplexer_passthrough {
        return Ok(Backend::Ansi);
    }
    if request != Backend::Auto {
        return Ok(request);
    }
    let mut caps = Capabilities::default();
    if let Ok(features) = std::env::var("TERM_FEATURES") {
        caps.features(&features);
    }
    if display.probe && display.probe_timeout_ms > 0 && !mux {
        #[cfg(unix)]
        match probe(display.probe_timeout_ms) {
            Ok(reply) => caps.replies(&reply),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => return Err(e),
            Err(_) => {}
        }
    }
    Ok(caps.select())
}
#[cfg(unix)]
fn probe(timeout: u16) -> std::io::Result<Vec<u8>> {
    use rustix::{
        fs::{OFlags, fcntl_setfl},
        termios::{OptionalActions, Termios, tcgetattr, tcsetattr},
    };
    use std::{
        fs::{File, OpenOptions},
        io::{Read, Write},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    struct Restore {
        file: File,
        original: Termios,
        signals: Vec<signal_hook::SigId>,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = tcsetattr(&self.file, OptionalActions::Now, &self.original);
            for id in &self.signals {
                signal_hook::low_level::unregister(*id);
            }
        }
    }
    let mut file = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    if rustix::io::ioctl_fionread(&file)? > 0 {
        return Ok(vec![]);
    }
    let original = tcgetattr(&file)?;
    let interrupted = Arc::new(AtomicBool::new(false));
    let mut restore = Restore {
        file: file.try_clone()?,
        original: original.clone(),
        signals: vec![],
    };
    for signal in [
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
    ] {
        restore
            .signals
            .push(signal_hook::flag::register(signal, interrupted.clone())?);
    }
    let mut raw = original;
    raw.make_raw();
    tcsetattr(&file, OptionalActions::Now, &raw)?;
    fcntl_setfl(&file, OFlags::NONBLOCK)?;
    file.write_all(
        b"\x1b_Gi=314159,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b]1337;Capabilities\x1b\\\x1b[c",
    )?;
    file.flush()?;
    let started = Instant::now();
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1024];
    while started.elapsed() < Duration::from_millis(u64::from(timeout)) && bytes.len() < 8192 {
        if interrupted.load(Ordering::Relaxed) {
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        match file.read(&mut buf) {
            Ok(n) if n > 0 => bytes.extend_from_slice(&buf[..n]),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }
        if bytes.ends_with(b"c") {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(restore);
    Ok(bytes)
}
