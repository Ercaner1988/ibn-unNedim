//! Ölçümler (Windows; başka yerde `None`).

use std::time::{SystemTime, UNIX_EPOCH};

pub fn simdi() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

#[cfg(windows)]
fn yerel_saat() -> windows_sys::Win32::Foundation::SYSTEMTIME {
    use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime};
    // SAFETY: SYSTEMTIME düz veri; sıfır geçerli, GetLocalTime hepsini yazar.
    let mut s: SYSTEMTIME = unsafe { std::mem::zeroed() };
    // SAFETY: `s` bu çerçevede yaşayan yerel yapı.
    unsafe { GetLocalTime(&mut s) };
    s
}

/// Yerel gece yarısından bu yana saniye.
pub fn gun_saniyesi() -> Option<u32> {
    #[cfg(windows)]
    {
        let s = yerel_saat();
        Some(u32::from(s.wHour) * 3600 + u32::from(s.wMinute) * 60 + u32::from(s.wSecond))
    }
    #[cfg(not(windows))]
    None
}

/// Yerel tarih YYYYAAGG (gün sayaçları için).
pub fn yerel_tarih() -> Option<u32> {
    #[cfg(windows)]
    {
        let s = yerel_saat();
        Some(u32::from(s.wYear) * 10_000 + u32::from(s.wMonth) * 100 + u32::from(s.wDay))
    }
    #[cfg(not(windows))]
    None
}
