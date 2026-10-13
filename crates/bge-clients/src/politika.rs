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

#[cfg(test)]
mod tests {
    use super::*;
    use arkaplan_politika::{Politika, SURDURENE_DEK, yaz};

    #[test]
    fn duraklatilmisken_bekler_surdurulunce_gecer() {
        let yol =
            std::env::temp_dir().join(format!("bge-politika-kapi-{}.toml", std::process::id()));
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
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(5))
            .unwrap();
        let bas = std::time::Instant::now();
        while !is.is_finished() {
            assert!(
                bas.elapsed() < Duration::from_secs(10),
                "sürdürülünce geçmeli"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_file(yol);
    }
}
