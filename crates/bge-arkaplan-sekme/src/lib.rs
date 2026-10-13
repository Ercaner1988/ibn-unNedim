//! Arka plan sekmesi: toplu gömme (bu sunucu) ve OCR (Nazar) için durum, elle duraklatma, saat
//! aralığı ve ortak sınırlar. Politika `arkaplan.toml`'a anında yazılır (arkaplan-politika).

mod mantik;
pub use mantik::*;
