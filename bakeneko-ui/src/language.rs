//! Normalización común de los locales que entrega Futon/Kotatsu.
//!
//! Futon expone etiquetas similares a BCP-47 (`es-419`, `pt-BR`, `zh-Hans`).
//! Para filtrar agrupamos por idioma base, de modo que pequeñas diferencias de
//! región o escritura no dupliquen un idioma en la interfaz.

/// Devuelve la clave canónica usada por todos los filtros de la aplicación.
pub fn key(locale: Option<&str>) -> &'static str {
    let raw = locale.unwrap_or("").trim();
    if raw.is_empty() { return "mixed"; }

    let normalized = raw.to_lowercase().replace('_', "-");
    // MangaDex/Futon guarda el idioma del capítulo en `branch` usando el
    // nombre nativo y añade " (n)" cuando hay ramas duplicadas.
    let without_branch_suffix = normalized.split(" (").next().unwrap_or(&normalized).trim();
    let primary = without_branch_suffix.split('-').next().unwrap_or("");
    match primary {
        "es" | "spa" | "español" | "castellano" => "es",
        "en" | "eng" | "english" => "en",
        "pt" | "por" | "br" | "português" | "portuguese" => "pt",
        "fr" | "fra" | "fre" | "français" | "french" => "fr",
        "de" | "deu" | "ger" | "deutsch" | "german" => "de",
        "it" | "ita" | "italiano" | "italian" => "it",
        "ja" | "jpn" | "jp" | "日本語" | "japanese" => "ja",
        "ko" | "kor" | "kr" | "한국어" | "조선말" | "korean" => "ko",
        "zh" | "zho" | "chi" | "cn" | "中文" | "汉语" | "漢語" | "chinese" => "zh",
        "ru" | "rus" | "русский" | "russian" => "ru",
        "id" | "ind" | "indonesia" | "indonesian" => "id",
        "vi" | "vie" | "tiếng việt" | "vietnamese" => "vi",
        "th" | "tha" | "ไทย" | "thai" => "th",
        "sl" | "slv" | "slovenščina" | "slovenian" => "sl",
        "tr" | "tur" | "türkçe" | "turkish" => "tr",
        "pl" | "pol" | "polski" | "polish" => "pl",
        "nl" | "nld" | "dut" | "nederlands" | "dutch" => "nl",
        "ar" | "ara" | "العربية" | "arabic" => "ar",
        "hi" | "hin" | "हिन्दी" | "हिंदी" | "hindi" => "hi",
        "ms" | "msa" | "may" | "melayu" | "malay" => "ms",
        "ca" | "cat" | "català" | "catalan" => "ca",
        "ro" | "ron" | "rum" | "română" | "romanian" => "ro",
        "uk" | "ukr" | "українська" | "ukrainian" => "uk",
        "hu" | "hun" | "magyar" | "hungarian" => "hu",
        "sv" | "swe" | "svenska" | "swedish" => "sv",
        "no" | "nor" | "nb" | "nn" | "norsk" | "norwegian" => "no",
        "da" | "dan" | "dansk" | "danish" => "da",
        "fi" | "fin" | "suomi" | "finnish" => "fi",
        "cs" | "ces" | "cze" | "čeština" | "czech" => "cs",
        "el" | "ell" | "gre" | "ελληνικά" | "greek" => "el",
        "he" | "heb" | "iw" | "עברית" | "hebrew" => "he",
        "fa" | "fas" | "per" | "فارسی" | "persian" => "fa",
        "bn" | "ben" | "বাংলা" | "bengali" => "bn",
        "tl" | "fil" | "filipino" | "tagalog" => "fil",
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
        assert_eq!(key(Some("English (2)")), "en");
        assert_eq!(key(Some("Español")), "es");
        assert_eq!(key(Some("Українська")), "uk");
        assert_eq!(key(None), "mixed");
        assert_eq!(label(Some("sl-SI")), "Esloveno");
    }
}
