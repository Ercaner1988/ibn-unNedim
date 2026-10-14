//! Toplu istekler için arka plan politikası (Arka plan sekmesi; `arkaplan-politika`). Hızlı şerit
//! (tek, kısa istek: aramalar, nazard yoklaması) buraya hiç uğramaz: duraklatma aramayı yavaşlatmaz.

pub use arkaplan_politika::Kapi;
use std::sync::LazyLock;

pub static KAPI: LazyLock<Kapi> = LazyLock::new(|| Kapi::yeni(arkaplan_politika::politika_yolu()));

/// Toplu gömme şimdi geçebilir mi (dosya yoksa evet). Engelleyicidir (CPU örneği): `spawn_blocking`
/// içinde çağrılır; bekleme döngüsü çağıranda async kurulur ki istemci vazgeçince bitsin.
pub fn gecer(kapi: &Kapi) -> bool {
    kapi.dene(arkaplan_politika::IsTuru::Gomme, None).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arkaplan_politika::{Politika, SURDURENE_DEK, yaz};
    use std::time::Duration;

    #[test]
    fn duraklatilmisken_gecmez_surdurulunce_gecer() {
        let yol =
            std::env::temp_dir().join(format!("bge-politika-kapi-{}.toml", std::process::id()));
        let mut p = Politika::default();
        p.gomme.duraklat_bitis = SURDURENE_DEK;
        p.sinirlar.bellek_yuzde = 100;
        p.sinirlar.cpu_yuzde = 100; // sınama makinesinin yükü kararı bozmasın
        yaz(&yol, &p).unwrap();
        let kapi = Kapi::yeni(yol.clone());
        assert!(!gecer(&kapi), "duraklatılmışken toplu iş geçmemeli");
        p.gomme.duraklat_bitis = 0;
        yaz(&yol, &p).unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(&yol).unwrap();
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(5))
            .unwrap();
        assert!(gecer(&kapi), "sürdürülünce geçmeli");
        let _ = std::fs::remove_file(yol);
    }
}
