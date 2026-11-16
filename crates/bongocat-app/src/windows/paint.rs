use super::*;
use ::windows::Win32::Graphics::Gdi::*;

/// GDI resources never leave the UI thread. The previous bitmap is restored before deletion.
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    width: i32,
    height: i32,
}
impl Surface {
    fn new(width: i32, height: i32) -> Result<Self> {
        // SAFETY: bounded dimensions, valid local BITMAPINFO, exclusively owned memory DC.
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err(Error::from_thread().into());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            {
                Ok(bitmap) => bitmap,
                Err(e) => {
                    let _ = DeleteDC(dc);
                    return Err(e.into());
                }
            };
            let old = SelectObject(dc, bitmap.into());
            let mut surface = Self {
                dc,
                bitmap,
                old,
                bits: bits.cast(),
                width,
                height,
            };
            surface.pixels().fill(0);
            Ok(surface)
        }
    }
    fn pixels(&mut self) -> &mut [u32] {
        // SAFETY: this DIB owns width*height writable 32-bit pixels until Drop.
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.width * self.height) as usize) }
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY: restore selection before deleting the owned GDI objects on their thread.
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

pub struct Canvas {
    pub width: i32,
    pub height: i32,
    pixels: Vec<u32>,
}
impl Canvas {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }
    fn blend(&mut self, x: i32, y: i32, color: [u8; 3], alpha: u8) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let p = &mut self.pixels[(y * self.width + x) as usize];
        let a = u32::from(alpha);
        let inverse = 255 - a;
        let channel =
            |shift: u32, c: u8| (u32::from(c) * a + ((*p >> shift) & 255u32) * inverse + 127) / 255;
        *p = (a + ((*p >> 24) * inverse + 127) / 255) << 24
            | channel(16, color[0]) << 16
            | channel(8, color[1]) << 8
            | channel(0, color[2]);
    }
    fn round(&mut self, rect: [f32; 4], radius: f32, color: [u8; 3], alpha: u8) {
        let [x, y, w, h] = rect;
        for py in y.floor().max(0.0) as i32..(y + h).ceil().min(self.height as f32) as i32 {
            for px in x.floor().max(0.0) as i32..(x + w).ceil().min(self.width as f32) as i32 {
                let dx = ((px as f32 + 0.5 - x - w / 2.0).abs() - w / 2.0 + radius).max(0.0);
                let dy = ((py as f32 + 0.5 - y - h / 2.0).abs() - h / 2.0 + radius).max(0.0);
                let coverage = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                self.blend(px, py, color, (f32::from(alpha) * coverage) as u8);
            }
        }
    }
    fn text(
        &mut self,
        text: &str,
        rect: [i32; 4],
        size: i32,
        bold: bool,
        color: [u8; 3],
        center: bool,
    ) -> Result<()> {
        let [x, y, width, height] = rect;
        let mut glyphs = Surface::new(width, height)?;
        // SAFETY: GDI paints only into the owned DIB; font selection is restored before deletion.
        unsafe {
            let font = CreateFontW(
                -size,
                0,
                0,
                0,
                if bold { 600 } else { 400 },
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                0,
                w!("Segoe UI"),
            );
            let old = SelectObject(glyphs.dc, font.into());
            SetBkMode(glyphs.dc, TRANSPARENT);
            SetTextColor(glyphs.dc, COLORREF(0x00ffffff));
            let mut line: Vec<u16> = text.encode_utf16().collect();
            DrawTextW(
                glyphs.dc,
                &mut line,
                &mut RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                },
                DT_SINGLELINE | DT_VCENTER | if center { DT_CENTER } else { DT_LEFT },
            );
            SelectObject(glyphs.dc, old);
            let _ = DeleteObject(font.into());
        }
        for (index, p) in glyphs.pixels().iter().enumerate() {
            let alpha = (*p & 255).max((*p >> 8) & 255).max((*p >> 16) & 255) as u8;
            if alpha > 0 {
                self.blend(
                    x + index as i32 % width,
                    y + index as i32 / width,
                    color,
                    alpha,
                );
            }
        }
        Ok(())
    }
    pub fn present(&self, hwnd: HWND, position: POINT, opacity: u8) -> Result<()> {
        let mut surface = Surface::new(self.width, self.height)?;
        surface.pixels().copy_from_slice(&self.pixels);
        // SAFETY: source memory is live and premultiplied BGRA for this synchronous update.
        unsafe {
            UpdateLayeredWindow(
                hwnd,
                None,
                Some(&position),
                Some(&SIZE {
                    cx: self.width,
                    cy: self.height,
                }),
                Some(surface.dc),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: opacity,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                }),
                ULW_ALPHA,
            )?;
        }
        Ok(())
    }
}

pub fn avatar(image: &image::RgbaImage, width: u32) -> Canvas {
    let height =
        (u64::from(width) * u64::from(image.height()) / u64::from(image.width())).max(1) as u32;
    let resized =
        image::imageops::resize(image, width, height, image::imageops::FilterType::Lanczos3);
    let mut canvas = Canvas::new(width as i32, height as i32);
    for (target, p) in canvas.pixels.iter_mut().zip(resized.pixels()) {
        let [r, g, b, a] = p.0;
        let a = u32::from(a);
        *target = a << 24
            | (u32::from(r) * a / 255) << 16
            | (u32::from(g) * a / 255) << 8
            | (u32::from(b) * a / 255);
    }
    canvas
}

pub fn panel(
    days: u64,
    total: u64,
    elapsed: u64,
    running: bool,
    hover: bool,
    dpi: u32,
) -> Result<Canvas> {
    let scale = dpi as f32 / 96.0;
    let mut canvas = Canvas::new((328.0 * scale) as i32, (228.0 * scale) as i32);
    let rect = |x: f32, y: f32, w: f32, h: f32| [x * scale, y * scale, w * scale, h * scale];
    // Soft shadow and a translucent surface: nothing outside the rounded card is opaque.
    for spread in (1..=10).rev() {
        let n = spread as f32;
        canvas.round(
            rect(12.0 - n, 14.0 - n, 304.0 + n * 2.0, 202.0 + n * 2.0),
            (24.0 + n) * scale,
            [0, 0, 0],
            3,
        );
    }
    canvas.round(
        rect(12.0, 10.0, 304.0, 202.0),
        24.0 * scale,
        [225, 235, 244],
        38,
    );
    canvas.round(
        rect(13.0, 11.0, 302.0, 200.0),
        23.0 * scale,
        [23, 33, 46],
        202,
    );
    let white = [242, 246, 250];
    let muted = [170, 187, 202];
    let accent = if running {
        [109, 224, 183]
    } else {
        [165, 193, 216]
    };
    canvas.round(rect(32.0, 29.0, 6.0, 6.0), 3.0 * scale, accent, 255);
    let r = |x: f32, y: f32, w: f32, h: f32| {
        [
            (x * scale) as i32,
            (y * scale) as i32,
            (w * scale) as i32,
            (h * scale) as i32,
        ]
    };
    canvas.text(
        "我的时光",
        r(46.0, 19.0, 120.0, 26.0),
        (13.0 * scale) as i32,
        true,
        white,
        false,
    )?;
    canvas.text(
        if running { "专注中" } else { "休息中" },
        r(236.0, 19.0, 58.0, 26.0),
        (12.0 * scale) as i32,
        false,
        accent,
        true,
    )?;
    canvas.text(
        &days.to_string(),
        r(32.0, 49.0, 118.0, 49.0),
        (36.0 * scale) as i32,
        true,
        white,
        false,
    )?;
    canvas.text(
        &model::hours(total),
        r(180.0, 49.0, 118.0, 49.0),
        (36.0 * scale) as i32,
        true,
        white,
        false,
    )?;
    canvas.text(
        "Days · 陪伴天数",
        r(33.0, 97.0, 132.0, 22.0),
        (11.0 * scale) as i32,
        false,
        muted,
        false,
    )?;
    canvas.text(
        "Hours · 累计工作",
        r(181.0, 97.0, 132.0, 22.0),
        (11.0 * scale) as i32,
        false,
        muted,
        false,
    )?;
    canvas.round(
        rect(164.0, 60.0, 1.0, 47.0),
        0.5 * scale,
        [255, 255, 255],
        24,
    );
    canvas.round(
        rect(30.0, 134.0, 268.0, 39.0),
        12.0 * scale,
        if running {
            [56, 121, 107]
        } else {
            [90, 126, 160]
        },
        if hover { 210 } else { 150 },
    );
    canvas.text(
        if running {
            "结束本次专注"
        } else {
            "开始专注"
        },
        r(36.0, 134.0, 256.0, 39.0),
        (14.0 * scale) as i32,
        true,
        white,
        true,
    )?;
    let seconds = elapsed / 1000;
    let caption = if running {
        format!(
            "本次 {:02}:{:02}:{:02}  ·  Ctrl + Alt + L",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        "Ctrl + Alt + L  开始 / 停止".into()
    };
    canvas.text(
        &caption,
        r(25.0, 180.0, 278.0, 20.0),
        (11.0 * scale) as i32,
        false,
        muted,
        true,
    )?;
    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panel_has_transparent_corners_and_translucent_body() {
        let image = panel(0, 0, 0, false, false, 96).unwrap();
        assert_eq!(image.pixels[0] >> 24, 0);
        let alpha = image.pixels[(125 * image.width + 150) as usize] >> 24;
        assert!(alpha > 150 && alpha < 255);
    }
}
