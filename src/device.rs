use crate::{
    app::{App, AppEvent, Button},
    notifications::WebhookServer,
    render,
};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{self, Read},
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

// Linux framebuffer ABI. On this target c_ulong and userspace pointers are 64-bit.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct BitField {
    offset: u32,
    length: u32,
    msb_right: u32,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct VarInfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: BitField,
    green: BitField,
    blue: BitField,
    transp: BitField,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
struct FixInfo {
    id: [u8; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    type_: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

fn framebuffer_info() -> io::Result<(VarInfo, FixInfo)> {
    let file = File::open("/dev/fb0")?;
    let mut var = VarInfo::default();
    let mut fix = FixInfo::default();
    // Read-only ioctls write exactly these repr(C) kernel structures.
    if unsafe { libc::ioctl(file.as_raw_fd(), 0x4600, &mut var) } < 0
        || unsafe { libc::ioctl(file.as_raw_fd(), 0x4602, &mut fix) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok((var, fix))
}

static RUNNING: AtomicBool = AtomicBool::new(true);
extern "C" fn stop(_: libc::c_int) {
    RUNNING.store(false, Ordering::Relaxed);
}

pub fn inspect() -> Result<(), Box<dyn Error>> {
    let (var, fix) = framebuffer_info()?;
    println!("framebuffer: {var:?}\nfixed: {fix:?}");
    println!(
        "input_event ABI bytes: {}",
        std::mem::size_of::<libc::input_event>()
    );
    Ok(())
}

fn inputs() -> io::Result<Vec<(String, File)>> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let path = entry?.path();
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        {
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&path)?;
            result.push((path.display().to_string(), file));
        }
    }
    if result.is_empty() {
        return Err(io::Error::other("no input event devices"));
    }
    Ok(result)
}

// Defaults cover this handheld's keyboard-style controls plus standard gamepads.
fn key_button(code: u16) -> Option<Button> {
    match code {
        103 | 544 => Some(Button::Up),
        108 | 545 => Some(Button::Down),
        105 | 546 => Some(Button::Left),
        106 | 547 => Some(Button::Right),
        30 | 304 => Some(Button::A),
        48 | 305 => Some(Button::B),
        45 | 308 => Some(Button::X),
        21 | 307 => Some(Button::Y),
        28 | 315 => Some(Button::Start),
        _ => None,
    }
}

pub fn run(
    app: &mut App,
    path: &Path,
    seconds: Option<f64>,
    snapshot: Option<&Path>,
    webhook: Option<&WebhookServer>,
) -> Result<(), Box<dyn Error>> {
    let mut fb = crate::drm_display::DrmDisplay::open()?;
    let mut inputs = inputs()?;
    let (display_width, display_height) = fb.dimensions();
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    eprintln!(
        "muku awake: {display_width}x{display_height}, {} inputs, save={}, START exits",
        inputs.len(),
        path.display()
    );
    let start = Instant::now();
    let mut previous = start;
    let mut last_save = start;
    let mut last_input: Option<(Button, Instant)> = None;
    let mut frames = 0u64;
    let mut snapshot_saved = false;
    while RUNNING.load(Ordering::Relaxed) {
        let now = Instant::now();
        if seconds.is_some_and(|limit| now.duration_since(start).as_secs_f64() >= limit) {
            break;
        }
        app.tick(now.duration_since(previous).as_secs_f32());
        previous = now;
        if let Some(server) = webhook {
            while let Ok(notice) = server.try_recv() {
                eprintln!("notice from {}: {}", notice.source, notice.title);
                app.push_notice(notice);
            }
        }
        for (name, file) in &mut inputs {
            loop {
                let mut buf = [0u8; std::mem::size_of::<libc::input_event>()];
                match file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) if n == buf.len() => {
                        // Byte array may be unaligned, so explicitly use read_unaligned.
                        let event = unsafe {
                            std::ptr::read_unaligned(buf.as_ptr().cast::<libc::input_event>())
                        };
                        if event.type_ != 1 || event.value != 1 {
                            continue;
                        }
                        eprintln!("key {name}: {} pressed", event.code);
                        if let Some(button) = key_button(event.code) {
                            // Duplicate GPIO/i2c events are common on this handheld.
                            if last_input.is_some_and(|(b, t)| {
                                b == button && now.duration_since(t) < Duration::from_millis(160)
                            }) {
                                continue;
                            }
                            match app.handle(button) {
                                AppEvent::Exit => RUNNING.store(false, Ordering::Relaxed),
                                AppEvent::Save => {
                                    app.pet.save(path)?;
                                    last_save = now;
                                }
                                AppEvent::None => {}
                            }
                            last_input = Some((button, now));
                            eprintln!("button {button:?}, interactions={}", app.pet.interactions);
                        }
                    }
                    Ok(_) => return Err("partial evdev input packet".into()),
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => return Err(err.into()),
                }
            }
        }
        let frame = render::render_app(app, now.duration_since(start).as_secs_f32());
        fb.present(&frame)?;
        frames += 1;
        if !snapshot_saved && now.duration_since(start) > Duration::from_secs(2) {
            if let Some(path) = snapshot {
                frame.save_png(path)?;
            }
            snapshot_saved = true;
        }
        if now.duration_since(last_save) > Duration::from_secs(30) {
            app.pet.save(path)?;
            last_save = now;
        }
        std::thread::sleep(Duration::from_millis(50).saturating_sub(now.elapsed()));
    }
    app.pet.save(path)?;
    eprintln!(
        "muku saved; {frames} frames in {:.2}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framebuffer_abi() {
        assert_eq!(std::mem::size_of::<VarInfo>(), 160);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(std::mem::size_of::<FixInfo>(), 80);
    }
    #[test]
    fn controls_are_mapped() {
        assert_eq!(key_button(103), Some(Button::Up));
        assert_eq!(key_button(544), Some(Button::Up));
        assert_eq!(key_button(105), Some(Button::Left));
        assert_eq!(key_button(547), Some(Button::Right));
        assert_eq!(key_button(30), Some(Button::A));
        assert_eq!(key_button(48), Some(Button::B));
        assert_eq!(key_button(45), Some(Button::X));
        assert_eq!(key_button(21), Some(Button::Y));
        assert_eq!(key_button(28), Some(Button::Start));
    }
}
