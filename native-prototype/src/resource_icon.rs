//! Extract embedded artwork without running the executable.
//! Adapted from the Windows icon reader in src-tauri/src/native_shortcuts.rs.
use image::RgbaImage;
use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path, ptr};
type Handle = *mut c_void;
#[repr(C)]
struct BitmapInfo {
    header_size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    image_size: u32,
    x_resolution: i32,
    y_resolution: i32,
    used_colors: u32,
    important_colors: u32,
    colors: [u32; 1],
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn ExtractIconExW(
        file: *const u16,
        index: i32,
        large: *mut Handle,
        small: *mut Handle,
        count: u32,
    ) -> u32;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn PrivateExtractIconsW(
        file: *const u16,
        index: i32,
        cx: i32,
        cy: i32,
        icons: *mut Handle,
        ids: *mut u32,
        count: u32,
        flags: u32,
    ) -> u32;
    fn DestroyIcon(icon: Handle) -> i32;
    fn DrawIconEx(
        dc: Handle,
        x: i32,
        y: i32,
        icon: Handle,
        cx: i32,
        cy: i32,
        step: u32,
        brush: Handle,
        flags: u32,
    ) -> i32;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: Handle) -> Handle;
    fn CreateDIBSection(
        dc: Handle,
        info: *const c_void,
        usage: u32,
        bits: *mut *mut c_void,
        section: Handle,
        offset: u32,
    ) -> Handle;
    fn SelectObject(dc: Handle, object: Handle) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
    fn DeleteDC(dc: Handle) -> i32;
    fn GdiFlush() -> i32;
}
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

pub fn read(path: &Path, index: i32, size: i32) -> Option<RgbaImage> {
    if !(-65535..=65535).contains(&index) {
        return None;
    }
    if !path.is_file() {
        return None;
    }
    let file = wide(path);
    if file[..file.len() - 1].contains(&0) {
        return None;
    }
    unsafe {
        let mut icon = ptr::null_mut();
        PrivateExtractIconsW(
            file.as_ptr(),
            index,
            size,
            size,
            &mut icon,
            ptr::null_mut(),
            1,
            0,
        );
        if icon.is_null() {
            ExtractIconExW(file.as_ptr(), index, &mut icon, ptr::null_mut(), 1);
        }
        if icon.is_null() {
            return None;
        }
        let result = pixels(icon, size);
        DestroyIcon(icon);
        result
    }
}
unsafe fn pixels(icon: Handle, size: i32) -> Option<RgbaImage> {
    let info = BitmapInfo {
        header_size: 40,
        width: size,
        height: -size,
        planes: 1,
        bit_count: 32,
        compression: 0,
        image_size: 0,
        x_resolution: 0,
        y_resolution: 0,
        used_colors: 0,
        important_colors: 0,
        colors: [0],
    };
    unsafe {
        let dc = CreateCompatibleDC(ptr::null_mut());
        if dc.is_null() {
            return None;
        }
        let mut bits: *mut c_void = ptr::null_mut();
        let bitmap = CreateDIBSection(
            dc,
            (&info as *const BitmapInfo).cast(),
            0,
            &mut bits,
            ptr::null_mut(),
            0,
        );
        if bitmap.is_null() || bits.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            DeleteDC(dc);
            return None;
        }
        let old = SelectObject(dc, bitmap);
        let out = if old.is_null() || old as isize == -1 {
            None
        } else {
            // Recover straight-alpha RGBA by measuring compositing against black
            // and white. This also handles older icons with a binary AND mask.
            let data =
                std::slice::from_raw_parts_mut(bits.cast::<u8>(), (size * size * 4) as usize);
            data.fill(0);
            let black_ok = DrawIconEx(dc, 0, 0, icon, size, size, 0, ptr::null_mut(), 3) != 0;
            GdiFlush();
            let black = data.to_vec();
            data.fill(255);
            let white_ok = DrawIconEx(dc, 0, 0, icon, size, size, 0, ptr::null_mut(), 3) != 0;
            GdiFlush();
            let result = if black_ok && white_ok {
                let mut rgba = Vec::with_capacity(data.len());
                for (b, w) in black.chunks_exact(4).zip(data.chunks_exact(4)) {
                    let clear = (0..3)
                        .map(|i| w[i].saturating_sub(b[i]) as u16)
                        .sum::<u16>()
                        / 3;
                    let alpha = 255 - clear;
                    let channel = |i: usize| {
                        if alpha == 0 {
                            0
                        } else {
                            ((u16::from(b[i]) * 255 + alpha / 2) / alpha).min(255) as u8
                        }
                    };
                    rgba.extend([channel(2), channel(1), channel(0), alpha as u8]);
                }
                if rgba.iter().skip(3).step_by(4).all(|&a| a == 0) {
                    None
                } else {
                    RgbaImage::from_raw(size as u32, size as u32, rgba)
                }
            } else {
                None
            };
            SelectObject(dc, old);
            result
        };
        DeleteObject(bitmap);
        DeleteDC(dc);
        out
    }
}
