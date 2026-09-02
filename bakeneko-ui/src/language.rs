//! Normalización común de los locales que entrega Futon/Kotatsu.
//!
//! Futon expone etiquetas similares a BCP-47 (`es-419`, `pt-BR`, `zh-Hans`).
//! Para filtrar agrupamos por idioma base, de modo que pequeñas diferencias de
//! región o escritura no dupliquen un idioma en la interfaz.

/// Devuelve la clave canónica usada por todos los filtros de la aplicación.
pub fn key(locale: Option<&str>) -> &'static str {
    let raw = locale.unwrap_or("").trim();
    if raw.is_empty() { return "mixed"; }

    let normalized = raw.to_ascii_lowercase().replace('_', "-");
    let primary = normalized.split('-').next().unwrap_or("");
    match primary {
        "es" | "spa" => "es",
        "en" | "eng" => "en",
        "pt" | "por" | "br" => "pt",
        "fr" | "fra" | "fre" => "fr",
        "de" | "deu" | "ger" => "de",
        "it" | "ita" => "it",
        "ja" | "jpn" | "jp" => "ja",
        "ko" | "kor" | "kr" => "ko",
        "zh" | "zho" | "chi" | "cn" => "zh",
        "ru" | "rus" => "ru",
        "id" | "ind" => "id",
        "vi" | "vie" => "vi",
        "th" | "tha" => "th",
        "sl" | "slv" => "sl",
        "tr" | "tur" => "tr",
        "pl" | "pol" => "pl",
        "nl" | "nld" | "dut" => "nl",
        "ar" | "ara" => "ar",
        "hi" | "hin" => "hi",
        "ms" | "msa" | "may" => "ms",
        "ca" | "cat" => "ca",
        "ro" | "ron" | "rum" => "ro",
        "uk" | "ukr" => "uk",
        "hu" | "hun" => "hu",
        "sv" | "swe" => "sv",
        "no" | "nor" | "nb" | "nn" => "no",
        "da" | "dan" => "da",
        "fi" | "fin" => "fi",
        "cs" | "ces" | "cze" => "cs",
        "el" | "ell" | "gre" => "el",
        "he" | "heb" | "iw" => "he",
        "fa" | "fas" | "per" => "fa",
        "bn" | "ben" => "bn",
        "tl" | "fil" => "fil",
        _ => "other",
    }
}

/// Nombre localizado para mostrar en chips, tarjetas y capítulos.
pub fn label(locale: Option<&str>) -> &'static str {
    label_for_key(key(locale))
}

pub fn label_for_key(language: &str) -> &'static str {
    match language {
        "es" => "Español", "en" => "Inglés", "pt" => "Portugués",
        "fr" => "Francés", "de" => "Alemán", "it" => "Italiano",
        "ja" => "Japonés", "ko" => "Coreano", "zh" => "Chino",
        "ru" => "Ruso", "id" => "Indonesio", "vi" => "Vietnamita",
        "th" => "Tailandés", "sl" => "Esloveno", "tr" => "Turco",
        "pl" => "Polaco", "nl" => "Neerlandés", "ar" => "Árabe",
        "hi" => "Hindi", "ms" => "Malayo", "ca" => "Catalán",
        "ro" => "Rumano", "uk" => "Ucraniano", "hu" => "Húngaro",
        "sv" => "Sueco", "no" => "Noruego", "da" => "Danés",
        "fi" => "Finés", "cs" => "Checo", "el" => "Griego",
        "he" => "Hebreo", "fa" => "Persa", "bn" => "Bengalí",
        "fil" => "Filipino", "mixed" => "Idioma mixto",
        _ => "Otro idioma",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_futon_and_legacy_locales() {
        assert_eq!(key(Some("es-419")), "es");
        assert_eq!(key(Some("pt_BR")), "pt");
        assert_eq!(key(Some("zh-Hans")), "zh");
        assert_eq!(key(Some("ENG")), "en");
        assert_eq!(key(None), "mixed");
        assert_eq!(label(Some("sl-SI")), "Esloveno");
    }
}
