//! Software-rendered KMS output for the handheld's 480 × 272 panel.
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
            .filter(|info| info.modes().iter().any(|mode| mode.size() == (480, 272)))
            .min_by_key(|info| info.interface() != connector::Interface::DPI)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "no connected 480x272 DRM panel")
            })?;
        let mode = panel
            .modes()
            .iter()
            .copied()
            .find(|mode| mode.size() == (480, 272))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "panel mode disappeared"))?;

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
            (480, 272),
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
        if image.width() != buffer.size().0 || image.height() != buffer.size().1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "DRM image must match the 480x272 panel",
            ));
        }
        let pitch = buffer.pitch() as usize;
        let mut mapping = self.card.map_dumb_buffer(buffer)?;
        copy_rgba_to_xrgb(image, mapping.as_mut(), pitch)
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

fn copy_rgba_to_xrgb(image: &tiny_skia::Pixmap, dest: &mut [u8], pitch: usize) -> io::Result<()> {
    let row_bytes = image.width() as usize * 4;
    let height = image.height() as usize;
    let required = pitch
        .checked_mul(height)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "DRM buffer size overflow"))?;
    if pitch < row_bytes || dest.len() < required {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DRM buffer is smaller than its image or pitch",
        ));
    }
    // tiny-skia stores premultiplied RGBA. XRGB8888 is B,G,R,X in memory
    // on this little-endian device. Transparent pixels composite onto black.
    for (source, target) in image
        .data()
        .chunks_exact(row_bytes)
        .zip(dest[..required].chunks_exact_mut(pitch))
    {
        for (rgba, bgrx) in source
            .chunks_exact(4)
            .zip(target[..row_bytes].chunks_exact_mut(4))
        {
            bgrx.copy_from_slice(&[rgba[2], rgba[1], rgba[0], 255]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::copy_rgba_to_xrgb;

    #[test]
    fn converts_channels_and_preserves_stride_padding() {
        let image = tiny_skia::Pixmap::from_vec(
            vec![10, 20, 30, 255, 40, 50, 60, 255],
            tiny_skia::IntSize::from_wh(1, 2).unwrap(),
        )
        .unwrap();
        let mut dest = vec![0xa5; 16];
        copy_rgba_to_xrgb(&image, &mut dest, 8).unwrap();
        assert_eq!(&dest[..4], &[30, 20, 10, 255]);
        assert_eq!(&dest[8..12], &[60, 50, 40, 255]);
        assert_eq!(&dest[4..8], &[0xa5; 4]);
        assert_eq!(&dest[12..], &[0xa5; 4]);
    }

    #[test]
    fn rejects_short_mapping_or_pitch() {
        let image = tiny_skia::Pixmap::new(2, 2).unwrap();
        assert!(copy_rgba_to_xrgb(&image, &mut [0; 15], 8).is_err());
        assert!(copy_rgba_to_xrgb(&image, &mut [0; 16], 4).is_err());
    }
}
