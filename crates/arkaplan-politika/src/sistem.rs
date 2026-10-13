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

/// Sistem belleği yükü (%).
pub fn bellek_yuku() -> Option<u32> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        // SAFETY: düz veri; dwLength işlevin istediği gibi yazılır.
        let mut d: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        d.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        // SAFETY: `d` bu çerçevede yaşayan yerel yapı.
        (unsafe { GlobalMemoryStatusEx(&mut d) } != 0).then_some(d.dwMemoryLoad)
    }
    #[cfg(not(windows))]
    None
}

/// Son klavye/fare girdisinden bu yana saniye (oturum geneli).
pub fn girdisiz_sn() -> Option<u64> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetTickCount;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        let mut l = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: `l` boyutu yazılmış yerel yapı.
        if unsafe { GetLastInputInfo(&mut l) } == 0 {
            return None;
        }
        // SAFETY: argümansız.
        Some(u64::from(unsafe { GetTickCount() }.wrapping_sub(l.dwTime)) / 1000)
    }
    #[cfg(not(windows))]
    None
}

/// Kullanıcının CPU payı (%): toplam meşguliyetten arka plan süreçlerinin payı düşülür.
/// Birimler aynı olmalı (100 ns, çekirdekler toplamı).
pub fn kullanici_payi(toplam: u64, mesgul: u64, arka_plan: u64) -> Option<u32> {
    (toplam > 0).then(|| (mesgul.saturating_sub(arka_plan) * 100 / toplam).min(100) as u32)
}

#[cfg(windows)]
fn ft(f: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
}

/// (toplam, boşta) sistem zamanı; çekirdek zamanı boşta zamanı içerir.
#[cfg(windows)]
fn sistem_zamanlari() -> Option<(u64, u64)> {
    use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};
    // SAFETY: düz veri, GetSystemTimes yazar.
    let (mut b, mut c, mut k): (FILETIME, FILETIME, FILETIME) = unsafe { std::mem::zeroed() };
    // SAFETY: üç yerel yapıya işaretçi.
    (unsafe { GetSystemTimes(&mut b, &mut c, &mut k) } != 0).then(|| (ft(c) + ft(k), ft(b)))
}

/// PID → çekirdek+kullanıcı zamanı, `ARKA_PLAN_SURECLERI` için.
#[cfg(windows)]
fn arka_plan_zamanlari() -> std::collections::HashMap<u32, u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let mut sonuc = std::collections::HashMap::new();
    // SAFETY: anlık görüntü tutamacı aşağıda kapatılır.
    let anlik = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if anlik == INVALID_HANDLE_VALUE {
        return sonuc;
    }
    // SAFETY: düz veri; dwSize işlevin istediği gibi.
    let mut g: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    g.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: geçerli anlık görüntü ve boyutu yazılmış yapı.
    let mut var = unsafe { Process32FirstW(anlik, &mut g) } != 0;
    while var {
        let n = g
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(g.szExeFile.len());
        let ad = String::from_utf16_lossy(&g.szExeFile[..n]).to_ascii_lowercase();
        if crate::ARKA_PLAN_SURECLERI.contains(&ad.as_str()) {
            // SAFETY: yalnız sorgu hakkı; tutamaç aşağıda kapatılır.
            let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, g.th32ProcessID) };
            if !h.is_null() {
                // SAFETY: düz veri.
                let (mut a, mut b, mut c, mut k): (FILETIME, FILETIME, FILETIME, FILETIME) =
                    unsafe { std::mem::zeroed() };
                // SAFETY: geçerli tutamaç, yerel yapılar.
                if unsafe { GetProcessTimes(h, &mut a, &mut b, &mut c, &mut k) } != 0 {
                    sonuc.insert(g.th32ProcessID, ft(c) + ft(k));
                }
                // SAFETY: OpenProcess'in döndürdüğü tutamaç.
                unsafe { CloseHandle(h) };
            }
        }
        // SAFETY: aynı anlık görüntü.
        var = unsafe { Process32NextW(anlik, &mut g) } != 0;
    }
    // SAFETY: CreateToolhelp32Snapshot'ın tutamacı.
    unsafe { CloseHandle(anlik) };
    sonuc
}

/// Kararın ölçümleri; `aralik` boyunca uyur (CPU örneği). Çizim/async iş parçacığında çağrılmaz.
pub fn olc(aralik: std::time::Duration) -> crate::Olcum {
    #[cfg(windows)]
    let kullanici_cpu = {
        let (s0, a0) = (sistem_zamanlari(), arka_plan_zamanlari());
        std::thread::sleep(aralik);
        let (s1, a1) = (sistem_zamanlari(), arka_plan_zamanlari());
        match (s0, s1) {
            (Some((t0, b0)), Some((t1, b1))) => {
                let toplam = t1.saturating_sub(t0);
                let mesgul = toplam.saturating_sub(b1.saturating_sub(b0));
                // Aralıkta açılan süreç 0 sayılır (önceki örneği yok).
                let arka: u64 = a1
                    .iter()
                    .map(|(pid, &v)| v.saturating_sub(a0.get(pid).copied().unwrap_or(v)))
                    .sum();
                kullanici_payi(toplam, mesgul, arka)
            }
            _ => None,
        }
    };
    #[cfg(not(windows))]
    let kullanici_cpu = {
        std::thread::sleep(aralik);
        None
    };
    crate::Olcum {
        simdi: simdi(),
        gun_dakikasi: gun_saniyesi().map(|s| s / 60),
        bellek: bellek_yuku(),
        kullanici_cpu,
        girdisiz_sn: girdisiz_sn(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kullanici_payi_arka_plani_duser() {
        // 4 çekirdek × 1 sn = 40_000_000 (100 ns); 30M meşgul, bunun 20M'i arka plan → %25.
        assert_eq!(kullanici_payi(40_000_000, 30_000_000, 20_000_000), Some(25));
        assert_eq!(
            kullanici_payi(40_000_000, 10_000_000, 20_000_000),
            Some(0),
            "taşma yok"
        );
        assert_eq!(kullanici_payi(0, 0, 0), None);
    }

    #[cfg(windows)]
    #[test]
    fn olcum_dolu_gelir() {
        let o = olc(std::time::Duration::from_millis(200));
        assert!(o.gun_dakikasi.is_some_and(|d| d < 1440));
        assert!(o.bellek.is_some_and(|b| b <= 100));
        assert!(o.kullanici_cpu.is_some_and(|c| c <= 100));
        assert!(o.girdisiz_sn.is_some());
    }
}
