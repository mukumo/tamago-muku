//! Software-rendered KMS output for the handheld's DRM panel.
//!
//! The launcher must stop the stock UI and its respawn loop before opening this
//! device. Opening the card alone does not ensure that we own DRM master.

use drm::buffer::{Buffer, DrmFourcc};
use drm::control::{connector, crtc, dumbbuffer, framebuffer, Device as ControlDevice};
use drm::Device as _;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsFd, BorrowedFd};

struct Card(File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for Card {}
impl ControlDevice for Card {}

/// Owns DRM master and a single XRGB8888 dumb buffer.
///
/// The saved CRTC is restored on normal shutdown where its old framebuffer is
/// still available. The launcher remains responsible for restarting the stock
/// UI, whose framebuffer may have disappeared when its process exited.
pub struct DrmDisplay {
    card: Card,
    old_crtc: crtc::Info,
    old_connectors: Vec<connector::Handle>,
    buffer: Option<dumbbuffer::DumbBuffer>,
    framebuffer: Option<framebuffer::Handle>,
    active: bool,
}

impl DrmDisplay {
    pub fn open() -> io::Result<Self> {
        let card = Card(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/dri/card0")?,
        );
        card.acquire_master_lock().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot acquire DRM master; stop the stock UI first: {error}"),
            )
        })?;

        let resources = card.resource_handles()?;
        let connectors = resources
            .connectors()
            .iter()
            .map(|&handle| card.get_connector(handle, true))
            .collect::<io::Result<Vec<_>>>()?;
        let panel = connectors
            .iter()
            .filter(|info| info.state() == connector::State::Connected)
            .filter(|info| !info.modes().is_empty())
            .min_by_key(|info| {
                !matches!(
                    info.interface(),
                    connector::Interface::DPI | connector::Interface::DSI
                )
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no connected DRM panel"))?;
        let mode = panel
            .modes()
            .iter()
            .copied()
            .max_by_key(|mode| u64::from(mode.size().0) * u64::from(mode.size().1))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "panel mode disappeared"))?;
        let (width, height) = mode.size();
        let (width, height) = (u32::from(width), u32::from(height));
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported DRM panel dimensions",
            ));
        }

        // Prefer the panel's current encoder/CRTC. A fallback must respect the
        // encoder's possible_crtcs mask rather than selecting an arbitrary CRTC.
        let mut encoder_handles = panel.encoders().to_vec();
        if let Some(current) = panel.current_encoder() {
            encoder_handles.retain(|&handle| handle != current);
            encoder_handles.insert(0, current);
        }
        let mut selected_crtc = None;
        for handle in encoder_handles {
            let encoder = card.get_encoder(handle)?;
            selected_crtc = encoder.crtc().or_else(|| {
                resources
                    .filter_crtcs(encoder.possible_crtcs())
                    .first()
                    .copied()
            });
            if selected_crtc.is_some() {
                break;
            }
        }
        let crtc = selected_crtc.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no compatible CRTC for the panel")
        })?;
        let old_crtc = card.get_crtc(crtc)?;
        let mut old_connectors = Vec::new();
        for info in &connectors {
            if let Some(encoder) = info.current_encoder() {
                if card.get_encoder(encoder)?.crtc() == Some(crtc) {
                    old_connectors.push(info.handle());
                }
            }
        }

        // Build the owner before allocating so later failures also clean up.
        let mut display = Self {
            card,
            old_crtc,
            old_connectors,
            buffer: None,
            framebuffer: None,
            active: false,
        };
        display.buffer = Some(display.card.create_dumb_buffer(
            (width, height),
            DrmFourcc::Xrgb8888,
            32,
        )?);
        let buffer = display
            .buffer
            .as_mut()
            .ok_or_else(|| io::Error::other("DRM buffer was not created"))?;
        // Clear all bytes, including pitch padding, before scanout starts.
        display.card.map_dumb_buffer(buffer)?.as_mut().fill(0);
        let framebuffer = display.card.add_framebuffer(buffer, 24, 32)?;
        display.framebuffer = Some(framebuffer);
        display.card.set_crtc(
            crtc,
            Some(framebuffer),
            (0, 0),
            &[panel.handle()],
            Some(mode),
        )?;
        display.active = true;
        Ok(display)
    }

    pub fn present(&mut self, image: &tiny_skia::Pixmap) -> io::Result<()> {
        let buffer = self
            .buffer
            .as_mut()
            .ok_or_else(|| io::Error::other("DRM display has no buffer"))?;
        let pitch = buffer.pitch() as usize;
        let (width, height) = buffer.size();
        let mut mapping = self.card.map_dumb_buffer(buffer)?;
        scale_rgba_to_xrgb(image, mapping.as_mut(), pitch, (width, height))
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.buffer
            .as_ref()
            .map(Buffer::size)
            .expect("DRM display always owns a buffer")
    }
}

impl Drop for DrmDisplay {
    fn drop(&mut self) {
        if self.active {
            let restored =
                if self.old_crtc.framebuffer().is_some() && self.old_crtc.mode().is_some() {
                    self.card.set_crtc(
                        self.old_crtc.handle(),
                        self.old_crtc.framebuffer(),
                        self.old_crtc.position(),
                        &self.old_connectors,
                        self.old_crtc.mode(),
                    )
                } else {
                    self.card
                        .set_crtc(self.old_crtc.handle(), None, (0, 0), &[], None)
                };
            if restored.is_err() {
                // The stock UI may have destroyed its old framebuffer. Disable
                // our scanout before removing its resources; the launcher will
                // recreate the stock UI's display state.
                let _ = self
                    .card
                    .set_crtc(self.old_crtc.handle(), None, (0, 0), &[], None);
            }
        }
        if let Some(framebuffer) = self.framebuffer.take() {
            let _ = self.card.destroy_framebuffer(framebuffer);
        }
        if let Some(buffer) = self.buffer.take() {
            let _ = self.card.destroy_dumb_buffer(buffer);
        }
        let _ = self.card.release_master_lock();
    }
}

fn scale_rgba_to_xrgb(
    image: &tiny_skia::Pixmap,
    dest: &mut [u8],
    pitch: usize,
    target: (u32, u32),
) -> io::Result<()> {
    let (target_width, target_height) = target;
    let target_row_bytes = target_width as usize * 4;
    let required = pitch
        .checked_mul(target_height as usize)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "DRM buffer size overflow"))?;
    if image.width() == 0
        || image.height() == 0
        || target_width == 0
        || target_height == 0
        || pitch < target_row_bytes
        || dest.len() < required
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DRM buffer is smaller than its image or pitch",
        ));
    }

    // Fit without changing the 480x272 aspect ratio, then center the result.
    // Use the UI paper color for the surrounding area on a 4:3 panel.
    let scale = (f64::from(target_width) / f64::from(image.width()))
        .min(f64::from(target_height) / f64::from(image.height()));
    let scaled_width = ((f64::from(image.width()) * scale).round() as u32).clamp(1, target_width);
    let scaled_height =
        ((f64::from(image.height()) * scale).round() as u32).clamp(1, target_height);
    let left = (target_width - scaled_width) / 2;
    let top = (target_height - scaled_height) / 2;
    let background = &image.data()[..4];

    for y in 0..target_height as usize {
        let row = &mut dest[y * pitch..y * pitch + target_row_bytes];
        for pixel in row.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[background[2], background[1], background[0], 255]);
        }
    }

    // tiny-skia stores premultiplied RGBA. XRGB8888 is B,G,R,X in memory
    // on this little-endian device.
    let source_stride = image.width() as usize * 4;
    for y in 0..scaled_height {
        let source_y =
            ((u64::from(y) * u64::from(image.height())) / u64::from(scaled_height)) as usize;
        for x in 0..scaled_width {
            let source_x =
                ((u64::from(x) * u64::from(image.width())) / u64::from(scaled_width)) as usize;
            let source = &image.data()[source_y * source_stride + source_x * 4..][..4];
            let target_offset =
                (top as usize + y as usize) * pitch + (left as usize + x as usize) * 4;
            dest[target_offset..target_offset + 4]
                .copy_from_slice(&[source[2], source[1], source[0], 255]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::scale_rgba_to_xrgb;

    #[test]
    fn scales_channels_and_preserves_aspect_ratio() {
        let image = tiny_skia::Pixmap::from_vec(
            vec![10, 20, 30, 255, 40, 50, 60, 255],
            tiny_skia::IntSize::from_wh(1, 2).unwrap(),
        )
        .unwrap();
        let mut dest = vec![0xa5; 32];
        scale_rgba_to_xrgb(&image, &mut dest, 16, (4, 2)).unwrap();
        assert_eq!(&dest[..4], &[30, 20, 10, 255]);
        assert_eq!(&dest[4..8], &[30, 20, 10, 255]);
        assert_eq!(&dest[8..12], &[30, 20, 10, 255]);
        assert_eq!(&dest[12..16], &[30, 20, 10, 255]);
        assert_eq!(&dest[16..20], &[60, 50, 40, 255]);
        assert_eq!(&dest[28..32], &[60, 50, 40, 255]);
    }

    #[test]
    fn rejects_short_mapping_or_pitch() {
        let image = tiny_skia::Pixmap::new(2, 2).unwrap();
        assert!(scale_rgba_to_xrgb(&image, &mut [0; 15], 8, (2, 2)).is_err());
        assert!(scale_rgba_to_xrgb(&image, &mut [0; 16], 4, (2, 2)).is_err());
    }
}
