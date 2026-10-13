# Arka plan sekmesi — uygulama planı

> **Ajanlar için:** GEREKLİ ALT BECERİ: superpowers:subagent-driven-development (önerilen) ya da superpowers:executing-plans. Adımlar `- [ ]` onay kutusuyla izlenir.

**Amaç:** ibnü'n-Nedim penceresine "Arka plan" sekmesi eklemek. Toplu gömme ve Nazar OCR'ı tek bir politika dosyasından yönetilecek: elle duraklatma (süre seçmeli), saat aralığı, bellek ve CPU sınırı, "kullanırken dur".

**Mimari:**
- **Ortak sandık:** ibnü'n-Nedim reposunda yeni `arkaplan-politika` sandığı. Dosya biçimi, okuma/yazma, karar ve Windows ölçümleri burada.
- **ibnü'n-Nedim tarafı:** Toplu istekler `TOPLU_KAPI`'dan önce bu karara sorulur. Yeni `bge-arkaplan-sekme` sandığı politikayı yazar, Nazar'ın durum dosyasını okur.
- **Nazar tarafı:** Ortak sandığa git rev ile bağlanır. Yeni `nazar-arkaplan` sandığı Nazar'ın kendi koşullarını ekler ve durum dosyasını yazar.

**Teknoloji:** Rust (edition 2024 ibnü'n-Nedim, 2021 Nazar), egui/eframe 0.36.2, kilim-tema `b62fcac`, toml 0.8, serde_json, windows-sys 0.61.

**Tasarım belgesi:** `docs/superpowers/specs/2026-10-12-arka-plan-sekmesi-design.md` (bu repo). Uygulayıcı planı ve belgeyi birlikte okur.

## Genel kısıtlar

- Sandık başına en çok 1500 KOD satırı (`src/`; boş satır, yorum, `#[cfg(test)]` sayılmaz). `nazar-daemon` 1497'de: ona eklenen her satır başka yerden çıkarılır (Görev 9).
- **egui:**
  - Renderer WGPU, Vulkan önce (yapıldı, 0476b1d).
  - Kare işlevinde koşulsuz `request_repaint()` yok; zamanlı yeniden çizim en az 500 ms.
  - Çizim iş parçacığında dosya okuma/yazma, `recv`, `sleep`, `block_on` yok: yalnız `try_recv()`.
- **Taşmasız yerleşim:**
  - Açılışta `kilim_tema::sarma_kur(ctx)`.
  - Formda `Grid` yok, `kilim_tema::alan_satiri` kullanılır.
  - Sarmalı satırda kendi kabını açan denetim `kilim_tema::birim` içinde.
- **İkili depolama:** kural dışı. Durum dosyası dışa açık biçim olduğu için JSON. Politika TOML.
- **Türkçe:** İçeride UTF-8. `.ps1` betiklerinin çalışan satırları ASCII (PowerShell 5.1 BOM'suz dosyada UTF-8'i bozar).
- **Commit ve push:** Ercan'ın kalıcı izni var. Her commit iletisi `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` ile biter.
- **Başka oturumun dosyası:** Nazar'daki `crates/nazar-cozum-pdf/src/ocr_kayma.rs` asla stage edilmez.
- **Kapı çıktısı kesilmez:** `kapi .` çıktısı dosyaya alınır, KIRMIZI oradan aranır.
- **Kurulum:**
  - ibnü'n-Nedim `./kur.ps1 -DerlemeYok -Durdur` ile (PowerShell 7) kurulur.
  - nazard için çalışan exe'nin adı değiştirilir, yenisi kopyalanır, eski süreç PID ile durdurulur, `nazard.exe arkaplan` başlatılır, günlükte "boru dinleniyor" beklenir.
- **Pencere doğrulaması:** Yalnız kendi penceremizin PrintWindow görüntüsüyle yapılır. Süreç ayakta diye "çalışıyor" denmez.

### Tasarım belgesinden bilinçli sapmalar (Görev 1'de belgeye işlenir)

1. **`duraklat_bitis` türü:** i64 ve "ben sürdürene dek" = `i64::MAX`, çünkü TOML tamsayısı i64'tür; `u64::MAX` yazılamaz.
2. **Durum simgeleri:** ⏸ ◐ ● ○ yerine renkli nokta (`theme::status_dot_row`) kullanılır. Bu simgeler gömülü yazı tiplerinde olmayabilir. Bitiş "bitiş 15:40" diye yazılır, çünkü "'a dek" eki saate göre değişir.
3. **İlk kurulum:** Politika dosyası `kullanirken_dur = false` ile kurulur (bugünkü davranış). Kullanıcı sekmeden açar.

## İnceleme odağı

Belgenin ima ettiği ama hiçbir görevin olağan sınamasının değmediği beş durum. Her biri sahibi olan göreve sınama olarak eklendi:

1. **Elle bozulan `arkaplan.toml`:** İki süreç son geçerli politikayla sürer, sekme kırmızı not gösterir, dosyanın üstüne kendiliğinden yazılmaz. Sınama: Görev 2, `bozuk_dosyada_son_gecerli_kalir`.
2. **Başı ile sonu aynı saat aralığı ("06:30-06:30"):** Sessizce "hiç çalışma" olmamalı, açık hata olmalı. Sınama: Görev 1, `ayni_bas_son_reddedilir`.
3. **"Ben sürdürene dek" duraklatması:** TOML'a yazılıp okunduğunda bozulmamalı; "bugün boyunca" yerel gece yarısında bitmeli. Sınama: Görev 1, `surdurene_dek_gidis_donus` ve `bugun_boyunca_gece_yarisi`.
4. **Dosya yokken ibnü'n-Nedim:** Toplu iş hiç beklememeli ve ölçüm yapılmamalı (bugünkü davranış). Sınama: Görev 2, `dosya_yokken_izin_olcmeden`.
5. **nazard kapalı ya da donmuşken:** Durum dosyası 60 sn'den eskiyse sekme OCR satırında "nazard yanıt vermiyor" der, yine de düğmeler çalışır. Sınama: Görev 5, `bayat_nazar_durumu`.

## Dosya yapısı

**ibnü'n-Nedim (`C:\Users\buzbe\Desktop\Github\bge-embed-rs`):**
- Yeni `crates/arkaplan-politika/`:
  - `Cargo.toml`
  - `src/lib.rs`: tipler ve TOML.
  - `src/karar.rs`: saat aralığı, karar, duraklatma ve `Kapi`.
  - `src/dosya.rs`: yollar, atomik yazma, `Okuyucu`, durum JSON.
  - `src/sistem.rs`: Windows ölçümleri.
  - `tests/sozlesme.rs`
- Yeni `crates/bge-arkaplan-sekme/`:
  - `Cargo.toml`
  - `src/lib.rs`: `Sekme`, işçi ve render.
  - `src/mantik.rs`: arayüzsüz mantık.
- Değişenler:
  - `Cargo.toml` (kök): workspace bağımlılıkları; kilim-tema ve fihrist-tema rev'leri.
  - `crates/bge-clients/`: `Cargo.toml`, `src/lib.rs`, `src/is.rs`, yeni `src/politika.rs`.
  - `src/main.rs`: gom() içinde politika kapısı.
  - `crates/bge-gui/`: `Cargo.toml`, `src/lib.rs` (Screen, sekmeler, Türkçe adlar, 500 ms, stage_label).
  - `crates/bge-theme/src/lib.rs`: `apply` içine `sarma_kur`.
  - `docs/superpowers/specs/2026-10-12-arka-plan-sekmesi-design.md`: sapmalar.
- Repo dışı: `~/.llama-embed-server/ac.ps1`.

**el-Fihrist (`C:\Users\buzbe\Desktop\Github\rust\el-Fihrist`):**
- Kök `Cargo.toml`: kilim-tema rev, böylece fihrist-tema aynı kilim'i çeker.

**Nazar (`C:\Users\buzbe\Desktop\Github\rust\Nazar`):**
- Kök `Cargo.toml`: üye ve `arkaplan-politika` git bağımlılığı.
- Yeni `crates/nazar-arkaplan/`:
  - `Cargo.toml`
  - `src/lib.rs`
  - `tests/sozlesme.rs`
- Değişenler:
  - `crates/nazar-daemon/`: `Cargo.toml`, `src/motor.rs`, `src/lib.rs` (sınama ayarı).
  - `crates/nazar-dis/src/ayar.rs`: `ocr_saatleri` kalkar, `politika_dosyasi` gelir.
  - `crates/nazar-dis/src/sistem.rs`: kullanılmayan `yerel_dakika` ve `cpu_yuku` kalkar.

---

### Görev 1: `arkaplan-politika`: tipler, TOML, saat aralığı, karar

**Dosyalar:**
- Oluştur: `crates/arkaplan-politika/Cargo.toml`, `src/lib.rs`, `src/karar.rs`, `src/dosya.rs` (Görev 2 doldurur; şimdilik boş modül), `src/sistem.rs` (Görev 3 doldurur; şimdilik `simdi` ve iki gün işlevi).
- Değiştir: kök `Cargo.toml`'da `[workspace.dependencies]` içine `arkaplan-politika = { path = "crates/arkaplan-politika" }`. Üyeler `crates/*` olduğu için sandık kendiliğinden girer.
- Değiştir: tasarım belgesi (yukarıdaki üç sapma).

**Arayüzler:**
- Üretir:
  - `Politika { surum: u32, sinirlar: Sinirlar, gomme: IsAyari, ocr: IsAyari }`
  - `Politika::ayristir(&str) -> Result<Politika, String>`
  - `Politika::metin(&self) -> String`
  - `Politika::is(&self, IsTuru) -> &IsAyari`
  - `Politika::is_mut(&mut self, IsTuru) -> &mut IsAyari`
  - `Sinirlar { bellek_yuzde: u32, cpu_yuzde: u32, kullanirken_dur: bool }`
  - `IsAyari { saatler: String, duraklat_bitis: i64 }`
  - `IsTuru { Gomme, Ocr }`
  - `SaatAraligi { HerZaman, Aralik { bas: u32, son: u32 } }`, `::ayristir(&str) -> Result<_, String>`, `.icinde(u32) -> bool`, `.metin() -> String`
  - `Olcum { simdi: i64, gun_dakikasi: Option<u32>, bellek: Option<u32>, kullanici_cpu: Option<u32>, girdisiz_sn: Option<u64> }`
  - `Neden { Duraklatildi { bitis: i64 }, SaatDisi, Bellek(u32), Cpu(u32), KullaniciEtkin }` (Display Türkçe)
  - `karar(&Politika, IsTuru, &Olcum, bellek_onceki_dar: bool) -> Result<(), Neden>`
  - `bellek_dar_mi(esik: u32, yuk: u32, onceki: bool) -> bool`
  - `Duraklatma { BirSaat, BugunBoyunca, SurdureneDek }`
  - `duraklat_bitisi(Duraklatma, simdi: i64, gun_saniyesi: Option<u32>) -> i64`
  - Sabitler: `SURUM`, `SURDURENE_DEK = i64::MAX`, `HER_ZAMAN = "her zaman"`, `ARKA_PLAN_SURECLERI`
  - `sistem::simdi() -> i64`, `sistem::gun_saniyesi() -> Option<u32>`, `sistem::yerel_tarih() -> Option<u32>`

- [ ] **Adım 1: Sandık iskeleti**

`crates/arkaplan-politika/Cargo.toml`:
```toml
[package]
name = "arkaplan-politika"
version = "0.1.0"
edition.workspace = true
license.workspace = true
description = "Arka plan işleri (toplu gömme, OCR) için ortak politika: arkaplan.toml, karar, ölçüm; ibnü'n-Nedim ve Nazar paylaşır"
repository.workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = ["Win32_Foundation", "Win32_System_SystemInformation", "Win32_System_Threading", "Win32_System_Diagnostics_ToolHelp", "Win32_UI_Input_KeyboardAndMouse"] }
```

`src/lib.rs`:
```rust
//! Arka plan işleri için ortak politika (ibnü'n-Nedim + Nazar): `arkaplan.toml` biçimi,
//! okuma/yazma, "şimdi çalışabilir mi?" kararı ve ölçümler. Tasarım:
//! docs/superpowers/specs/2026-10-12-arka-plan-sekmesi-design.md

mod dosya;
mod karar;
pub mod sistem;

pub use dosya::*;
pub use karar::*;

use serde::{Deserialize, Serialize};

pub const SURUM: u32 = 1;
/// `duraklat_bitis` için "ben sürdürene dek" (TOML tamsayısı i64'tür; u64::MAX yazılamaz).
pub const SURDURENE_DEK: i64 = i64::MAX;
pub const HER_ZAMAN: &str = "her zaman";
/// CPU sınırından düşülen arka plan süreçleri (küçük harf).
pub const ARKA_PLAN_SURECLERI: &[&str] = &["ibnun-nedim.exe", "nazard.exe", "tesseract.exe"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Politika {
    pub surum: u32,
    #[serde(default)]
    pub sinirlar: Sinirlar,
    #[serde(default)]
    pub gomme: IsAyari,
    #[serde(default)]
    pub ocr: IsAyari,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sinirlar {
    pub bellek_yuzde: u32,
    pub cpu_yuzde: u32,
    pub kullanirken_dur: bool,
}

impl Default for Sinirlar {
    fn default() -> Self {
        Self { bellek_yuzde: 90, cpu_yuzde: 50, kullanirken_dur: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IsAyari {
    pub saatler: String,
    pub duraklat_bitis: i64,
}

impl Default for IsAyari {
    fn default() -> Self {
        Self { saatler: HER_ZAMAN.into(), duraklat_bitis: 0 }
    }
}

/// Bugünkü Nazar davranışı: bellek %90, CPU %50, saat kısıtı yok, duraklatma yok.
impl Default for Politika {
    fn default() -> Self {
        Self { surum: SURUM, sinirlar: Sinirlar::default(), gomme: IsAyari::default(), ocr: IsAyari::default() }
    }
}

impl Politika {
    pub fn is(&self, t: IsTuru) -> &IsAyari {
        match t {
            IsTuru::Gomme => &self.gomme,
            IsTuru::Ocr => &self.ocr,
        }
    }

    pub fn is_mut(&mut self, t: IsTuru) -> &mut IsAyari {
        match t {
            IsTuru::Gomme => &mut self.gomme,
            IsTuru::Ocr => &mut self.ocr,
        }
    }

    /// TOML'dan okur ve doğrular: bilinmeyen sürüm, geçersiz saat aralığı, 1-100 dışı yüzde hata.
    pub fn ayristir(metin: &str) -> Result<Self, String> {
        let p: Politika = toml::from_str(metin).map_err(|e| e.to_string())?;
        if p.surum != SURUM {
            return Err(format!("bilinmeyen surum {} (beklenen {SURUM})", p.surum));
        }
        for (ad, is) in [("gomme", &p.gomme), ("ocr", &p.ocr)] {
            SaatAraligi::ayristir(&is.saatler).map_err(|e| format!("[{ad}] saatler: {e}"))?;
        }
        let s = &p.sinirlar;
        if !(1..=100).contains(&s.bellek_yuzde) || !(1..=100).contains(&s.cpu_yuzde) {
            return Err("sinirlar: yüzde 1-100 dışında".into());
        }
        Ok(p)
    }

    pub fn metin(&self) -> String {
        toml::to_string(self).expect("Politika her zaman TOML'a yazılır")
    }
}
```

`src/dosya.rs` şimdilik yalnız bir yorum satırı içerir: `//! Görev 2.`

`src/sistem.rs` (Görev 3 genişletir):
```rust
//! Ölçümler (Windows; başka yerde `None`).

use std::time::{SystemTime, UNIX_EPOCH};

pub fn simdi() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
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
```

- [ ] **Adım 2: Başarısız sınamaları yaz** (`src/karar.rs`'in sonuna `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Politika, SURDURENE_DEK};

    fn olcum() -> Olcum {
        Olcum { simdi: 1_000, gun_dakikasi: Some(12 * 60), bellek: Some(50), kullanici_cpu: Some(10), girdisiz_sn: Some(600) }
    }

    #[test]
    fn saat_araligi_ayristirma() {
        assert_eq!(SaatAraligi::ayristir("her zaman"), Ok(SaatAraligi::HerZaman));
        assert_eq!(SaatAraligi::ayristir("00:00-06:30"), Ok(SaatAraligi::Aralik { bas: 0, son: 390 }));
        assert!(SaatAraligi::ayristir("gece").is_err());
        assert!(SaatAraligi::ayristir("24:00-06:00").is_err());
        assert_eq!(SaatAraligi::Aralik { bas: 0, son: 390 }.metin(), "00:00-06:30");
    }

    #[test]
    fn ayni_bas_son_reddedilir() {
        assert!(SaatAraligi::ayristir("06:30-06:30").is_err(), "hiç çalışmayan aralık sessizce kabul edilmez");
    }

    #[test]
    fn gece_yarisini_asan_aralik() {
        let a = SaatAraligi::ayristir("22:00-06:30").unwrap();
        assert!(a.icinde(23 * 60) && a.icinde(60) && !a.icinde(12 * 60) && !a.icinde(390));
    }

    #[test]
    fn her_kosul_tek_basina_keser() {
        let p = Politika::default();
        assert_eq!(karar(&p, IsTuru::Gomme, &olcum(), false), Ok(()));

        let mut d = p.clone();
        d.gomme.duraklat_bitis = 2_000;
        assert_eq!(karar(&d, IsTuru::Gomme, &olcum(), false), Err(Neden::Duraklatildi { bitis: 2_000 }));
        assert_eq!(karar(&d, IsTuru::Ocr, &olcum(), false), Ok(()), "duraklatma işe özgü");

        let mut s = p.clone();
        s.ocr.saatler = "00:00-06:30".into();
        assert_eq!(karar(&s, IsTuru::Ocr, &olcum(), false), Err(Neden::SaatDisi));

        let o = Olcum { bellek: Some(93), ..olcum() };
        assert_eq!(karar(&p, IsTuru::Gomme, &o, false), Err(Neden::Bellek(93)));

        let o = Olcum { kullanici_cpu: Some(71), ..olcum() };
        assert_eq!(karar(&p, IsTuru::Gomme, &o, false), Err(Neden::Cpu(71)));

        let mut k = p.clone();
        k.sinirlar.kullanirken_dur = true;
        let o = Olcum { girdisiz_sn: Some(5), ..olcum() };
        assert_eq!(karar(&k, IsTuru::Gomme, &o, false), Err(Neden::KullaniciEtkin));
        assert_eq!(karar(&p, IsTuru::Gomme, &o, false), Ok(()), "kapalıyken etkinlik engel değil");
    }

    #[test]
    fn olculemeyen_kosul_engel_degil() {
        let mut p = Politika::default();
        p.ocr.saatler = "00:00-06:30".into();
        p.sinirlar.kullanirken_dur = true;
        let o = Olcum { simdi: 1_000, gun_dakikasi: None, bellek: None, kullanici_cpu: None, girdisiz_sn: None };
        assert_eq!(karar(&p, IsTuru::Ocr, &o, false), Ok(()));
    }

    #[test]
    fn bellek_esikli() {
        assert!(bellek_dar_mi(90, 90, false));
        assert!(!bellek_dar_mi(90, 87, false));
        assert!(bellek_dar_mi(90, 87, true), "%85'in altına inene dek dar kalır");
        assert!(!bellek_dar_mi(90, 84, true));
    }

    #[test]
    fn neden_metinleri() {
        assert_eq!(Neden::Bellek(93).to_string(), "bellek %93");
        assert_eq!(Neden::SaatDisi.to_string(), "saat dışı");
        assert_eq!(Neden::Cpu(71).to_string(), "işlemci %71");
    }

    #[test]
    fn bugun_boyunca_gece_yarisi() {
        // 23:00'te (82800 sn) "bugün boyunca" → 3600 sn sonra.
        assert_eq!(duraklat_bitisi(Duraklatma::BugunBoyunca, 10_000, Some(82_800)), 13_600);
        assert_eq!(duraklat_bitisi(Duraklatma::BirSaat, 10_000, None), 13_600);
        assert_eq!(duraklat_bitisi(Duraklatma::SurdureneDek, 10_000, None), SURDURENE_DEK);
    }

    #[test]
    fn surdurene_dek_gidis_donus() {
        let mut p = Politika::default();
        p.ocr.duraklat_bitis = SURDURENE_DEK;
        p.gomme.saatler = "22:00-06:30".into();
        let q = Politika::ayristir(&p.metin()).unwrap();
        assert_eq!(q, p);
    }

    #[test]
    fn bozuk_politika_aciklayici_hata() {
        assert!(Politika::ayristir("surum = 2").unwrap_err().contains("surum"));
        assert!(Politika::ayristir("surum = 1\n[ocr]\nsaatler = \"gece\"").unwrap_err().contains("[ocr]"));
        assert!(Politika::ayristir("surum = 1\n[sinirlar]\ncpu_yuzde = 0").is_err());
        assert!(Politika::ayristir("bu toml değil").is_err());
    }
}
```

- [ ] **Adım 3: Sınamaların başarısız olduğunu gör**

Çalıştır: `cd C:\Users\buzbe\Desktop\Github\bge-embed-rs; cargo test -p arkaplan-politika`
Beklenen: derleme hatası (`karar`, `SaatAraligi`, … tanımsız).

- [ ] **Adım 4: `src/karar.rs`'i yaz** (sınama modülünün üstüne)

```rust
//! "Bu iş şimdi çalışabilir mi?" kararı: duraklatma → saat → bellek → CPU → kullanıcı etkinliği.
//! Ölçülemeyen koşul (None) engel sayılmaz.

use crate::{Politika, HER_ZAMAN, SURDURENE_DEK};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsTuru {
    Gomme,
    Ocr,
}

/// Gün içi dakikalarla saat aralığı; `bas > son` gece yarısını aşar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaatAraligi {
    HerZaman,
    Aralik { bas: u32, son: u32 },
}

impl SaatAraligi {
    pub fn ayristir(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s == HER_ZAMAN {
            return Ok(Self::HerZaman);
        }
        let hata = || format!("\"{s}\" ne \"{HER_ZAMAN}\" ne \"SS:DD-SS:DD\"");
        let dakika = |t: &str| -> Option<u32> {
            let (h, m) = t.trim().split_once(':')?;
            let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
            (h < 24 && m < 60).then_some(h * 60 + m)
        };
        let (a, b) = s.split_once('-').ok_or_else(hata)?;
        match (dakika(a), dakika(b)) {
            (Some(bas), Some(son)) if bas != son => Ok(Self::Aralik { bas, son }),
            (Some(_), Some(_)) => Err(format!("\"{s}\": başlangıç ile bitiş aynı")),
            _ => Err(hata()),
        }
    }

    pub fn icinde(self, gun_dakikasi: u32) -> bool {
        match self {
            Self::HerZaman => true,
            Self::Aralik { bas, son } if bas < son => (bas..son).contains(&gun_dakikasi),
            Self::Aralik { bas, son } => gun_dakikasi >= bas || gun_dakikasi < son,
        }
    }

    pub fn metin(self) -> String {
        match self {
            Self::HerZaman => HER_ZAMAN.into(),
            Self::Aralik { bas, son } => {
                format!("{:02}:{:02}-{:02}:{:02}", bas / 60, bas % 60, son / 60, son % 60)
            }
        }
    }
}

/// Kararın girdisi; `sistem::olc` doldurur, sınamalar elle kurar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Olcum {
    pub simdi: i64,
    pub gun_dakikasi: Option<u32>,
    pub bellek: Option<u32>,
    /// Arka plan süreçleri düşülmüş CPU yükü (%).
    pub kullanici_cpu: Option<u32>,
    /// Son klavye/fare girdisinden bu yana saniye.
    pub girdisiz_sn: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Neden {
    Duraklatildi { bitis: i64 },
    SaatDisi,
    Bellek(u32),
    Cpu(u32),
    KullaniciEtkin,
}

impl fmt::Display for Neden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Neden::Duraklatildi { .. } => write!(f, "elle duraklatıldı"),
            Neden::SaatDisi => write!(f, "saat dışı"),
            Neden::Bellek(y) => write!(f, "bellek %{y}"),
            Neden::Cpu(y) => write!(f, "işlemci %{y}"),
            Neden::KullaniciEtkin => write!(f, "kullanıcı etkin"),
        }
    }
}

/// "Kullanırken dur": son girdi bundan yeniyse kullanıcı etkin sayılır.
pub const KULLANIM_PENCERESI_SN: u64 = 60;
/// Bellek eşiği bandı: dar hâl `esik - BELLEK_BANDI`'nin altına inince kalkar (salınım önler).
pub const BELLEK_BANDI: u32 = 5;

pub fn bellek_dar_mi(esik: u32, yuk: u32, onceki: bool) -> bool {
    yuk >= esik || (onceki && yuk + BELLEK_BANDI >= esik)
}

pub fn karar(p: &Politika, is: IsTuru, o: &Olcum, bellek_onceki_dar: bool) -> Result<(), Neden> {
    let a = p.is(is);
    if a.duraklat_bitis > o.simdi {
        return Err(Neden::Duraklatildi { bitis: a.duraklat_bitis });
    }
    let saat = SaatAraligi::ayristir(&a.saatler).unwrap_or(SaatAraligi::HerZaman);
    if o.gun_dakikasi.is_some_and(|d| !saat.icinde(d)) {
        return Err(Neden::SaatDisi);
    }
    let s = &p.sinirlar;
    if let Some(y) = o.bellek.filter(|&y| bellek_dar_mi(s.bellek_yuzde, y, bellek_onceki_dar)) {
        return Err(Neden::Bellek(y));
    }
    if let Some(y) = o.kullanici_cpu.filter(|&y| y >= s.cpu_yuzde) {
        return Err(Neden::Cpu(y));
    }
    if s.kullanirken_dur && o.girdisiz_sn.is_some_and(|g| g < KULLANIM_PENCERESI_SN) {
        return Err(Neden::KullaniciEtkin);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duraklatma {
    BirSaat,
    BugunBoyunca,
    SurdureneDek,
}

/// `gun_saniyesi`: yerel gece yarısından bu yana saniye; bilinmiyorsa "bugün boyunca" 24 saattir.
pub fn duraklat_bitisi(d: Duraklatma, simdi: i64, gun_saniyesi: Option<u32>) -> i64 {
    match d {
        Duraklatma::BirSaat => simdi + 3_600,
        Duraklatma::BugunBoyunca => simdi + 86_400 - i64::from(gun_saniyesi.unwrap_or(0)),
        Duraklatma::SurdureneDek => SURDURENE_DEK,
    }
}
```

- [ ] **Adım 5: Sınamaların geçtiğini gör**

Çalıştır: `cargo test -p arkaplan-politika`
Beklenen: 10 sınama geçer.

- [ ] **Adım 6: Tasarım belgesine sapmaları işle**

`docs/superpowers/specs/2026-10-12-arka-plan-sekmesi-design.md` üç yerde değişir:
- §1.1'deki `duraklat_bitis` yorumu: `# unix sn; 0 = çalışıyor, i64::MAX = "ben sürdürene dek" (TOML tamsayısı i64)`.
- §2.2'deki "durum ve nedeni" maddesi: `renkli nokta + metin ("çalışıyor", "saat dışı", "duraklatıldı · bitiş 15:40"); simge karakteri yok (gömülü yazı tipinde olmayabilir)`.
- §3.2'deki kurulum maddesine: `kullanirken_dur = false (bugünkü davranış; sekmeden açılır)`.

- [ ] **Adım 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/arkaplan-politika docs/superpowers/specs/2026-10-12-arka-plan-sekmesi-design.md
git commit -m "arkaplan-politika: politika tipleri, TOML, saat aralığı ve karar

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Görev 2: `arkaplan-politika`: dosyalar, `Okuyucu`, durum JSON, `Kapi`

**Dosyalar:**
- Değiştir: `crates/arkaplan-politika/src/dosya.rs`, `src/karar.rs` (`Kapi` eklenir).
- Oluştur: `crates/arkaplan-politika/tests/sozlesme.rs`

**Arayüzler:**
- Tüketir: Görev 1'deki her şey. `sistem::olc` Görev 3'te gelir; `Kapi::dene` onu çağırır. Bu görevde `sistem.rs`'e geçici `pub fn olc(_: Duration) -> Olcum { Olcum { simdi: simdi(), ..Default::default() } }` eklenir, Görev 3 bunu gerçeğiyle değiştirir.
- Üretir:
  - `politika_yolu() -> PathBuf` (`%APPDATA%\ibnun-nedim\arkaplan.toml`)
  - `durum_yolu() -> PathBuf` (`%LOCALAPPDATA%\nazar\arkaplan-durum.json`)
  - `yaz(&Path, &Politika) -> io::Result<()>` (atomik)
  - `Okuma { Yok, Gecerli(Politika), Bozuk { hata: String, son_gecerli: Option<Politika> } }`
  - `Okuyucu::yeni(PathBuf)`, `.yol() -> &Path`, `.oku() -> Okuma`, `.etkin() -> Option<Politika>`
  - `IsDurumu { calisiyor: bool, neden: Option<String>, bugun: u64 }`
  - `ArkaplanDurumu { zaman: i64, parca_vektoru: (usize, usize), ocr_kuyrugu: usize, gomme: IsDurumu, ocr: IsDurumu }`
  - `durum_yaz(&Path, &ArkaplanDurumu) -> io::Result<()>`, `durum_oku(&Path) -> Option<ArkaplanDurumu>`
  - `Kapi::yeni(PathBuf)`, `.dene(IsTuru, varsayilan: Option<&Politika>) -> Result<(), Neden>`, `.son_neden() -> Option<Neden>`, `.okuyucu() -> &Okuyucu`

- [ ] **Adım 1: Başarısız sınamaları yaz**

`src/dosya.rs` sonuna:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn gecici(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("arkaplan-politika-{ad}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d.join("arkaplan.toml")
    }

    /// Ardışık yazışlar aynı değişme tarihine düşmesin (okuyucu tarihe bakar).
    fn yaz_ileri(yol: &Path, metin: &str, ileri_sn: u64) {
        fs::write(yol, metin).unwrap();
        let f = fs::OpenOptions::new().write(true).open(yol).unwrap();
        f.set_modified(SystemTime::now() + Duration::from_secs(ileri_sn)).unwrap();
    }

    #[test]
    fn yazilan_okunur_ve_atomik() {
        let yol = gecici("yaz");
        let mut p = Politika::default();
        p.ocr.saatler = "00:00-06:30".into();
        yaz(&yol, &p).unwrap();
        assert_eq!(Okuyucu::yeni(yol.clone()).oku(), Okuma::Gecerli(p));
        let artik: Vec<_> = fs::read_dir(yol.parent().unwrap()).unwrap().flatten()
            .filter(|g| g.file_name().to_string_lossy().contains("yaziliyor")).collect();
        assert!(artik.is_empty(), "geçici dosya kalmaz");
    }

    #[test]
    fn dosya_yoksa_yok() {
        let o = Okuyucu::yeni(gecici("yok"));
        assert_eq!(o.oku(), Okuma::Yok);
        assert_eq!(o.etkin(), None);
    }

    #[test]
    fn bozuk_dosyada_son_gecerli_kalir() {
        let yol = gecici("bozuk");
        let o = Okuyucu::yeni(yol.clone());
        let p = Politika::default();
        yaz_ileri(&yol, &p.metin(), 1);
        assert_eq!(o.oku(), Okuma::Gecerli(p.clone()));
        yaz_ileri(&yol, "surum = 1\n[ocr]\nsaatler = \"gece\"\n", 2);
        match o.oku() {
            Okuma::Bozuk { hata, son_gecerli } => {
                assert!(hata.contains("[ocr]"), "{hata}");
                assert_eq!(son_gecerli, Some(p.clone()));
            }
            d => panic!("bozuk beklenirdi: {d:?}"),
        }
        assert_eq!(o.etkin(), Some(p), "son geçerli politika sürer");
        assert!(fs::read_to_string(&yol).unwrap().contains("gece"), "dosyanın üstüne yazılmaz");
    }

    #[test]
    fn durum_gidis_donus() {
        let yol = gecici("durum").with_file_name("arkaplan-durum.json");
        assert_eq!(durum_oku(&yol), None);
        let d = ArkaplanDurumu {
            zaman: 5,
            parca_vektoru: (1, 2),
            ocr_kuyrugu: 3,
            gomme: IsDurumu { calisiyor: true, neden: None, bugun: 7 },
            ocr: IsDurumu { calisiyor: false, neden: Some("saat dışı".into()), bugun: 0 },
        };
        durum_yaz(&yol, &d).unwrap();
        assert_eq!(durum_oku(&yol), Some(d));
    }
}
```

`src/karar.rs`'in sınama modülüne ekle:
```rust
    #[test]
    fn dosya_yokken_izin_olcmeden() {
        let k = Kapi::yeni(std::env::temp_dir().join("arkaplan-politika-hic-yok.toml"));
        let bas = std::time::Instant::now();
        assert_eq!(k.dene(IsTuru::Gomme, None), Ok(()));
        assert!(bas.elapsed() < std::time::Duration::from_millis(200), "ölçüm (1 sn) yapılmamalı");
        assert_eq!(k.son_neden(), None);
    }

    #[test]
    fn duraklatilmis_dosyada_kapi_keser() {
        let yol = std::env::temp_dir().join(format!("arkaplan-politika-kapi-{}.toml", std::process::id()));
        let mut p = Politika::default();
        p.gomme.duraklat_bitis = crate::SURDURENE_DEK;
        crate::yaz(&yol, &p).unwrap();
        let k = Kapi::yeni(yol.clone());
        assert_eq!(k.dene(IsTuru::Gomme, None), Err(Neden::Duraklatildi { bitis: crate::SURDURENE_DEK }));
        assert_eq!(k.son_neden(), Some(Neden::Duraklatildi { bitis: crate::SURDURENE_DEK }));
        let _ = std::fs::remove_file(yol);
    }
```

`tests/sozlesme.rs` (Nazar'da aynısı var, Görev 8):
```rust
//! İki repo arası sözleşme: bu metinler Nazar'ın `crates/nazar-arkaplan/tests/sozlesme.rs`'inde
//! de AYNEN durur. Biri değişirse öteki de değişmeli.
use arkaplan_politika::{ArkaplanDurumu, Politika};

const ORNEK: &str = r#"surum = 1
[sinirlar]
bellek_yuzde = 90
cpu_yuzde = 50
kullanirken_dur = true
[gomme]
saatler = "her zaman"
duraklat_bitis = 0
[ocr]
saatler = "00:00-06:30"
duraklat_bitis = 0
"#;

const DURUM: &str = r#"{"zaman":1,"parca_vektoru":[2,3],"ocr_kuyrugu":4,"gomme":{"calisiyor":true,"neden":null,"bugun":5},"ocr":{"calisiyor":false,"neden":"saat dışı","bugun":6}}"#;

#[test]
fn ornek_politika_okunur() {
    let p = Politika::ayristir(ORNEK).unwrap();
    assert_eq!((p.sinirlar.bellek_yuzde, p.sinirlar.cpu_yuzde), (90, 50));
    assert!(p.sinirlar.kullanirken_dur);
    assert_eq!(p.ocr.saatler, "00:00-06:30");
    assert_eq!(p.gomme.saatler, "her zaman");
}

#[test]
fn durum_json_alanlari() {
    let d: ArkaplanDurumu = serde_json::from_str(DURUM).unwrap();
    assert_eq!((d.zaman, d.parca_vektoru, d.ocr_kuyrugu), (1, (2, 3), 4));
    assert_eq!(d.ocr.neden.as_deref(), Some("saat dışı"));
    assert_eq!((d.gomme.bugun, d.ocr.bugun), (5, 6));
}
```

- [ ] **Adım 2: Başarısız olduğunu gör**

Çalıştır: `cargo test -p arkaplan-politika`
Beklenen: derleme hatası (`Okuyucu`, `yaz`, `Kapi`, … tanımsız).

- [ ] **Adım 3: `src/dosya.rs`'i yaz** (sınama modülünün üstüne, `//! Görev 2.` satırının yerine)

```rust
//! Dosyalar: politika (`%APPDATA%\ibnun-nedim\arkaplan.toml`, ibnü'n-Nedim sahibi) ve Nazar'ın
//! durum dosyası (`%LOCALAPPDATA%\nazar\arkaplan-durum.json`). Yazma geçici dosya + ad değiştirme:
//! okuyan yarım dosya görmez.

use crate::Politika;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

fn taban(degisken: &str) -> PathBuf {
    std::env::var_os(degisken).map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

pub fn politika_yolu() -> PathBuf {
    taban("APPDATA").join("ibnun-nedim").join("arkaplan.toml")
}

pub fn durum_yolu() -> PathBuf {
    taban("LOCALAPPDATA").join("nazar").join("arkaplan-durum.json")
}

fn atomik_yaz(yol: &Path, icerik: &[u8]) -> io::Result<()> {
    if let Some(d) = yol.parent() {
        fs::create_dir_all(d)?;
    }
    let gecici = yol.with_extension(format!("yaziliyor-{}", std::process::id()));
    fs::write(&gecici, icerik)?;
    fs::rename(&gecici, yol)
}

pub fn yaz(yol: &Path, p: &Politika) -> io::Result<()> {
    atomik_yaz(yol, p.metin().as_bytes())
}

#[derive(Clone, Debug, PartialEq)]
pub enum Okuma {
    Yok,
    Gecerli(Politika),
    /// Okunamadı ya da doğrulanmadı; son geçerli politika (varsa) sürer, dosyanın üstüne yazılmaz.
    Bozuk { hata: String, son_gecerli: Option<Politika> },
}

/// Politikayı değişme tarihine bakarak yeniden okur (her denetimde maliyet yalnız `metadata`).
pub struct Okuyucu {
    yol: PathBuf,
    son: Mutex<(Option<SystemTime>, Okuma)>,
}

impl Okuyucu {
    pub fn yeni(yol: PathBuf) -> Self {
        Self { yol, son: Mutex::new((None, Okuma::Yok)) }
    }

    pub fn yol(&self) -> &Path {
        &self.yol
    }

    pub fn oku(&self) -> Okuma {
        let mut son = self.son.lock().unwrap_or_else(|e| e.into_inner());
        let Some(tarih) = fs::metadata(&self.yol).and_then(|m| m.modified()).ok() else {
            *son = (None, Okuma::Yok);
            return Okuma::Yok;
        };
        if son.0 == Some(tarih) {
            return son.1.clone();
        }
        let onceki = match &son.1 {
            Okuma::Gecerli(p) => Some(p.clone()),
            Okuma::Bozuk { son_gecerli, .. } => son_gecerli.clone(),
            Okuma::Yok => None,
        };
        let yeni = match fs::read_to_string(&self.yol) {
            Ok(m) => match Politika::ayristir(&m) {
                Ok(p) => Okuma::Gecerli(p),
                Err(hata) => Okuma::Bozuk { hata, son_gecerli: onceki },
            },
            Err(e) => Okuma::Bozuk { hata: e.to_string(), son_gecerli: onceki },
        };
        if let Okuma::Bozuk { hata, .. } = &yeni {
            eprintln!("{} okunamadı, son geçerli politika sürüyor: {hata}", self.yol.display());
        }
        *son = (Some(tarih), yeni.clone());
        yeni
    }

    /// Uygulanacak politika: geçerli, bozuksa son geçerli, dosya yoksa `None`.
    pub fn etkin(&self) -> Option<Politika> {
        match self.oku() {
            Okuma::Gecerli(p) => Some(p),
            Okuma::Bozuk { son_gecerli, .. } => son_gecerli,
            Okuma::Yok => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IsDurumu {
    pub calisiyor: bool,
    pub neden: Option<String>,
    /// Bugün işlenen: gömmede parça, OCR'da belge.
    pub bugun: u64,
}

/// nazard'ın ~10 sn'de bir yazdığı durum; Arka plan sekmesi okur.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArkaplanDurumu {
    pub zaman: i64,
    /// (vektörü olan, toplam) parça.
    pub parca_vektoru: (usize, usize),
    pub ocr_kuyrugu: usize,
    pub gomme: IsDurumu,
    pub ocr: IsDurumu,
}

pub fn durum_yaz(yol: &Path, d: &ArkaplanDurumu) -> io::Result<()> {
    atomik_yaz(yol, &serde_json::to_vec_pretty(d).map_err(io::Error::other)?)
}

pub fn durum_oku(yol: &Path) -> Option<ArkaplanDurumu> {
    serde_json::from_slice(&fs::read(yol).ok()?).ok()
}
```

`src/karar.rs`'e `Kapi` (sınama modülünün üstüne):
```rust
use crate::Okuyucu;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::Duration;

/// Politika kapısı: dosyayı okur, ölçer, karar verir; bellek eşiğinin önceki hâlini ve son nedeni tutar.
pub struct Kapi {
    okuyucu: Okuyucu,
    bellek_dar: AtomicBool,
    son: Mutex<Option<Neden>>,
}

impl Kapi {
    pub fn yeni(politika: PathBuf) -> Self {
        Self { okuyucu: Okuyucu::yeni(politika), bellek_dar: AtomicBool::new(false), son: Mutex::new(None) }
    }

    /// İş şimdi koşabilir mi. Dosya yoksa (ya da hiç geçerli okunmadıysa) `varsayilan`; o da yoksa
    /// ölçmeden izin. Ölçüm ~1 sn sürer (CPU örneği): çizim ya da async iş parçacığında çağrılmaz.
    /// ponytail: duraklatma/saat engelinde de CPU ölçülür (1 sn boşa); sık çağrılırsa önce ucuz koşullar.
    pub fn dene(&self, is: IsTuru, varsayilan: Option<&Politika>) -> Result<(), Neden> {
        let p = match (self.okuyucu.etkin(), varsayilan) {
            (Some(p), _) => p,
            (None, Some(v)) => v.clone(),
            (None, None) => {
                *self.son.lock().unwrap_or_else(|e| e.into_inner()) = None;
                return Ok(());
            }
        };
        let o = crate::sistem::olc(Duration::from_secs(1));
        let onceki = self.bellek_dar.load(Relaxed);
        self.bellek_dar
            .store(o.bellek.is_some_and(|y| bellek_dar_mi(p.sinirlar.bellek_yuzde, y, onceki)), Relaxed);
        let r = karar(&p, is, &o, onceki);
        *self.son.lock().unwrap_or_else(|e| e.into_inner()) = r.err();
        r
    }

    pub fn son_neden(&self) -> Option<Neden> {
        *self.son.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn okuyucu(&self) -> &Okuyucu {
        &self.okuyucu
    }
}
```

`src/sistem.rs`'e geçici ölçüm (Görev 3 değiştirir):
```rust
pub fn olc(_aralik: std::time::Duration) -> crate::Olcum {
    crate::Olcum { simdi: simdi(), ..Default::default() }
}
```

- [ ] **Adım 4: Geçtiğini gör**

Çalıştır: `cargo test -p arkaplan-politika`
Beklenen: Görev 1'in 10 sınaması, buradaki 6 birim sınaması ve 2 sözleşme sınaması, toplam 18 sınama geçer.

- [ ] **Adım 5: Commit**

```bash
git add crates/arkaplan-politika
git commit -m "arkaplan-politika: atomik dosya, Okuyucu (son geçerli), durum JSON, Kapi

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Görev 3: `arkaplan-politika`: Windows ölçümleri (`sistem::olc`)

**Dosyalar:**
- Değiştir: `crates/arkaplan-politika/src/sistem.rs`. Geçici `olc` kalkar.

**Arayüzler:**
- Üretir:
  - `sistem::olc(Duration) -> Olcum`: aralık boyunca uyur.
  - `sistem::bellek_yuku() -> Option<u32>`
  - `sistem::girdisiz_sn() -> Option<u64>`
  - `sistem::kullanici_payi(toplam: u64, mesgul: u64, arka_plan: u64) -> Option<u32>`

- [ ] **Adım 1: Başarısız sınamaları yaz** (`sistem.rs` sonu)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kullanici_payi_arka_plani_duser() {
        // 4 çekirdek × 1 sn = 40_000_000 (100 ns); 30M meşgul, bunun 20M'i arka plan → %25.
        assert_eq!(kullanici_payi(40_000_000, 30_000_000, 20_000_000), Some(25));
        assert_eq!(kullanici_payi(40_000_000, 10_000_000, 20_000_000), Some(0), "taşma yok");
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
```

- [ ] **Adım 2: Başarısız olduğunu gör**

Çalıştır: `cargo test -p arkaplan-politika sistem`
Beklenen: `kullanici_payi` tanımsız diye derleme hatası.

- [ ] **Adım 3: Ölçümleri yaz** (geçici `olc`'nin yerine, `gun_saniyesi`/`yerel_tarih`'in altına)

```rust
use crate::Olcum;
use std::collections::HashMap;
use std::time::Duration;

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
        let mut l = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
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
fn arka_plan_zamanlari() -> HashMap<u32, u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let mut sonuc = HashMap::new();
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
        let n = g.szExeFile.iter().position(|&c| c == 0).unwrap_or(g.szExeFile.len());
        let ad = String::from_utf16_lossy(&g.szExeFile[..n]).to_ascii_lowercase();
        if crate::ARKA_PLAN_SURECLERI.contains(&ad.as_str()) {
            // SAFETY: yalnız sorgu hakkı; tutamaç aşağıda kapatılır.
            let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, g.th32ProcessID) };
            if !h.is_null() {
                // SAFETY: düz veri.
                let (mut a, mut b, mut c, mut k): (FILETIME, FILETIME, FILETIME, FILETIME) = unsafe { std::mem::zeroed() };
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
pub fn olc(aralik: Duration) -> Olcum {
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
                let arka: u64 = a1.iter().map(|(pid, &v)| v.saturating_sub(a0.get(pid).copied().unwrap_or(v))).sum();
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
    Olcum {
        simdi: simdi(),
        gun_dakikasi: gun_saniyesi().map(|s| s / 60),
        bellek: bellek_yuku(),
        kullanici_cpu,
        girdisiz_sn: girdisiz_sn(),
    }
}
```
Windows dışında `HashMap` kullanılmaz: `use std::collections::HashMap;` satırı `#[cfg(windows)]` ile işaretlenir.

- [ ] **Adım 4: Geçtiğini gör; uyarı yok**

Çalıştır: `cargo test -p arkaplan-politika; cargo clippy -p arkaplan-politika --all-targets -- -D warnings`
Beklenen: 20 sınama geçer, clippy temiz.

- [ ] **Adım 5: Commit ve push** (Nazar Görev 8'de bu rev'e bağlanacak)

```bash
git add crates/arkaplan-politika
git commit -m "arkaplan-politika: Windows ölçümleri (bellek, son girdi, arka plan süreçleri düşülmüş CPU)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
git rev-parse HEAD   # Görev 8'deki <POLITIKA_REV>
```

---

### Görev 4: ibnü'n-Nedim toplu iş kapısı

**Dosyalar:**
- Değiştir: `crates/bge-clients/Cargo.toml` (`arkaplan-politika.workspace = true` bağımlılığı).
- Oluştur: `crates/bge-clients/src/politika.rs`
- Değiştir: `crates/bge-clients/src/lib.rs` (`pub mod politika;`).
- Değiştir: `crates/bge-clients/src/is.rs` (yeni `Evre::PolitikaBekliyor`; `bekliyor_mu` onu da kapsar).
- Değiştir: `src/main.rs` (`gom()` içinde kapı).
- Değiştir: `crates/bge-gui/src/lib.rs` (`stage_label`).

**Arayüzler:**
- Tüketir: `Kapi`, `IsTuru` (Görev 2).
- Üretir:
  - `bge_clients::politika::KAPI: LazyLock<Kapi>`
  - `bge_clients::politika::bekle()`
  - `bge_clients::politika::bekle_kapida(&Kapi, Duration)`
  - `bge_clients::Evre::PolitikaBekliyor`

- [ ] **Adım 1: Başarısız sınama** (`crates/bge-clients/src/politika.rs`'in sonu)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arkaplan_politika::{yaz, Politika, SURDURENE_DEK};

    #[test]
    fn duraklatilmisken_bekler_surdurulunce_gecer() {
        let yol = std::env::temp_dir().join(format!("bge-politika-kapi-{}.toml", std::process::id()));
        let mut p = Politika::default();
        p.gomme.duraklat_bitis = SURDURENE_DEK;
        yaz(&yol, &p).unwrap();
        let kapi = std::sync::Arc::new(Kapi::yeni(yol.clone()));
        let k = kapi.clone();
        let is = std::thread::spawn(move || bekle_kapida(&k, Duration::from_millis(100)));
        std::thread::sleep(Duration::from_millis(2500));
        assert!(!is.is_finished(), "duraklatılmışken toplu iş beklemeli");
        p.gomme.duraklat_bitis = 0;
        p.sinirlar.bellek_yuzde = 100;
        p.sinirlar.cpu_yuzde = 100; // sınama makinesinin yükü kararı bozmasın
        yaz(&yol, &p).unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(&yol).unwrap();
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(5)).unwrap();
        let bas = std::time::Instant::now();
        while !is.is_finished() {
            assert!(bas.elapsed() < Duration::from_secs(10), "sürdürülünce geçmeli");
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_file(yol);
    }
}
```

- [ ] **Adım 2: Başarısız olduğunu gör**

Çalıştır: `cargo test -p bge-clients politika`
Beklenen: modül yok diye derleme hatası.

- [ ] **Adım 3: Kapıyı yaz**

`crates/bge-clients/src/politika.rs` (sınamanın üstü):
```rust
//! Toplu istekler için arka plan politikası (Arka plan sekmesi; `arkaplan-politika`). Hızlı şerit
//! (tek, kısa istek: aramalar, nazard yoklaması) buraya hiç uğramaz: duraklatma aramayı yavaşlatmaz.

pub use arkaplan_politika::Kapi;
use std::sync::LazyLock;
use std::time::Duration;

pub static KAPI: LazyLock<Kapi> = LazyLock::new(|| Kapi::yeni(arkaplan_politika::politika_yolu()));

/// İzin gelene dek bekler (dosya yoksa hemen döner). Engelleyicidir: `spawn_blocking` içinde çağrılır.
pub fn bekle() {
    bekle_kapida(&KAPI, Duration::from_secs(5));
}

pub fn bekle_kapida(kapi: &Kapi, aralik: Duration) {
    while kapi.dene(arkaplan_politika::IsTuru::Gomme, None).is_err() {
        std::thread::sleep(aralik);
    }
}
```

`crates/bge-clients/src/lib.rs`: `mod is;` satırının altına `pub mod politika;`.

`crates/bge-clients/src/is.rs`: `Evre`'ye, `SiraBekliyor`'un üstüne şu varyant eklenir:
```rust
    /// Toplu iş, arka plan politikası izin versin diye bekliyor (duraklatma, saat, bellek, CPU, kullanıcı).
    PolitikaBekliyor,
```
`bekliyor_mu` şöyle olur:
```rust
        matches!(self, Evre::ModelBekliyor | Evre::PolitikaBekliyor | Evre::SiraBekliyor)
```

`src/main.rs` içinde `gom()`'de `let hizli = kisa_mi(&inputs);` satırının hemen altına:
```rust
    if !hizli {
        // Arka plan politikası (Arka plan sekmesi); dosya yoksa hemen geçer.
        let _evre = bekci.evre(Evre::PolitikaBekliyor, text_count);
        let _ = tokio::task::spawn_blocking(bge_clients::politika::bekle).await;
    }
```

`crates/bge-gui/src/lib.rs`'teki `stage_label`'a kol eklenir. Süre sonsuz olduğu için takılma çubuğu kırmızıya dönmez:
```rust
        bge_clients::Evre::PolitikaBekliyor => ("held by background policy", f32::INFINITY),
```

- [ ] **Adım 4: Geçtiğini gör**

Çalıştır: `cargo test -p bge-clients; cargo test --release --bin ibnun-nedim; cargo build --release`
Beklenen: hepsi geçer, `is.rs`'teki mevcut sınamalar da.

- [ ] **Adım 5: Commit**

```bash
git add crates/bge-clients src/main.rs crates/bge-gui/src/lib.rs Cargo.lock
git commit -m "Toplu iş arka plan politikasını bekler (PolitikaBekliyor evresi)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Görev 5: `bge-arkaplan-sekme`: arayüzsüz mantık

**Dosyalar:**
- Oluştur:
  - `crates/bge-arkaplan-sekme/Cargo.toml`
  - `src/mantik.rs`
  - `src/lib.rs` (şimdilik yalnız `mod mantik; pub use mantik::*;`)
- Değiştir: kök `Cargo.toml` workspace bağımlılıkları (`bge-arkaplan-sekme = { path = "crates/bge-arkaplan-sekme" }`).

**Arayüzler:**
- Tüketir: `Politika`, `IsTuru`, `Duraklatma`, `SaatAraligi`, `Neden`, `ArkaplanDurumu`, `duraklat_bitisi`, `SURDURENE_DEK`.
- Üretir:
  - `duraklat(&mut Politika, IsTuru, Duraklatma, simdi: i64, gun_sn: Option<u32>)`
  - `surdur(&mut Politika, IsTuru)`
  - `saat_ayarla(&mut Politika, IsTuru, SaatAraligi)`
  - `duraklatildi_mi(&Politika, IsTuru, simdi: i64) -> bool`
  - `bitis_metni(bitis: i64, simdi: i64, gun_sn: Option<u32>) -> String`
  - `Tur { Calisiyor, Bekliyor, Duraklatildi, Bosta, Hata }`
  - `SatirDurumu { tur: Tur, metin: String }`
  - `gomme_durumu(&Politika, simdi, gun_sn, son_neden: Option<Neden>, bekleyen: bool, hesaplanan: bool) -> SatirDurumu`
  - `ocr_durumu(&Politika, simdi, gun_sn, Option<&ArkaplanDurumu>) -> SatirDurumu`
  - `bugun_farki(&mut (Option<u32>, u64), tarih: Option<u32>, toplam: u64) -> u64`
  - `sira(IsTuru) -> usize`
  - `BAYAT_SN`, `VARSAYILAN_ARALIK`

- [ ] **Adım 1: Sandık ve başarısız sınamalar**

`crates/bge-arkaplan-sekme/Cargo.toml`:
```toml
[package]
name = "bge-arkaplan-sekme"
version = "0.1.0"
edition.workspace = true
license.workspace = true
description = "ibnun-nedim: Arka plan sekmesi (toplu gömme + Nazar OCR politikası)"
repository.workspace = true

[dependencies]
arkaplan-politika.workspace = true
bge-clients.workspace = true
bge-settings.workspace = true
bge-theme.workspace = true
eframe.workspace = true
kilim-tema.workspace = true
```

`src/mantik.rs` sınamaları (dosya sonu):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use arkaplan_politika::{ArkaplanDurumu, IsDurumu, Neden, Politika};

    #[test]
    fn duraklat_ve_surdur() {
        let mut p = Politika::default();
        duraklat(&mut p, IsTuru::Ocr, Duraklatma::BirSaat, 1_000, None);
        assert!(duraklatildi_mi(&p, IsTuru::Ocr, 1_000) && !duraklatildi_mi(&p, IsTuru::Gomme, 1_000));
        assert!(!duraklatildi_mi(&p, IsTuru::Ocr, 4_600), "süre dolunca kendiliğinden kalkar");
        surdur(&mut p, IsTuru::Ocr);
        assert!(!duraklatildi_mi(&p, IsTuru::Ocr, 1_000));
    }

    #[test]
    fn bitis_yerel_saatle() {
        // Şimdi 15:00 (54000 sn), bitiş 40 dk sonra → 15:40.
        assert_eq!(bitis_metni(2_400 + 100, 100, Some(54_000)), "bitiş 15:40");
        assert_eq!(bitis_metni(SURDURENE_DEK, 100, Some(54_000)), "sürdürene dek");
        assert_eq!(bitis_metni(100 + 600, 100, None), "10 dk daha");
    }

    #[test]
    fn gomme_satiri() {
        let p = Politika::default();
        let d = gomme_durumu(&p, 0, None, Some(Neden::SaatDisi), true, false);
        assert_eq!((d.tur, d.metin.as_str()), (Tur::Bekliyor, "saat dışı"));
        assert_eq!(gomme_durumu(&p, 0, None, None, false, true).tur, Tur::Calisiyor);
        assert_eq!(gomme_durumu(&p, 0, None, None, false, false).tur, Tur::Bosta);
        let mut q = p.clone();
        q.gomme.duraklat_bitis = SURDURENE_DEK;
        let d = gomme_durumu(&q, 0, None, None, true, false);
        assert_eq!((d.tur, d.metin.as_str()), (Tur::Duraklatildi, "duraklatıldı · sürdürene dek"));
    }

    #[test]
    fn bayat_nazar_durumu() {
        let p = Politika::default();
        let d = ArkaplanDurumu { zaman: 0, ocr_kuyrugu: 3, ..Default::default() };
        let s = ocr_durumu(&p, BAYAT_SN + 1, None, Some(&d));
        assert_eq!((s.tur, s.metin.as_str()), (Tur::Hata, "nazard yanıt vermiyor"));
        assert_eq!(ocr_durumu(&p, 0, None, None).tur, Tur::Hata);
        let d = ArkaplanDurumu { zaman: 0, ocr_kuyrugu: 3, ocr: IsDurumu { neden: Some("saat dışı".into()), ..Default::default() }, ..Default::default() };
        assert_eq!(ocr_durumu(&p, 10, None, Some(&d)).metin, "saat dışı");
        let d = ArkaplanDurumu { zaman: 0, ocr_kuyrugu: 0, ..Default::default() };
        assert_eq!(ocr_durumu(&p, 10, None, Some(&d)).tur, Tur::Bosta);
    }

    #[test]
    fn bugun_gece_yarisi_sifirlanir() {
        let mut baz = (Some(20261013), 0);
        assert_eq!(bugun_farki(&mut baz, Some(20261013), 40), 40);
        assert_eq!(bugun_farki(&mut baz, Some(20261014), 55), 0, "yeni gün");
        assert_eq!(bugun_farki(&mut baz, Some(20261014), 60), 5);
    }

    #[test]
    fn saat_ayari_yazilir() {
        let mut p = Politika::default();
        saat_ayarla(&mut p, IsTuru::Gomme, VARSAYILAN_ARALIK);
        assert_eq!(p.gomme.saatler, "00:00-06:30");
    }
}
```

- [ ] **Adım 2: Başarısız olduğunu gör**

Çalıştır: `cargo test -p bge-arkaplan-sekme`
Beklenen: derleme hatası (işlevler tanımsız).

- [ ] **Adım 3: Mantığı yaz** (`src/mantik.rs`, sınamanın üstü)

```rust
//! Sekmenin arayüzsüz mantığı: düğme → politikaya yazılan değer, satır durumu metinleri.

use arkaplan_politika::{
    duraklat_bitisi, ArkaplanDurumu, Duraklatma, IsTuru, Neden, Politika, SaatAraligi, SURDURENE_DEK,
};

/// Nazar durum dosyası bundan eskiyse nazard yanıt vermiyor sayılır.
pub const BAYAT_SN: i64 = 60;
/// "her zaman" kapatılınca önerilen aralık.
pub const VARSAYILAN_ARALIK: SaatAraligi = SaatAraligi::Aralik { bas: 0, son: 390 };

pub fn sira(is: IsTuru) -> usize {
    match is {
        IsTuru::Gomme => 0,
        IsTuru::Ocr => 1,
    }
}

pub fn duraklat(p: &mut Politika, is: IsTuru, d: Duraklatma, simdi: i64, gun_sn: Option<u32>) {
    p.is_mut(is).duraklat_bitis = duraklat_bitisi(d, simdi, gun_sn);
}

pub fn surdur(p: &mut Politika, is: IsTuru) {
    p.is_mut(is).duraklat_bitis = 0;
}

pub fn saat_ayarla(p: &mut Politika, is: IsTuru, a: SaatAraligi) {
    p.is_mut(is).saatler = a.metin();
}

pub fn duraklatildi_mi(p: &Politika, is: IsTuru, simdi: i64) -> bool {
    p.is(is).duraklat_bitis > simdi
}

/// Duraklatmanın bitişi yerel saatle ("bitiş 15:40"); yerel saat bilinmiyorsa kalan dakika.
pub fn bitis_metni(bitis: i64, simdi: i64, gun_sn: Option<u32>) -> String {
    if bitis == SURDURENE_DEK {
        return "sürdürene dek".into();
    }
    let Some(g) = gun_sn else {
        return format!("{} dk daha", (bitis - simdi).max(0) / 60);
    };
    let yerel = (i64::from(g) + (bitis - simdi)).rem_euclid(86_400);
    format!("bitiş {:02}:{:02}", yerel / 3_600, yerel % 3_600 / 60)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tur {
    Calisiyor,
    Bekliyor,
    Duraklatildi,
    Bosta,
    Hata,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SatirDurumu {
    pub tur: Tur,
    pub metin: String,
}

fn satir(tur: Tur, metin: impl Into<String>) -> SatirDurumu {
    SatirDurumu { tur, metin: metin.into() }
}

fn duraklatma_satiri(p: &Politika, is: IsTuru, simdi: i64, gun_sn: Option<u32>) -> Option<SatirDurumu> {
    duraklatildi_mi(p, is, simdi).then(|| {
        satir(Tur::Duraklatildi, format!("duraklatıldı · {}", bitis_metni(p.is(is).duraklat_bitis, simdi, gun_sn)))
    })
}

/// `bekleyen`: politika bekleyen toplu iş var; `hesaplanan`: toplu iş hesaplanıyor.
pub fn gomme_durumu(
    p: &Politika,
    simdi: i64,
    gun_sn: Option<u32>,
    son_neden: Option<Neden>,
    bekleyen: bool,
    hesaplanan: bool,
) -> SatirDurumu {
    if let Some(d) = duraklatma_satiri(p, IsTuru::Gomme, simdi, gun_sn) {
        return d;
    }
    match (bekleyen, son_neden, hesaplanan) {
        (true, Some(n), _) => satir(Tur::Bekliyor, n.to_string()),
        (_, _, true) => satir(Tur::Calisiyor, "çalışıyor"),
        _ => satir(Tur::Bosta, "boşta"),
    }
}

pub fn ocr_durumu(p: &Politika, simdi: i64, gun_sn: Option<u32>, nazar: Option<&ArkaplanDurumu>) -> SatirDurumu {
    if let Some(d) = duraklatma_satiri(p, IsTuru::Ocr, simdi, gun_sn) {
        return d;
    }
    let Some(d) = nazar.filter(|d| simdi - d.zaman <= BAYAT_SN) else {
        return satir(Tur::Hata, "nazard yanıt vermiyor");
    };
    match (&d.ocr.neden, d.ocr_kuyrugu) {
        (Some(n), _) => satir(Tur::Bekliyor, n.clone()),
        (None, 0) => satir(Tur::Bosta, "kuyruk boş"),
        (None, _) => satir(Tur::Calisiyor, "çalışıyor"),
    }
}

/// Bugün gömülen: sayaç gün değişince o anki değerden yeniden sayılır.
pub fn bugun_farki(baz: &mut (Option<u32>, u64), tarih: Option<u32>, toplam: u64) -> u64 {
    if baz.0 != tarih {
        *baz = (tarih, toplam);
    }
    toplam.saturating_sub(baz.1)
}
```

`src/lib.rs`:
```rust
//! Arka plan sekmesi: toplu gömme (bu sunucu) ve OCR (Nazar) için durum, elle duraklatma, saat
//! aralığı ve ortak sınırlar. Politika `arkaplan.toml`'a anında yazılır (arkaplan-politika).

mod mantik;
pub use mantik::*;
```

- [ ] **Adım 4: Geçtiğini gör**

Çalıştır: `cargo test -p bge-arkaplan-sekme`
Beklenen: 6 sınama geçer.

- [ ] **Adım 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/bge-arkaplan-sekme
git commit -m "bge-arkaplan-sekme: arayüzsüz mantık (duraklatma, saat, satır durumları)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Görev 6: kilim-tema yükseltmesi (el-Fihrist ve ibnü'n-Nedim), `sarma_kur`, Türkçe sekme adları

**Dosyalar:**
- **el-Fihrist:** Kök `Cargo.toml`'da kilim-tema rev `a62bfc71227202fe1173b8f9d36aaa723fd5a12c` yerine `b62fcaceaf4e2804a86166c1dcf3ea01c500040f`.
- **ibnü'n-Nedim kök `Cargo.toml`:** kilim-tema rev aynı yeni değer; fihrist-tema rev el-Fihrist'in yeni commit'i.
- **ibnü'n-Nedim:**
  - `crates/bge-theme/src/lib.rs`: `apply` içinde `kilim_tema::sarma_kur(ctx)`.
  - `crates/bge-gui/src/lib.rs`: sekme adları Türkçe; yeniden çizim 250 → 500 ms.

**Arayüzler:** yeni arayüz yok. Bu görev Görev 7'nin kullandığı `kilim_tema::alan_satiri` ve `birim`'i getirir.

- [ ] **Adım 1: el-Fihrist'i yükselt**

`C:\Users\buzbe\Desktop\Github\rust\el-Fihrist\Cargo.toml` satır 38'deki rev'i değiştir, sonra:
```bash
cd C:/Users/buzbe/Desktop/Github/rust/el-Fihrist
cargo update -p kilim-tema
cargo test -p fihrist-tema 2>&1 | tail -5   # sınama özetine bakmak için; KIRMIZI aranırken kesme
cargo build --workspace > $TEMP/fihrist-kilim.log 2>&1; echo $?
```
Beklenen: çıkış 0. Kırılan API olursa kilim-tema `git log a62bfc7..b62fcac` notlarına bakılır; aradaki commit'ler ekleme niteliğinde (yerleşim yardımcıları, tema menüsü, onay kutusu, LF).

- [ ] **Adım 2: el-Fihrist commit ve push; rev'i al**

```bash
git add Cargo.toml Cargo.lock
git commit -m "kilim-tema b62fcac (taşmasız yerleşim yardımcıları)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
git rev-parse HEAD   # <FIHRIST_REV>
```
pre-push kapısı kırmızı verirse çıktısı dosyaya alınır ve KIRMIZI oradan okunur; kesilmez.

- [ ] **Adım 3: ibnü'n-Nedim rev'leri, `sarma_kur`, sekme adları, 500 ms**

Kök `Cargo.toml`:
- kilim-tema `rev = "b62fcaceaf4e2804a86166c1dcf3ea01c500040f"`
- fihrist-tema `rev = "<FIHRIST_REV>"`

`crates/bge-theme/src/lib.rs`, `apply`:
```rust
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    apply_visuals(ctx);
    ctx.set_theme(egui::ThemePreference::System);
    // Taşmasız yerleşim: her metin kullanılabilir genişlikte sarılır (kilim-tema yerlesim.rs).
    kilim_tema::sarma_kur(ctx);
}
```

`crates/bge-gui/src/lib.rs`, `nav_bar` içindeki dizi:
```rust
                for (label, target) in [
                    ("Durum", Screen::Status),
                    ("Bağlantılar", Screen::Connections),
                    ("Ayarlar", Screen::Settings),
                ] {
```
`GuiApp::ui` içinde `ctx.request_repaint_after(Duration::from_millis(250));` → `Duration::from_millis(500)`. Yorum: `// CLAUDE.md: zamanlı yeniden çizim en az 500 ms.`

- [ ] **Adım 4: Derle ve sına**

```bash
cd C:/Users/buzbe/Desktop/Github/bge-embed-rs
cargo update -p kilim-tema -p fihrist-tema
cargo test --workspace --release > $TEMP/nedim-kilim.log 2>&1; echo $?
cargo build --release --features fihrist >> $TEMP/nedim-kilim.log 2>&1; echo $?
```
Beklenen: ikisi de 0. `cargo tree -i kilim-tema --features fihrist` tek bir kilim-tema sürümü göstermeli; iki tane görünürse fihrist-tema rev'i yanlıştır.

- [ ] **Adım 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/bge-theme/src/lib.rs crates/bge-gui/src/lib.rs
git commit -m "kilim-tema b62fcac + sarma_kur; sekme adları Türkçe; yeniden çizim 500 ms

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Görev 7: Sekme arayüzü ve pencereye bağlanması

**Dosyalar:**
- Değiştir: `crates/bge-arkaplan-sekme/src/lib.rs` (`Sekme`, işçi, render).
- Değiştir: `crates/bge-gui/Cargo.toml` (`bge-arkaplan-sekme.workspace = true`).
- Değiştir: `crates/bge-gui/src/lib.rs`:
  - `Screen::ArkaPlan`
  - `GuiApp.arkaplan` alanı
  - eşleme kolu
  - dört sekme

**Arayüzler:**
- Tüketir:
  - Görev 5'in mantığı.
  - `bge_clients::politika::KAPI.son_neden()`, `bge_clients::defter().isler()`, `Evre`.
  - `bge_settings::Status.texts_embedded`.
  - `bge_theme`: `card`, `rich`, `status_dot_row`, `secondary_button`, `toggle`, `text_input`, `SIZE_*`, `GAP_*`, `INPUT_HEIGHT`.
  - `kilim_tema::alan_satiri`, `kilim_tema::birim`.
- Üretir: `bge_arkaplan_sekme::Sekme::yeni(&egui::Context) -> Sekme`, `Sekme::ui(&mut self, &mut Ui, Palette, &Status)`.

- [ ] **Adım 1: `Sekme`'yi yaz** (`src/lib.rs`'e, `pub use mantik::*;`'in altına)

```rust
use arkaplan_politika::{self as ap, ArkaplanDurumu, Duraklatma, IsTuru, Okuma, Politika, SaatAraligi};
use bge_settings::Status;
use bge_theme::{self as theme, Palette, Weight};
use eframe::egui::{self, Ui};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// İşçinin gönderdiği anlık durum: politika okuması, Nazar durumu, son yazma hatası.
#[derive(Clone, Debug, PartialEq)]
struct Anlik {
    okuma: Okuma,
    nazar: Option<ArkaplanDurumu>,
    yazma_hatasi: Option<String>,
}

/// Anahtar sütununun genişliği (`alan_satiri`).
const ANAHTAR: f32 = 96.0;
const SAAT_KUTUSU: f32 = 56.0;

pub struct Sekme {
    rx: Receiver<Anlik>,
    tx: Sender<Politika>,
    anlik: Option<Anlik>,
    /// (tarih, gün başındaki `texts_embedded`).
    bugun_baz: (Option<u32>, u64),
    /// Saat kutuları [gömme, ocr] × (baş, son); işçiden yeni anlık gelince doldurulur.
    saat: [(String, String); 2],
    saat_hatasi: [bool; 2],
    menu: Option<IsTuru>,
}

impl Sekme {
    /// Dosya işini yapan işçiyi açar: 5 sn'de bir (ya da yazma isteği gelince) okur, değişiklik
    /// varsa gönderir ve yeniden çizim ister. Çizim iş parçacığı dosyaya dokunmaz.
    pub fn yeni(ctx: &egui::Context) -> Self {
        let (atx, arx) = channel::<Anlik>();
        let (ptx, prx) = channel::<Politika>();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("arkaplan-sekme".into())
            .spawn(move || {
                let okuyucu = ap::Okuyucu::yeni(ap::politika_yolu());
                let durum = ap::durum_yolu();
                let mut onceki: Option<Anlik> = None;
                let mut yazma_hatasi = None;
                loop {
                    let a = Anlik { okuma: okuyucu.oku(), nazar: ap::durum_oku(&durum), yazma_hatasi: yazma_hatasi.take() };
                    if onceki.as_ref() != Some(&a) {
                        if atx.send(a.clone()).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                        onceki = Some(a);
                    }
                    match prx.recv_timeout(Duration::from_secs(5)) {
                        Ok(mut p) => {
                            // Sürükleme sırasında biriken istekler: yalnız sonuncusu yazılır.
                            while let Ok(q) = prx.try_recv() {
                                p = q;
                            }
                            if let Err(e) = ap::yaz(okuyucu.yol(), &p) {
                                yazma_hatasi = Some(e.to_string());
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
            })
            .expect("arkaplan-sekme iş parçacığı açılamadı");
        Self {
            rx: arx,
            tx: ptx,
            anlik: None,
            bugun_baz: (ap::sistem::yerel_tarih(), 0),
            saat: Default::default(),
            saat_hatasi: [false; 2],
            menu: None,
        }
    }

    /// Ekranda kullanılan politika: geçerli, bozuksa son geçerli, yoksa varsayılan.
    fn politika(&self) -> Politika {
        match self.anlik.as_ref().map(|a| &a.okuma) {
            Some(Okuma::Gecerli(p)) | Some(Okuma::Bozuk { son_gecerli: Some(p), .. }) => p.clone(),
            _ => Politika::default(),
        }
    }

    /// Değişikliği işçiye yazdırır; işçi yeniden okuyana dek ekranda yeni değer görünür.
    fn degistir(&mut self, f: impl FnOnce(&mut Politika)) {
        let mut p = self.politika();
        f(&mut p);
        if let Some(a) = self.anlik.as_mut() {
            a.okuma = Okuma::Gecerli(p.clone());
        }
        let _ = self.tx.send(p);
    }

    fn saatleri_doldur(&mut self) {
        let p = self.politika();
        for is in [IsTuru::Gomme, IsTuru::Ocr] {
            let a = match SaatAraligi::ayristir(&p.is(is).saatler) {
                Ok(SaatAraligi::Aralik { bas, son }) => (bas, son),
                _ => match VARSAYILAN_ARALIK {
                    SaatAraligi::Aralik { bas, son } => (bas, son),
                    SaatAraligi::HerZaman => (0, 390),
                },
            };
            let m = |d: u32| format!("{:02}:{:02}", d / 60, d % 60);
            self.saat[sira(is)] = (m(a.0), m(a.1));
        }
    }

    fn taze_nazar(&self, simdi: i64) -> Option<ArkaplanDurumu> {
        self.anlik.as_ref()?.nazar.clone().filter(|d| simdi - d.zaman <= BAYAT_SN)
    }

    pub fn ui(&mut self, ui: &mut Ui, p: Palette, status: &Status) {
        let mut yeni = false;
        while let Ok(a) = self.rx.try_recv() {
            self.anlik = Some(a);
            yeni = true;
        }
        if yeni {
            self.saatleri_doldur();
        }
        let simdi = ap::sistem::simdi();
        let gun_sn = ap::sistem::gun_saniyesi();
        self.render_dosya_notu(ui, p);
        self.render_gomme(ui, p, status, simdi, gun_sn);
        ui.add_space(theme::GAP_MD);
        self.render_ocr(ui, p, simdi, gun_sn);
        ui.add_space(theme::GAP_MD);
        self.render_sinirlar(ui, p);
    }

    fn render_dosya_notu(&self, ui: &mut Ui, p: Palette) {
        let Some(a) = &self.anlik else { return };
        let (metin, renk) = match (&a.yazma_hatasi, &a.okuma) {
            (Some(e), _) => (format!("Kaydedilemedi: {e}"), p.error),
            (None, Okuma::Bozuk { hata, .. }) => (
                format!("arkaplan.toml okunamadı ({hata}); son geçerli ayar sürüyor. İlk değişiklik dosyayı düzeltir."),
                p.error,
            ),
            (None, Okuma::Yok) => (
                "Ayar dosyası yok: varsayılanlar gösteriliyor, toplu gömme kısıtlanmıyor. İlk değişiklik dosyayı oluşturur.".into(),
                p.text_muted,
            ),
            (None, Okuma::Gecerli(_)) => return,
        };
        ui.label(theme::rich(metin, theme::SIZE_CAPTION, Weight::Regular, renk));
        ui.add_space(theme::GAP_SM);
    }

    fn render_gomme(&mut self, ui: &mut Ui, p: Palette, status: &Status, simdi: i64, gun_sn: Option<u32>) {
        let pol = self.politika();
        let isler = bge_clients::defter().isler();
        let bekleyen = isler.iter().any(|j| j.evre == bge_clients::Evre::PolitikaBekliyor);
        let hesaplanan = isler.iter().any(|j| j.evre == bge_clients::Evre::TopluHesap);
        let durum = gomme_durumu(&pol, simdi, gun_sn, bge_clients::politika::KAPI.son_neden(), bekleyen, hesaplanan);
        let toplam = status.texts_embedded.load(Ordering::Relaxed);
        let bugun = bugun_farki(&mut self.bugun_baz, ap::sistem::yerel_tarih(), toplam);
        let mut sayilar = format!("bugün {bugun} metin");
        if let Some(d) = self.taze_nazar(simdi) {
            sayilar += &format!(" · Nazar parça vektörü {}/{}", d.parca_vektoru.0, d.parca_vektoru.1);
        }
        self.render_is(ui, p, IsTuru::Gomme, "Toplu gömme", &pol, durum, sayilar, (simdi, gun_sn));
    }

    fn render_ocr(&mut self, ui: &mut Ui, p: Palette, simdi: i64, gun_sn: Option<u32>) {
        let pol = self.politika();
        let nazar = self.taze_nazar(simdi);
        let durum = ocr_durumu(&pol, simdi, gun_sn, nazar.as_ref());
        let sayilar = match &nazar {
            Some(d) => format!("kuyruk {} · bugün {} belge", d.ocr_kuyrugu, d.ocr.bugun),
            None => "nazard'dan durum gelmiyor".into(),
        };
        self.render_is(ui, p, IsTuru::Ocr, "OCR (Nazar)", &pol, durum, sayilar, (simdi, gun_sn));
    }

    #[allow(clippy::too_many_arguments)]
    fn render_is(
        &mut self,
        ui: &mut Ui,
        p: Palette,
        is: IsTuru,
        baslik: &str,
        pol: &Politika,
        durum: SatirDurumu,
        sayilar: String,
        (simdi, gun_sn): (i64, Option<u32>),
    ) {
        let renk = match durum.tur {
            Tur::Calisiyor => p.success,
            Tur::Bekliyor | Tur::Duraklatildi => p.warning,
            Tur::Bosta => p.text_muted,
            Tur::Hata => p.error,
        };
        theme::card(ui, p, |ui| {
            ui.label(theme::rich(baslik, theme::SIZE_BODY, Weight::SemiBold, p.text));
            ui.add_space(theme::GAP_XS);
            kilim_tema::alan_satiri(ui, ANAHTAR, "Durum", |ui| theme::status_dot_row(ui, p, renk, &durum.metin));
            kilim_tema::alan_satiri(ui, ANAHTAR, "Sayılar", |ui| {
                ui.label(theme::rich(sayilar, theme::SIZE_CAPTION, Weight::Regular, p.text_muted))
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "Saatler", |ui| self.render_saat(ui, p, is, pol));
            ui.add_space(theme::GAP_SM);
            self.render_duraklat(ui, p, is, pol, simdi, gun_sn);
        });
    }

    fn render_saat(&mut self, ui: &mut Ui, p: Palette, is: IsTuru, pol: &Politika) {
        let i = sira(is);
        let mut her_zaman =
            matches!(SaatAraligi::ayristir(&pol.is(is).saatler), Ok(SaatAraligi::HerZaman) | Err(_));
        ui.horizontal_wrapped(|ui| {
            if theme::toggle(ui, p, &mut her_zaman).changed() {
                let a = if her_zaman {
                    SaatAraligi::HerZaman
                } else {
                    SaatAraligi::ayristir(&format!("{}-{}", self.saat[i].0, self.saat[i].1)).unwrap_or(VARSAYILAN_ARALIK)
                };
                self.degistir(|q| saat_ayarla(q, is, a));
            }
            ui.label(theme::rich("her zaman", theme::SIZE_CAPTION, Weight::Regular, p.text));
            if her_zaman {
                return;
            }
            let boyut = egui::vec2(SAAT_KUTUSU, theme::INPUT_HEIGHT);
            let r1 = kilim_tema::birim(ui, SAAT_KUTUSU, |ui| theme::text_input(ui, p, &mut self.saat[i].0, boyut));
            ui.label(theme::rich("–", theme::SIZE_CAPTION, Weight::Regular, p.text));
            let r2 = kilim_tema::birim(ui, SAAT_KUTUSU, |ui| theme::text_input(ui, p, &mut self.saat[i].1, boyut));
            if r1.lost_focus() || r2.lost_focus() {
                match SaatAraligi::ayristir(&format!("{}-{}", self.saat[i].0, self.saat[i].1)) {
                    Ok(a) => {
                        self.saat_hatasi[i] = false;
                        self.degistir(|q| saat_ayarla(q, is, a));
                    }
                    Err(_) => self.saat_hatasi[i] = true,
                }
            }
            if self.saat_hatasi[i] {
                ui.label(theme::rich("SS:DD biçiminde, başlangıç ≠ bitiş", theme::SIZE_CAPTION, Weight::Regular, p.error));
            }
        });
    }

    fn render_duraklat(&mut self, ui: &mut Ui, p: Palette, is: IsTuru, pol: &Politika, simdi: i64, gun_sn: Option<u32>) {
        let durdu = duraklatildi_mi(pol, is, simdi);
        ui.horizontal_wrapped(|ui| {
            if durdu {
                if theme::secondary_button(ui, p, "Sürdür", true).clicked() {
                    self.degistir(|q| surdur(q, is));
                }
            } else if theme::secondary_button(ui, p, "Duraklat…", true).clicked() {
                self.menu = if self.menu == Some(is) { None } else { Some(is) };
            }
        });
        if self.menu == Some(is) && !durdu {
            ui.horizontal_wrapped(|ui| {
                for (ad, d) in [
                    ("1 saat", Duraklatma::BirSaat),
                    ("bugün boyunca", Duraklatma::BugunBoyunca),
                    ("ben sürdürene dek", Duraklatma::SurdureneDek),
                ] {
                    if theme::secondary_button(ui, p, ad, true).clicked() {
                        self.degistir(|q| duraklat(q, is, d, simdi, gun_sn));
                        self.menu = None;
                    }
                }
            });
        }
    }

    fn render_sinirlar(&mut self, ui: &mut Ui, p: Palette) {
        let pol = self.politika();
        let s = &pol.sinirlar;
        let (mut bellek, mut cpu, mut kullanirken) = (s.bellek_yuzde, s.cpu_yuzde, s.kullanirken_dur);
        let mut degisti = false;
        theme::card(ui, p, |ui| {
            ui.label(theme::rich("Ortak sınırlar", theme::SIZE_BODY, Weight::SemiBold, p.text));
            ui.add_space(theme::GAP_XS);
            kilim_tema::alan_satiri(ui, ANAHTAR, "Bellek", |ui| {
                degisti |= ui.add(egui::DragValue::new(&mut bellek).range(50..=99).suffix(" %")).changed();
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "İşlemci", |ui| {
                degisti |= ui.add(egui::DragValue::new(&mut cpu).range(10..=95).suffix(" %")).changed();
            });
            kilim_tema::alan_satiri(ui, ANAHTAR, "Kullanırken", |ui| {
                ui.horizontal(|ui| {
                    degisti |= theme::toggle(ui, p, &mut kullanirken).changed();
                    ui.label(theme::rich("bilgisayarı kullanırken dur", theme::SIZE_CAPTION, Weight::Regular, p.text));
                });
            });
            ui.label(theme::rich(
                "İşlemci sınırı yalnız senin programlarının yüküne bakar; ibnü'n-Nedim, nazard ve tesseract düşülür.",
                theme::SIZE_CAPTION,
                Weight::Regular,
                p.text_muted,
            ));
        });
        if degisti {
            self.degistir(|q| {
                q.sinirlar.bellek_yuzde = bellek;
                q.sinirlar.cpu_yuzde = cpu;
                q.sinirlar.kullanirken_dur = kullanirken;
            });
        }
    }
}
```

- [ ] **Adım 2: Pencereye bağla** (`crates/bge-gui`)

`Cargo.toml` `[dependencies]`: `bge-arkaplan-sekme.workspace = true`.

`src/lib.rs`:
- `enum Screen` içine `ArkaPlan,` (Settings'in altına).
- `pub struct GuiApp` içine `arkaplan: bge_arkaplan_sekme::Sekme,`.
- `GuiApp::new` içinde `Self { … }`'ye `arkaplan: bge_arkaplan_sekme::Sekme::yeni(ctx),`.
- Ekran eşlemesine yeni kol:
```rust
                            Screen::ArkaPlan => self.arkaplan.ui(ui, p, &self.status),
```
- `nav_bar`'da genişlik ve dizi:
```rust
                let width = (ui.available_width() - 3.0 * theme::GAP_XS) / 4.0;
                for (label, target) in [
                    ("Durum", Screen::Status),
                    ("Bağlantılar", Screen::Connections),
                    ("Ayarlar", Screen::Settings),
                    ("Arka plan", Screen::ArkaPlan),
                ] {
```
- Dosyanın başındaki modül yorumu: "The four-screen egui GUI … Durum, Bağlantılar, Ayarlar, Arka plan".

- [ ] **Adım 3: Derle, sına, clippy**

```bash
cargo test --workspace --release > $TEMP/nedim-sekme.log 2>&1; echo $?
cargo clippy --workspace --all-targets -- -D warnings >> $TEMP/nedim-sekme.log 2>&1; echo $?
kapi . > $TEMP/nedim-kapi.log 2>&1; grep -a "KIRMIZI\|kırmızı [1-9]\|/1500" $TEMP/nedim-kapi.log
```
Beklenen: 0, 0; sandıkların hepsi 1500'ün altında; kırmızı 0.

- [ ] **Adım 4: Commit**

```bash
git add crates/bge-arkaplan-sekme crates/bge-gui Cargo.lock
git commit -m "Arka plan sekmesi: durum, duraklatma, saatler, ortak sınırlar

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Görev 8: Nazar `nazar-arkaplan` sandığı

**Dosyalar:**
- Değiştir: Nazar kök `Cargo.toml`.
  - Üyelere `"crates/nazar-arkaplan"`.
  - `[workspace.dependencies]`'e `arkaplan-politika = { git = "https://github.com/Ercaner1988/ibn-unNedim", rev = "<POLITIKA_REV>" }`. Değer Görev 3, Adım 5'in `git rev-parse HEAD` çıktısıdır. Görev 4-7 ortak sandığı değiştirmez, o rev yeter.
- Oluştur: `crates/nazar-arkaplan/Cargo.toml`, `src/lib.rs`, `tests/sozlesme.rs`

**Arayüzler:**
- Tüketir: `Kapi`, `IsTuru`, `Neden`, `Politika`, `Okuma`, `ArkaplanDurumu`, `IsDurumu`, `durum_yaz`, `sistem::{simdi, yerel_tarih}`.
- Üretir:
  - `pub use arkaplan_politika::{self, IsTuru}`
  - `Baglam { aramadan_beri_sn: i64, indeksleniyor: bool, belge_vektoru_bekliyor: bool }`
  - `Engel { Politika(Neden), Arama, Indeksleme, BelgeVektoru }` (Display)
  - `nazar_engeli(IsTuru, &Baglam, bosta_gerekir: bool) -> Option<Engel>`
  - `Bekci::yeni(PathBuf)`
  - `.izin(IsTuru, &Baglam, bosta_gerekir: bool) -> Result<(), Engel>`
  - `.islendi(IsTuru, u64)`, `.islendi_tarihte(IsTuru, u64, Option<u32>)`
  - `.anlik((usize, usize), usize) -> ArkaplanDurumu`, `.anlik_tarihte(i64, Option<u32>, (usize, usize), usize) -> ArkaplanDurumu`
  - `.politika_okumasi() -> Okuma`
  - `durum_dongusu(&Bekci, &Path, girdi: impl Fn() -> ((usize, usize), usize), gunluk: impl Fn(&str)) -> !`

- [ ] **Adım 1: Sandık ve başarısız sınamalar**

`crates/nazar-arkaplan/Cargo.toml`:
```toml
[package]
name = "nazar-arkaplan"
version = "0.1.0"
edition = "2021"
description = "nazard arka plan işlerinin (gömme, OCR) ne zaman koşacağı: ortak politika + Nazar koşulları; durum dosyası"

[dependencies]
arkaplan-politika = { workspace = true }

[dev-dependencies]
serde_json = { workspace = true }
```

`src/lib.rs` sonuna sınamalar:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn bos() -> Baglam {
        Baglam { aramadan_beri_sn: 3_600, indeksleniyor: false, belge_vektoru_bekliyor: false }
    }

    #[test]
    fn nazar_kosullari() {
        let b = Baglam { aramadan_beri_sn: 10, ..bos() };
        assert_eq!(nazar_engeli(IsTuru::Gomme, &b, true), Some(Engel::Arama));
        assert_eq!(nazar_engeli(IsTuru::Gomme, &b, false), None, "belge vektörü aramayı beklemez");
        let b = Baglam { indeksleniyor: true, ..bos() };
        assert_eq!(nazar_engeli(IsTuru::Ocr, &b, true), Some(Engel::Indeksleme));
        let b = Baglam { belge_vektoru_bekliyor: true, ..bos() };
        assert_eq!(nazar_engeli(IsTuru::Ocr, &b, true), Some(Engel::BelgeVektoru));
        assert_eq!(nazar_engeli(IsTuru::Gomme, &b, true), None, "yalnız OCR belge vektörünü bekler");
        assert_eq!(Engel::Arama.to_string(), "arama sürüyor");
    }

    #[test]
    fn engel_durumu_kaydeder() {
        let b = Bekci::yeni(std::env::temp_dir().join("nazar-arkaplan-yok.toml"));
        let arama = Baglam { aramadan_beri_sn: 1, ..bos() };
        assert_eq!(b.izin(IsTuru::Ocr, &arama, true), Err(Engel::Arama));
        let d = b.anlik_tarihte(5, Some(20261013), (1, 2), 3);
        assert!(!d.ocr.calisiyor);
        assert_eq!(d.ocr.neden.as_deref(), Some("arama sürüyor"));
        assert_eq!((d.zaman, d.parca_vektoru, d.ocr_kuyrugu), (5, (1, 2), 3));
    }

    #[test]
    fn sayaclar_gece_yarisi_sifirlanir() {
        let b = Bekci::yeni(std::env::temp_dir().join("nazar-arkaplan-yok.toml"));
        b.islendi_tarihte(IsTuru::Gomme, 8, Some(20261013));
        b.islendi_tarihte(IsTuru::Ocr, 1, Some(20261013));
        let d = b.anlik_tarihte(0, Some(20261013), (0, 0), 0);
        assert_eq!((d.gomme.bugun, d.ocr.bugun), (8, 1));
        let d = b.anlik_tarihte(0, Some(20261014), (0, 0), 0);
        assert_eq!((d.gomme.bugun, d.ocr.bugun), (0, 0), "yeni gün");
    }
}
```

`tests/sozlesme.rs`: ibnü'n-Nedim'deki `crates/arkaplan-politika/tests/sozlesme.rs`'in AYNI içeriği (Görev 2, Adım 1'deki metin; `ORNEK` ve `DURUM` sabitleri bayt bayt aynı). Yalnız ilk satırdaki yorum "ibnü'n-Nedim reposundaki `crates/arkaplan-politika/tests/sozlesme.rs`" diye ötekini gösterir. `use` satırı: `use nazar_arkaplan::arkaplan_politika::{ArkaplanDurumu, Politika};`.

- [ ] **Adım 2: Başarısız olduğunu gör**

Çalıştır: `cd C:/Users/buzbe/Desktop/Github/rust/Nazar; cargo test -p nazar-arkaplan`
Beklenen: derleme hatası (`Bekci`, … tanımsız).

- [ ] **Adım 3: Sandığı yaz** (sınamaların üstü)

```rust
//! nazard arka plan işlerinin (belge/terim/parça gömmesi, kuyruk OCR'ı) ne zaman koşacağı: ortak
//! politika (`arkaplan-politika`, ibnü'n-Nedim reposu; Arka plan sekmesi yazar) + Nazar'ın kendi
//! koşulları. Sekmenin okuduğu durum dosyasını da yazar.

pub use arkaplan_politika::{self, IsTuru};
use arkaplan_politika::{sistem, ArkaplanDurumu, IsDurumu, Kapi, Neden, Okuma, Politika};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// Son aramadan bu kadar saniye geçmeden boşta işi başlamaz.
pub const BOSTA_SN: i64 = 60;

#[derive(Clone, Copy, Debug, Default)]
pub struct Baglam {
    pub aramadan_beri_sn: i64,
    pub indeksleniyor: bool,
    pub belge_vektoru_bekliyor: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engel {
    Politika(Neden),
    Arama,
    Indeksleme,
    BelgeVektoru,
}

impl fmt::Display for Engel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Engel::Politika(n) => write!(f, "{n}"),
            Engel::Arama => write!(f, "arama sürüyor"),
            Engel::Indeksleme => write!(f, "indeksleme sürüyor"),
            Engel::BelgeVektoru => write!(f, "belge vektörleri bekleniyor"),
        }
    }
}

/// Nazar'ın ölçümsüz (ucuz) koşulları; politikadan önce bakılır. `bosta_gerekir`: terim/parça
/// vektörü ve OCR için true, belge vektörü için false (yalnız politika).
pub fn nazar_engeli(is: IsTuru, b: &Baglam, bosta_gerekir: bool) -> Option<Engel> {
    if !bosta_gerekir {
        return None;
    }
    if b.aramadan_beri_sn < BOSTA_SN {
        Some(Engel::Arama)
    } else if b.indeksleniyor {
        Some(Engel::Indeksleme)
    } else if is == IsTuru::Ocr && b.belge_vektoru_bekliyor {
        Some(Engel::BelgeVektoru)
    } else {
        None
    }
}

#[derive(Default)]
struct Sayac {
    tarih: Option<u32>,
    gomme: u64,
    ocr: u64,
    gomme_durum: IsDurumu,
    ocr_durum: IsDurumu,
}

impl Sayac {
    fn gun(&mut self, tarih: Option<u32>) {
        if self.tarih != tarih {
            self.tarih = tarih;
            self.gomme = 0;
            self.ocr = 0;
        }
    }
}

pub struct Bekci {
    kapi: Kapi,
    sayac: Mutex<Sayac>,
}

impl Bekci {
    pub fn yeni(politika: PathBuf) -> Self {
        Self { kapi: Kapi::yeni(politika), sayac: Mutex::default() }
    }

    fn sayac(&self) -> std::sync::MutexGuard<'_, Sayac> {
        self.sayac.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// İş şimdi koşabilir mi; politika denetimi ~1 sn ölçer. Sonuç durum dosyasına yansır.
    pub fn izin(&self, is: IsTuru, b: &Baglam, bosta_gerekir: bool) -> Result<(), Engel> {
        let r = match nazar_engeli(is, b, bosta_gerekir) {
            Some(e) => Err(e),
            // Dosya yoksa bugünkü sınırlar (bellek %90, CPU %50), saat kısıtı yok.
            None => self.kapi.dene(is, Some(&Politika::default())).map_err(Engel::Politika),
        };
        let mut s = self.sayac();
        let d = match is {
            IsTuru::Gomme => &mut s.gomme_durum,
            IsTuru::Ocr => &mut s.ocr_durum,
        };
        d.calisiyor = r.is_ok();
        d.neden = r.as_ref().err().map(ToString::to_string);
        r
    }

    pub fn islendi(&self, is: IsTuru, n: u64) {
        self.islendi_tarihte(is, n, sistem::yerel_tarih());
    }

    pub fn islendi_tarihte(&self, is: IsTuru, n: u64, tarih: Option<u32>) {
        let mut s = self.sayac();
        s.gun(tarih);
        match is {
            IsTuru::Gomme => s.gomme += n,
            IsTuru::Ocr => s.ocr += n,
        }
    }

    pub fn anlik(&self, parca_vektoru: (usize, usize), ocr_kuyrugu: usize) -> ArkaplanDurumu {
        self.anlik_tarihte(sistem::simdi(), sistem::yerel_tarih(), parca_vektoru, ocr_kuyrugu)
    }

    pub fn anlik_tarihte(
        &self,
        simdi: i64,
        tarih: Option<u32>,
        parca_vektoru: (usize, usize),
        ocr_kuyrugu: usize,
    ) -> ArkaplanDurumu {
        let mut s = self.sayac();
        s.gun(tarih);
        ArkaplanDurumu {
            zaman: simdi,
            parca_vektoru,
            ocr_kuyrugu,
            gomme: IsDurumu { bugun: s.gomme, ..s.gomme_durum.clone() },
            ocr: IsDurumu { bugun: s.ocr, ..s.ocr_durum.clone() },
        }
    }

    pub fn politika_okumasi(&self) -> Okuma {
        self.kapi.okuyucu().oku()
    }
}

/// Durum dosyasını 10 sn'de bir yazar; bozuk politika ve yazma hatasını yalnız değişince günlüğe düşer.
pub fn durum_dongusu(
    b: &Bekci,
    yol: &Path,
    girdi: impl Fn() -> ((usize, usize), usize),
    gunluk: impl Fn(&str),
) -> ! {
    let (mut son_bozuk, mut son_yazma) = (None::<String>, None::<String>);
    loop {
        let bozuk = match b.politika_okumasi() {
            Okuma::Bozuk { hata, .. } => Some(hata),
            _ => None,
        };
        if bozuk != son_bozuk {
            if let Some(h) = &bozuk {
                gunluk(&format!("arkaplan.toml bozuk, son geçerli politika sürüyor: {h}"));
            }
            son_bozuk = bozuk;
        }
        let (kapsam, kuyruk) = girdi();
        let yazma = arkaplan_politika::durum_yaz(yol, &b.anlik(kapsam, kuyruk)).err().map(|e| e.to_string());
        if yazma != son_yazma {
            if let Some(e) = &yazma {
                gunluk(&format!("arka plan durumu yazılamadı ({}): {e}", yol.display()));
            }
            son_yazma = yazma;
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}
```

- [ ] **Adım 4: Geçtiğini gör**

Çalıştır: `cargo test -p nazar-arkaplan`
Beklenen: 3 birim + 2 sözleşme sınaması geçer. `engel_durumu_kaydeder` ölçüm yapmaz (arama engeli önce gelir).

- [ ] **Adım 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/nazar-arkaplan
git commit -m "nazar-arkaplan: ortak politika + Nazar koşulları, günlük sayaçlar, durum dosyası

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Görev 9: nazard'ı bağla; `ocr_saatleri`'ni kaldır

**Dosyalar:**
- Değiştir: `crates/nazar-daemon/Cargo.toml` (`nazar-arkaplan = { path = "../nazar-arkaplan" }`).
- Değiştir: `crates/nazar-daemon/src/motor.rs`
- Değiştir: `crates/nazar-daemon/src/lib.rs` (sınama ayarı)
- Değiştir: `crates/nazar-dis/src/ayar.rs`, `crates/nazar-dis/src/sistem.rs`

**Arayüzler:**
- Tüketir: Görev 8'in `Bekci`, `Baglam`, `IsTuru`, `durum_dongusu`.
- Üretir: `Motor.arkaplan: nazar_arkaplan::Bekci`, `Ayar.politika_dosyasi: Option<PathBuf>`.

- [ ] **Adım 1: Sınama ayarını yaz (önce kırmızı)**

`crates/nazar-daemon/src/lib.rs`'teki `kur()` içinde, `Ayar { … }` değişmezine `indeks_dizini`'nin altına şu satır eklenir. Sınama gerçek `%APPDATA%` politikasından bağımsız olur:
```rust
            politika_dosyasi: Some(d.join("politika-yok.toml")),
```
Çalıştır: `cargo test -p nazar-daemon`
Beklenen: derleme hatası, `Ayar`'da `politika_dosyasi` alanı yok.

- [ ] **Adım 2: `nazar-dis`**

`ayar.rs`:
- **Kaldırılır:**
  - `pub ocr_saatleri: Option<String>` alanı ve `Default`'taki satırı,
  - `yukle` içindeki `ocr_saatleri` doğrulaması (satır ~97-101),
  - `ocr_saatinde`, `aralik`, `dakika_icinde` işlevleri,
  - bunların sınamaları: `ocr_saatleri`/`aralik`/`dakika_icinde` geçen sınama işlevleri, satır ~225-240 civarı.

  Bu sınamalar zayıflatılmıyor: aynı davranış (gece yarısını aşan aralık, bozuk değer açık hata) `arkaplan-politika`'nın `gece_yarisini_asan_aralik` ve `bozuk_politika_aciklayici_hata` sınamalarına taşındı.
- **Eklenir:** `Ayar`'a alan (`#[serde(default)]` yapı düzeyinde zaten var):
```rust
    /// Arka plan politikası dosyası; yoksa `%APPDATA%\ibnun-nedim\arkaplan.toml` (ibnü'n-Nedim sekmesi yazar).
    pub politika_dosyasi: Option<PathBuf>,
```
  `Default`'a `politika_dosyasi: None,`.

`sistem.rs`:
- **Kaldırılır:** `yerel_dakika` ve `cpu_yuku` (artık çağıranı yok; ölçüm `arkaplan-politika`'da). Kaldırmadan önce `rg "yerel_dakika|cpu_yuku" crates` ile yalnız `motor.rs:745`'in kaldığı doğrulanır; o satır Adım 3'te gider.
- **Kalır:** `bellek_dar`/`bellek_bekle`. İndeksleme ve ısıtma işçileri bunları kullanmaya devam eder, onlar politika kapsamı dışında.

- [ ] **Adım 3: `motor.rs`**

1. `const BOSTA_SN` ve `const BOSTA_CPU` (satır 25-26) ile `fn bosta` (satır ~739-746) silinir.
2. `Motor`'a alan:
```rust
    /// Arka plan işlerinin politikası (Arka plan sekmesi) + Nazar koşulları.
    pub arkaplan: nazar_arkaplan::Bekci,
```
   `Motor::yeni`'de `Self { … }` değişmezinden ÖNCE (ayar taşınmadan):
```rust
        let politika = ayar
            .politika_dosyasi
            .clone()
            .unwrap_or_else(nazar_arkaplan::arkaplan_politika::politika_yolu);
```
   değişmeze `arkaplan: nazar_arkaplan::Bekci::yeni(politika),`.
3. `bosta`'nın yerine:
```rust
    fn baglam(&self) -> nazar_arkaplan::Baglam {
        nazar_arkaplan::Baglam {
            aramadan_beri_sn: unix(std::time::SystemTime::now()) - self.son_arama.load(Relaxed),
            indeksleniyor: self.indeksleniyor(),
            belge_vektoru_bekliyor: self.belge_vektoru_bekliyor(),
        }
    }
```
4. `ocr_sirasi`'nın ilk koşulu ve belge yorumu:
```rust
    /// Sıradaki OCR işi: önce acil (beklemeden); otomatik olan yalnız arka plan politikası ve
    /// Nazar koşulları izin verince (boşta, belge vektörleri bitmiş). Arama gelince yeni iş başlamaz.
    pub(crate) fn ocr_sirasi(&self) -> Option<PathBuf> {
        if self.arkaplan.izin(IsTuru::Ocr, &self.baglam(), true).is_err() {
            return self.ocr_acil.al(Duration::from_secs(5));
        }
```
   (`use nazar_arkaplan::IsTuru;` dosya başına.)
5. `gomme_dongusu`:
   - döngü başındaki `nazar_dis::sistem::bellek_bekle();` silinir (bellek artık politikada);
   - `if !belge_isleri.is_empty() {` bloğunun İLK satırları:
```rust
                if self.arkaplan.izin(IsTuru::Gomme, &self.baglam(), false).is_err() {
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
```
   - `if !self.bosta() {` satırı `if self.arkaplan.izin(IsTuru::Gomme, &self.baglam(), true).is_err() {` olur;
   - parça vektörleri atanan `for ((id, _), v) in isler.iter().zip(v) { … }` döngüsünün ardından, `self.kirli.store(true, Relaxed);` satırının üstüne:
```rust
                    self.arkaplan.islendi(IsTuru::Gomme, isler.len() as u64);
```
   - fonksiyonun belge yorumundaki "(2) yalnız boştayken (son 60 sn'de arama yok, sistem CPU < %50)" ifadesi "(2) yalnız arka plan politikası ve Nazar koşulları izin verince" olur.
6. OCR işçisinde `(Ok(c), Ok(md)) =>` kolunda `m.cozum_yaz(…)` çağrısının ardından:
```rust
                    m.arkaplan.islendi(IsTuru::Ocr, 1);
```
7. Durum yazıcısı: `std::thread::spawn(move || m.gomme_dongusu());` satırının altına:
```rust
        let m = self.clone();
        std::thread::spawn(move || {
            let yol = nazar_arkaplan::arkaplan_politika::durum_yolu();
            nazar_arkaplan::durum_dongusu(&m.arkaplan, &yol, || m.durum_girdisi(), |s| gunluk!("{s}"))
        });
```
   ve `kapsam_ozeti`'nin yanına:
```rust
    /// Durum dosyası için (vektörlü, toplam) parça ve OCR kuyruğu.
    fn durum_girdisi(&self) -> ((usize, usize), usize) {
        let kapsam = self
            .indeks
            .read()
            .map(|ix| (ix.parca_sayisi() - ix.vektorsuz_sayisi(), ix.parca_sayisi()))
            .unwrap_or_default();
        (kapsam, self.ocr_kuyrugu.uzunluk())
    }
```

- [ ] **Adım 4: Sına, kapı, sandık sınırı**

```bash
cargo test -p nazar-dis -p nazar-arkaplan -p nazar-daemon --release > $TEMP/nazar-arkaplan.log 2>&1; echo $?
kapi . > $TEMP/nazar-kapi.log 2>&1; grep -a -B1 "/1500" $TEMP/nazar-kapi.log; grep -a "kırmızı" $TEMP/nazar-kapi.log
```
Beklenen: 0; `nazar-daemon` ≤ 1500; kırmızı 0.

**`nazar-daemon` 1500'ü aşarsa:** `kapsam_ozeti` (biçim dizesi) `nazar-arkaplan`'a taşınır.
```rust
/// `nazard durum` ve günlük için vektör kapsamı metni.
pub fn kapsam_metni(belge: (usize, usize), parca: (usize, usize), terim: (usize, usize)) -> String {
    format!(
        "belge vektörü: {}/{}\nparça vektörü: {}/{}\nterim vektörü: {}/{}",
        belge.0, belge.1, parca.0, parca.1, terim.0, terim.1
    )
}
```
`motor.rs`'teki `kapsam_ozeti` gövdesi şu çağrıya iner:
```rust
nazar_arkaplan::kapsam_metni(ix.belge_vektoru_kapsami(), (p - ix.vektorsuz_sayisi(), p), (ix.terim_vektoru_sayisi(), TERIM_ONBELLEGI))
```
Yeniden ölçülür.

- [ ] **Adım 5: Commit ve push**

```bash
git add crates/nazar-daemon crates/nazar-dis Cargo.lock   # ocr_kayma.rs ASLA
git commit -m "nazard: arka plan politikası (Arka plan sekmesi); ocr_saatleri politikaya taşındı

Belge, terim ve parça vektörleri ile kuyruk OCR'ı arkaplan.toml'a sorar (duraklatma,
saat, bellek, kullanıcı CPU'su, kullanıcı etkinliği); Nazar'ın kendi koşulları sürer.
Durum %LOCALAPPDATA%\\nazar\\arkaplan-durum.json'a 10 sn'de bir yazılır.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

---

### Görev 10: Kurulum, `ac.ps1`, canlı ölçüm ve pencere doğrulaması

**Dosyalar:**
- Oluştur: `%APPDATA%\ibnun-nedim\arkaplan.toml`
- Değiştir: `%APPDATA%\nazar\ayar.toml` (`ocr_saatleri` satırı silinir)
- Değiştir: `~/.llama-embed-server/ac.ps1`

- [ ] **Adım 1: Politika dosyası ve ayar**

`%APPDATA%\ibnun-nedim\arkaplan.toml` (Write aracıyla):
```toml
surum = 1

[sinirlar]
bellek_yuzde = 90
cpu_yuzde = 50
kullanirken_dur = false

[gomme]
saatler = "her zaman"
duraklat_bitis = 0

[ocr]
saatler = "00:00-06:30"
duraklat_bitis = 0
```
`%APPDATA%\nazar\ayar.toml`: önce Read, sonra `ocr_saatleri = "00:00-06:30"` satırı Edit ile silinir. Öncesinde yedek scratchpad'e kopyalanır.

- [ ] **Adım 2: `ac.ps1`'i düzelt** (ASCII; pencere sürecin görünür, başlıklı penceresinden bulunur)

Dosyanın tamamı:
```powershell
# ibnun-nedim penceresini one getirir; calismiyorsa start.ps1 ile baslatir.
Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int c);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(System.IntPtr h);
[DllImport("user32.dll")] public static extern int GetWindowTextLength(System.IntPtr h);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc f, System.IntPtr l);
public delegate bool EnumWindowsProc(System.IntPtr h, System.IntPtr l);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
public static System.IntPtr AnaPencere(uint hedef) {
    System.IntPtr bulunan = System.IntPtr.Zero;
    EnumWindows((h, l) => {
        uint pid; GetWindowThreadProcessId(h, out pid);
        if (pid == hedef && GetWindowTextLength(h) > 0) { bulunan = h; return false; }
        return true;
    }, System.IntPtr.Zero);
    return bulunan;
}
'@

function Get-Pencere {
    $p = Get-Process ibnun-nedim -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $p) { return [System.IntPtr]::Zero }
    return [W.U]::AnaPencere([uint32]$p.Id)
}

if (-not (Get-Process ibnun-nedim -ErrorAction SilentlyContinue)) {
    & "$PSScriptRoot\start.ps1"
}
$h = [System.IntPtr]::Zero
for ($i = 0; $i -lt 60; $i++) {
    $h = Get-Pencere
    if ($h -ne [System.IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 500
}
if ($h -ne [System.IntPtr]::Zero) {
    [W.U]::ShowWindow($h, 9) | Out-Null   # SW_RESTORE
    [W.U]::SetForegroundWindow($h) | Out-Null
}
```
Eski "--headless" dalı kalkar: ibnun-nedim'in GUI'siz kipi `BGE_HEADLESS` ortam değişkeniyle seçiliyor, komut satırı bayrağıyla değil (`src/main.rs:850`).

Doğrulama: pencere açıkken `powershell -NoProfile -ExecutionPolicy Bypass -File ~/.llama-embed-server/ac.ps1` çalıştırılır. Pencere öne gelmeli ve yeni süreç başlamamalı: `Get-Process ibnun-nedim` sayısı 1 kalır.

- [ ] **Adım 3: Kur**

1. ibnü'n-Nedim: `cargo build --release`, sonra PowerShell 7'de `./kur.ps1 -DerlemeYok -Durdur`. Beklenen: 10 sn içinde biter, "hizmet yeniden başlatıldı".
2. nazard: `cargo build --release -p nazar-daemon`, sonra ad değiştirme ile kurulum (Genel kısıtlar). Beklenen: günlükte "boru dinleniyor".
3. 15 sn sonra `%LOCALAPPDATA%\nazar\arkaplan-durum.json` var ve `zaman` alanı şimdiden en çok 15 sn eski.

- [ ] **Adım 4: Canlı ölçümler** (her biri çıktısıyla rapor edilir)

1. **OCR saat dışı:** 06:30-24:00 arasında durum dosyasında `"ocr": { "calisiyor": false, "neden": "saat dışı" … }` görülür. 00:00-06:30 arasındaysa beklenen `calisiyor: true` (ya da Nazar engeli).
2. **Duraklatma CPU'yu keser:**
   1. Politikada `[gomme] duraklat_bitis` şimdi + 3600 yapılır (Görev 7 sekmesi ya da elle).
   2. 30 sn beklenir.
   3. Sonraki 30 sn'de `ibnun-nedim` ve `nazard`'ın CPU artışı ölçülür. Beklenen: her biri < 1 sn.
   4. Durum dosyasında `gomme.neden == "elle duraklatıldı"` görülür.
3. **Toplu iş bekler, arama geçer:**
   1. `curl -s -X POST http://127.0.0.1:11434/v1/embeddings -H "Content-Type: application/json" -d '{"model":"bge-m3","input":["bir","iki"]}'` arka planda başlatılır. Duraklatma sürdükçe yanıt gelmez.
   2. Aynı anda `nazar ara --limit 1 asabiyye` anlam düzeyinde sonuç verir (hızlı şerit).
   3. Duraklatma kaldırılır (`duraklat_bitis = 0`). curl en çok ~7 sn içinde yanıtlanır.
4. **Bozuk dosya:**
   1. Politikaya geçersiz `saatler = "gece"` yazılır.
   2. nazard günlüğünde "arkaplan.toml bozuk" satırı ve `bge-err.log`'da "okunamadı" satırı görülür; iş son geçerli politikayla sürer.
   3. Dosya düzeltilir.

- [ ] **Adım 5: Pencere doğrulaması (gerçek pencere, yalnız kendi penceremiz)**

1. Scratchpad'deki `pencere-cek.ps1` ile önce Durum sekmesinin görüntüsü alınır. Sekme adları Türkçe görünmeli ve taşmamalı.
2. "Arka plan" sekmesine gerçek tıklama gerekir. PostMessage tıklamaları egui'de işlenmez (bellek kaydı `feedback_ekran_goruntusu_pencere`). Betiğe bir `-Tikla "x,y"` seçeneği eklenir ve tıklama yalnız şu koşullar tutarsa yapılır:
   - Alt bas-bırak + `SetForegroundWindow(h)` uygulanır.
   - `GetForegroundWindow() == h` ve `WindowFromPoint(hedef) == h` doğrulanır.
   - `SetCursorPos` + `mouse_event(LEFTDOWN/LEFTUP)` ile tıklanır.
   - İmleç eski yerine konur.
3. Koordinat: sekme çubuğu Durum görüntüsünden ölçülür; dördüncü sekme ortası.
4. Arka plan sekmesinin görüntüsü alınır ve Read ile bakılır. Görülmesi gerekenler:
   - iki kart (Toplu gömme, OCR (Nazar)) ve Ortak sınırlar kartı,
   - renkli durum noktaları,
   - "kuyruk N · bugün M belge",
   - harf harf kırılma ya da sağdan taşma yok.
5. **Sekme adları taşarsa:** `src/main.rs`'te pencere en küçük/ilk genişliği 480 → 520 yapılır (`with_inner_size([520.0, 560.0])`, `with_min_inner_size([520.0, 560.0])`), sonra yeniden derlenir, kurulur ve görüntü alınır.
6. Görüntüler işi bitince silinir.

- [ ] **Adım 6: Bellek kaydı ve son commit**

- `C:\Users\buzbe\.claude\projects\C--Users-buzbe\memory\kaynak_tuketimi_ayarlari.md` güncellenir: `ocr_saatleri` kalktı, yerini `%APPDATA%\ibnun-nedim\arkaplan.toml` aldı; Arka plan sekmesi yönetir.
- Görev 10 Adım 5'te pencere boyutu değiştiyse ibnü'n-Nedim'de commit + push edilir.

---

## Öz denetim notları

- **Tasarım belgesi kapsamı:**
  - §1.1 → G1 ve G2; §1.2 → G2 ve G8; §1.3 → G1-G3; §1.4 → G3 (`kullanici_payi`).
  - §2.1 → G4; §2.2 → G5-G7; §2.3 → G6 (adlar) ve G10 (`ac.ps1`).
  - §3.1 ve §3.2 → G8 ve G9; §3.3 → G8 (`durum_dongusu`, atomik yazma G2'de).
  - §4.1:
    - Yok → G2 `dosya_yokken_izin_olcmeden` ve G7 not;
    - Bozuk → G2 `bozuk_dosyada_son_gecerli_kalir`, G7 not, G10 4.4;
    - bayat → G5 `bayat_nazar_durumu`;
    - ölçülemeyen → G1 `olculemeyen_kosul_engel_degil`;
    - sahipsiz → bugünkü davranış, değişmedi.
  - §4.2 → G1-G9 sınamaları + G10 ölçümler.
- **Kapsam dışında bırakılanlar:** Nazar masaüstü ve `nazar_durum` MCP değişmez (belge §3, kapsam dışı).
- **Bilinçli tavanlar (ponytail):**
  - `Kapi::dene` engel olsa da 1 sn ölçer.
  - Arka plan süreç listesi sabittir.
  - Gömme "bugün" sayacı süreç ömrüyle sınırlı. Gün içinde yeniden başlarsa sıfırdan sayar; belge §3.3 bunu kabul ediyor.
