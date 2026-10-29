//! Sekmenin arayüzsüz mantığı: düğme → politikaya yazılan değer, satır durumu metinleri.

use arkaplan_politika::{
    ArkaplanDurumu, Duraklatma, IsTuru, Neden, Politika, SURDURENE_DEK, SaatAraligi,
    duraklat_bitisi,
};

/// Nazar durum dosyası bundan eskiyse nazard yanıt vermiyor sayılır.
pub const BAYAT_SN: i64 = 60;
/// "her zaman" kapatılınca önerilen aralık.
pub const VARSAYILAN_ARALIK: SaatAraligi = SaatAraligi::Aralik { bas: 0, son: 390 };

/// Sekmedeki saat satırı sırası; bu sekmede satırı olmayan iş türü (`Gorsel`, görüntü gömme)
/// `None` — bge görüntü işi koşturmaz, Nazar'ın kendi arka plan sekmesi yoktur.
pub fn sira(is: IsTuru) -> Option<usize> {
    match is {
        IsTuru::Gomme => Some(0),
        IsTuru::Ocr => Some(1),
        IsTuru::Gorsel => None,
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
    SatirDurumu {
        tur,
        metin: metin.into(),
    }
}

fn duraklatma_satiri(
    p: &Politika,
    is: IsTuru,
    simdi: i64,
    gun_sn: Option<u32>,
) -> Option<SatirDurumu> {
    duraklatildi_mi(p, is, simdi).then(|| {
        satir(
            Tur::Duraklatildi,
            format!(
                "duraklatıldı · {}",
                bitis_metni(p.is(is).duraklat_bitis, simdi, gun_sn)
            ),
        )
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

pub fn ocr_durumu(
    p: &Politika,
    simdi: i64,
    gun_sn: Option<u32>,
    nazar: Option<&ArkaplanDurumu>,
) -> SatirDurumu {
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
pub fn bugun_farki(taban: &mut (Option<u32>, u64), tarih: Option<u32>, toplam: u64) -> u64 {
    if taban.0 != tarih {
        *taban = (tarih, toplam);
    }
    toplam.saturating_sub(taban.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arkaplan_politika::{IsDurumu, Politika};

    #[test]
    fn duraklat_ve_surdur() {
        let mut p = Politika::default();
        duraklat(&mut p, IsTuru::Ocr, Duraklatma::BirSaat, 1_000, None);
        assert!(
            duraklatildi_mi(&p, IsTuru::Ocr, 1_000) && !duraklatildi_mi(&p, IsTuru::Gomme, 1_000)
        );
        assert!(
            !duraklatildi_mi(&p, IsTuru::Ocr, 4_600),
            "süre dolunca kendiliğinden kalkar"
        );
        surdur(&mut p, IsTuru::Ocr);
        assert!(!duraklatildi_mi(&p, IsTuru::Ocr, 1_000));
    }

    #[test]
    fn bitis_yerel_saatle() {
        // Şimdi 15:00 (54000 sn), bitiş 40 dk sonra → 15:40.
        assert_eq!(bitis_metni(2_400 + 100, 100, Some(54_000)), "bitiş 15:40");
        assert_eq!(
            bitis_metni(SURDURENE_DEK, 100, Some(54_000)),
            "sürdürene dek"
        );
        assert_eq!(bitis_metni(100 + 600, 100, None), "10 dk daha");
    }

    #[test]
    fn gomme_satiri() {
        let p = Politika::default();
        let d = gomme_durumu(&p, 0, None, Some(Neden::SaatDisi), true, false);
        assert_eq!((d.tur, d.metin.as_str()), (Tur::Bekliyor, "saat dışı"));
        assert_eq!(
            gomme_durumu(&p, 0, None, None, false, true).tur,
            Tur::Calisiyor
        );
        assert_eq!(
            gomme_durumu(&p, 0, None, None, false, false).tur,
            Tur::Bosta
        );
        let mut q = p.clone();
        q.gomme.duraklat_bitis = SURDURENE_DEK;
        let d = gomme_durumu(&q, 0, None, None, true, false);
        assert_eq!(
            (d.tur, d.metin.as_str()),
            (Tur::Duraklatildi, "duraklatıldı · sürdürene dek")
        );
    }

    #[test]
    fn bayat_nazar_durumu() {
        let p = Politika::default();
        let d = ArkaplanDurumu {
            zaman: 0,
            ocr_kuyrugu: 3,
            ..Default::default()
        };
        let s = ocr_durumu(&p, BAYAT_SN + 1, None, Some(&d));
        assert_eq!(
            (s.tur, s.metin.as_str()),
            (Tur::Hata, "nazard yanıt vermiyor")
        );
        assert_eq!(ocr_durumu(&p, 0, None, None).tur, Tur::Hata);
        let d = ArkaplanDurumu {
            zaman: 0,
            ocr_kuyrugu: 3,
            ocr: IsDurumu {
                neden: Some("saat dışı".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(ocr_durumu(&p, 10, None, Some(&d)).metin, "saat dışı");
        let d = ArkaplanDurumu {
            zaman: 0,
            ocr_kuyrugu: 0,
            ..Default::default()
        };
        assert_eq!(ocr_durumu(&p, 10, None, Some(&d)).tur, Tur::Bosta);
    }

    #[test]
    fn bugun_gece_yarisi_sifirlanir() {
        let mut taban = (Some(20261013), 0);
        assert_eq!(bugun_farki(&mut taban, Some(20261013), 40), 40);
        assert_eq!(bugun_farki(&mut taban, Some(20261014), 55), 0, "yeni gün");
        assert_eq!(bugun_farki(&mut taban, Some(20261014), 60), 5);
    }

    #[test]
    fn saat_ayari_yazilir() {
        let mut p = Politika::default();
        saat_ayarla(&mut p, IsTuru::Gomme, VARSAYILAN_ARALIK);
        assert_eq!(p.gomme.saatler, "00:00-06:30");
    }
}
