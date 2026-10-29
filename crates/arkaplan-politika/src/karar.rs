//! "Bu iş şimdi çalışabilir mi?" kararı: duraklatma → saat → bellek → CPU → kullanıcı etkinliği.
//! Ölçülemeyen koşul (None) engel sayılmaz.

use crate::{HER_ZAMAN, Politika, SURDURENE_DEK};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsTuru {
    Gomme,
    Ocr,
    /// Görüntü gömme (Nazar `nazar-gorsel`): Qwen3-VL modeli tek başına ~8 GB tutar,
    /// bu yüzden toplu gömmeden ayrı saat/duraklatma ister. bge bu işi koşturmaz.
    Gorsel,
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
                format!(
                    "{:02}:{:02}-{:02}:{:02}",
                    bas / 60,
                    bas % 60,
                    son / 60,
                    son % 60
                )
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
        return Err(Neden::Duraklatildi {
            bitis: a.duraklat_bitis,
        });
    }
    let saat = SaatAraligi::ayristir(&a.saatler).unwrap_or(SaatAraligi::HerZaman);
    if o.gun_dakikasi.is_some_and(|d| !saat.icinde(d)) {
        return Err(Neden::SaatDisi);
    }
    let s = &p.sinirlar;
    if let Some(y) = o
        .bellek
        .filter(|&y| bellek_dar_mi(s.bellek_yuzde, y, bellek_onceki_dar))
    {
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

use crate::Okuyucu;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;

/// Politika kapısı: dosyayı okur, ölçer, karar verir; bellek eşiğinin önceki hâlini ve son nedeni tutar.
pub struct Kapi {
    okuyucu: Okuyucu,
    bellek_dar: AtomicBool,
    son: Mutex<Option<Neden>>,
    /// Son ölçüm; `OLCUM_OMRU` içinde yeniden kullanılır.
    olcum: Mutex<Option<(std::time::Instant, Olcum)>>,
}

/// Ölçüm 1 sn uyur; art arda gelen toplu istekler (ve nazard'ın dilimleri) bunu her seferinde ödemesin.
const OLCUM_OMRU: Duration = Duration::from_secs(5);

impl Kapi {
    pub fn yeni(politika: PathBuf) -> Self {
        Self {
            okuyucu: Okuyucu::yeni(politika),
            bellek_dar: AtomicBool::new(false),
            son: Mutex::new(None),
            olcum: Mutex::new(None),
        }
    }

    /// İş şimdi koşabilir mi. Dosya yoksa (ya da hiç geçerli okunmadıysa) `varsayilan`; o da yoksa
    /// ölçmeden izin. Ölçüm ~1 sn sürer (CPU örneği): çizim ya da async iş parçacığında çağrılmaz.
    /// Son ölçüm 5 sn içindeyse yeniden kullanılır (duraklatma bitişi en çok 5 sn geç görülür).
    pub fn dene(&self, is: IsTuru, varsayilan: Option<&Politika>) -> Result<(), Neden> {
        let p = match (self.okuyucu.etkin(), varsayilan) {
            (Some(p), _) => p,
            (None, Some(v)) => v.clone(),
            (None, None) => {
                *self.son.lock().unwrap_or_else(|e| e.into_inner()) = None;
                return Ok(());
            }
        };
        let o = self.olc();
        let onceki = self.bellek_dar.load(Relaxed);
        self.bellek_dar.store(
            o.bellek
                .is_some_and(|y| bellek_dar_mi(p.sinirlar.bellek_yuzde, y, onceki)),
            Relaxed,
        );
        let r = karar(&p, is, &o, onceki);
        *self.son.lock().unwrap_or_else(|e| e.into_inner()) = r.err();
        r
    }

    fn olc(&self) -> Olcum {
        let mut son = self.olcum.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((t, o)) = *son
            && t.elapsed() < OLCUM_OMRU
        {
            return o;
        }
        let o = crate::sistem::olc(Duration::from_secs(1));
        *son = Some((std::time::Instant::now(), o));
        o
    }

    pub fn son_neden(&self) -> Option<Neden> {
        *self.son.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn okuyucu(&self) -> &Okuyucu {
        &self.okuyucu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Politika, SURDURENE_DEK};

    #[test]
    fn dosya_yokken_izin_olcmeden() {
        let k = Kapi::yeni(std::env::temp_dir().join("arkaplan-politika-hic-yok.toml"));
        let bas = std::time::Instant::now();
        assert_eq!(k.dene(IsTuru::Gomme, None), Ok(()));
        assert!(
            bas.elapsed() < std::time::Duration::from_millis(200),
            "ölçüm (1 sn) yapılmamalı"
        );
        assert_eq!(k.son_neden(), None);
    }

    #[test]
    fn duraklatilmis_dosyada_kapi_keser() {
        let yol = std::env::temp_dir().join(format!(
            "arkaplan-politika-kapi-{}.toml",
            std::process::id()
        ));
        let mut p = Politika::default();
        p.gomme.duraklat_bitis = SURDURENE_DEK;
        crate::yaz(&yol, &p).unwrap();
        let k = Kapi::yeni(yol.clone());
        let beklenen = Neden::Duraklatildi {
            bitis: SURDURENE_DEK,
        };
        assert_eq!(k.dene(IsTuru::Gomme, None), Err(beklenen));
        assert_eq!(k.son_neden(), Some(beklenen));
        let _ = std::fs::remove_file(yol);
    }

    #[test]
    fn ardisik_izinler_olcumu_yinelemez() {
        // Her toplu istek ve nazard'ın her gömme dilimi 1 sn ölçüm bekliyordu (iki kapıdan geçen
        // dilim 2 sn); 5 sn içindeki ikinci soru son ölçümü kullanır.
        let yol = std::env::temp_dir().join(format!(
            "arkaplan-politika-ardisik-{}.toml",
            std::process::id()
        ));
        let mut p = Politika::default();
        p.sinirlar.bellek_yuzde = 100;
        p.sinirlar.cpu_yuzde = 100;
        crate::yaz(&yol, &p).unwrap();
        let k = Kapi::yeni(yol.clone());
        assert_eq!(k.dene(IsTuru::Gomme, None), Ok(()));
        let bas = std::time::Instant::now();
        assert_eq!(k.dene(IsTuru::Gomme, None), Ok(()));
        assert!(
            bas.elapsed() < std::time::Duration::from_millis(500),
            "ikinci soru yeniden ölçtü: {:?}",
            bas.elapsed()
        );
        let _ = std::fs::remove_file(yol);
    }

    fn olcum() -> Olcum {
        Olcum {
            simdi: 1_000,
            gun_dakikasi: Some(12 * 60),
            bellek: Some(50),
            kullanici_cpu: Some(10),
            girdisiz_sn: Some(600),
        }
    }

    #[test]
    fn saat_araligi_ayristirma() {
        assert_eq!(
            SaatAraligi::ayristir("her zaman"),
            Ok(SaatAraligi::HerZaman)
        );
        assert_eq!(
            SaatAraligi::ayristir("00:00-06:30"),
            Ok(SaatAraligi::Aralik { bas: 0, son: 390 })
        );
        assert!(SaatAraligi::ayristir("gece").is_err());
        assert!(SaatAraligi::ayristir("24:00-06:00").is_err());
        assert_eq!(
            SaatAraligi::Aralik { bas: 0, son: 390 }.metin(),
            "00:00-06:30"
        );
    }

    #[test]
    fn ayni_bas_son_reddedilir() {
        assert!(
            SaatAraligi::ayristir("06:30-06:30").is_err(),
            "hiç çalışmayan aralık sessizce kabul edilmez"
        );
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
        assert_eq!(
            karar(&d, IsTuru::Gomme, &olcum(), false),
            Err(Neden::Duraklatildi { bitis: 2_000 })
        );
        assert_eq!(
            karar(&d, IsTuru::Ocr, &olcum(), false),
            Ok(()),
            "duraklatma işe özgü"
        );

        let mut s = p.clone();
        s.ocr.saatler = "00:00-06:30".into();
        assert_eq!(
            karar(&s, IsTuru::Ocr, &olcum(), false),
            Err(Neden::SaatDisi)
        );

        let o = Olcum {
            bellek: Some(93),
            ..olcum()
        };
        assert_eq!(karar(&p, IsTuru::Gomme, &o, false), Err(Neden::Bellek(93)));

        let o = Olcum {
            kullanici_cpu: Some(71),
            ..olcum()
        };
        assert_eq!(karar(&p, IsTuru::Gomme, &o, false), Err(Neden::Cpu(71)));

        let mut k = p.clone();
        k.sinirlar.kullanirken_dur = true;
        let o = Olcum {
            girdisiz_sn: Some(5),
            ..olcum()
        };
        assert_eq!(
            karar(&k, IsTuru::Gomme, &o, false),
            Err(Neden::KullaniciEtkin)
        );
        assert_eq!(
            karar(&p, IsTuru::Gomme, &o, false),
            Ok(()),
            "kapalıyken etkinlik engel değil"
        );
    }

    #[test]
    fn olculemeyen_kosul_engel_degil() {
        let mut p = Politika::default();
        p.ocr.saatler = "00:00-06:30".into();
        p.sinirlar.kullanirken_dur = true;
        let o = Olcum {
            simdi: 1_000,
            gun_dakikasi: None,
            bellek: None,
            kullanici_cpu: None,
            girdisiz_sn: None,
        };
        assert_eq!(karar(&p, IsTuru::Ocr, &o, false), Ok(()));
    }

    #[test]
    fn gorsel_isi_ayri_saat_ve_duraklatma() {
        // Görüntü gömme modeli ~8 GB: toplu gömme duraklatılınca o da dursaydı gece çalıştırmak
        // imkânsız olurdu. Üç iş türü saat ve duraklatma bakımından birbirinden bağımsızdır.
        let mut p = Politika::default();
        p.gomme.duraklat_bitis = 2_000;
        assert_eq!(
            karar(&p, IsTuru::Gorsel, &olcum(), false),
            Ok(()),
            "toplu gömme duraklatması görüntü işini kesmez"
        );
        p.gorsel.saatler = "00:00-06:30".into();
        assert_eq!(
            karar(&p, IsTuru::Gorsel, &olcum(), false),
            Err(Neden::SaatDisi)
        );
        assert_eq!(
            karar(&p, IsTuru::Gomme, &olcum(), false),
            Err(Neden::Duraklatildi { bitis: 2_000 })
        );
        assert_eq!(
            karar(&p, IsTuru::Ocr, &olcum(), false),
            Ok(()),
            "OCR etkilenmez"
        );
    }

    #[test]
    fn bellek_esikli() {
        assert!(bellek_dar_mi(90, 90, false));
        assert!(!bellek_dar_mi(90, 87, false));
        assert!(
            bellek_dar_mi(90, 87, true),
            "%85'in altına inene dek dar kalır"
        );
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
        assert_eq!(
            duraklat_bitisi(Duraklatma::BugunBoyunca, 10_000, Some(82_800)),
            13_600
        );
        assert_eq!(duraklat_bitisi(Duraklatma::BirSaat, 10_000, None), 13_600);
        assert_eq!(
            duraklat_bitisi(Duraklatma::SurdureneDek, 10_000, None),
            SURDURENE_DEK
        );
    }

    #[test]
    fn surdurene_dek_gidis_donus() {
        let mut p = Politika::default();
        p.ocr.duraklat_bitis = SURDURENE_DEK;
        p.gomme.saatler = "22:00-06:30".into();
        p.gorsel.saatler = "00:00-06:30".into();
        let q = Politika::ayristir(&p.metin()).unwrap();
        assert_eq!(q, p);
    }

    #[test]
    fn bozuk_politika_aciklayici_hata() {
        assert!(
            Politika::ayristir("surum = 2")
                .unwrap_err()
                .contains("surum")
        );
        assert!(
            Politika::ayristir("surum = 1\n[ocr]\nsaatler = \"gece\"")
                .unwrap_err()
                .contains("[ocr]")
        );
        assert!(Politika::ayristir("surum = 1\n[sinirlar]\ncpu_yuzde = 0").is_err());
        assert!(Politika::ayristir("bu toml değil").is_err());
    }
}
