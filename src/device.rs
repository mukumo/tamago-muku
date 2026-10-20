use crate::{
    render,
    state::{Action, PetState},
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
fn key_action(code: u16) -> Option<Action> {
    match code {
        57 | 30 | 304 | 103 => Some(Action::Pet), // space/A/gamepad south/up
        48 | 305 | 106 => Some(Action::Feed),     // B/gamepad east/right
        45 | 31 | 44 | 308 | 108 => Some(Action::Play), // X/S/Z/gamepad west/down
        21 | 32 | 307 | 309 | 105 => Some(Action::Sleep), // Y/D/gamepad north/left
        _ => None,
    }
}

pub fn run(
    state: &mut PetState,
    path: &Path,
    seconds: Option<f64>,
    snapshot: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    let mut fb = crate::drm_display::DrmDisplay::open()?;
    let mut inputs = inputs()?;
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    eprintln!(
        "muku awake: 480x272, {} inputs, save={}, START exits",
        inputs.len(),
        path.display()
    );
    let start = Instant::now();
    let mut previous = start;
    let mut last_save = start;
    let mut last_input: Option<(Action, Instant)> = None;
    let mut frames = 0u64;
    let mut snapshot_saved = false;
    while RUNNING.load(Ordering::Relaxed) {
        let now = Instant::now();
        if seconds.is_some_and(|limit| now.duration_since(start).as_secs_f64() >= limit) {
            break;
        }
        state.tick(now.duration_since(previous).as_secs_f32());
        previous = now;
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
                        if [28, 315].contains(&event.code) {
                            RUNNING.store(false, Ordering::Relaxed);
                        } else if let Some(action) = key_action(event.code) {
                            // Duplicate GPIO/i2c events are common on this handheld.
                            if last_input.is_some_and(|(a, t)| {
                                a == action && now.duration_since(t) < Duration::from_millis(160)
                            }) {
                                continue;
                            }
                            state.act(action);
                            state.save(path)?;
                            last_save = now;
                            last_input = Some((action, now));
                            eprintln!("action {action:?}, interactions={}", state.interactions);
                        }
                    }
                    Ok(_) => return Err("partial evdev input packet".into()),
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => return Err(err.into()),
                }
            }
        }
        let frame = render::render(state, now.duration_since(start).as_secs_f32());
        fb.present(&frame)?;
        frames += 1;
        if !snapshot_saved && now.duration_since(start) > Duration::from_secs(2) {
            if let Some(path) = snapshot {
                frame.save_png(path)?;
            }
            snapshot_saved = true;
        }
        if now.duration_since(last_save) > Duration::from_secs(30) {
            state.save(path)?;
            last_save = now;
        }
        std::thread::sleep(Duration::from_millis(50).saturating_sub(now.elapsed()));
    }
    state.save(path)?;
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
    fn dpad_covers_care() {
        assert_eq!(key_action(103), Some(Action::Pet));
        assert_eq!(key_action(106), Some(Action::Feed));
        assert_eq!(key_action(108), Some(Action::Play));
        assert_eq!(key_action(105), Some(Action::Sleep));
    }
}
