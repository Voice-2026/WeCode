use super::types::AppSettings;

pub fn locale_from_language_setting(language: &str) -> String {
    match language {
        "english" => "en",
        "simplifiedChinese" => "zh-Hans",
        "traditionalChinese" => "zh-Hant",
        "japanese" => "ja",
        "korean" => "ko",
        "french" => "fr",
        "german" => "de",
        "spanish" => "es",
        "portugueseBrazil" => "pt-BR",
        "russian" => "ru",
        _ => locale_from_system_setting(),
    }
    .to_string()
}

pub fn sync_process_locale_preference(settings: &AppSettings) {
    #[cfg(target_os = "macos")]
    {
        macos_sync_process_locale_preference(&settings.language);
    }
}

fn locale_from_system_setting() -> &'static str {
    let locale = std::env::var("LC_ALL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("LANG")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    locale
        .as_deref()
        .map(locale_from_system_locale)
        .unwrap_or("en")
}

fn locale_from_system_locale(locale: &str) -> &'static str {
    let normalized = locale.replace('_', "-").to_lowercase();
    if normalized.starts_with("zh-tw")
        || normalized.starts_with("zh-hk")
        || normalized.starts_with("zh-mo")
    {
        return "zh-Hant";
    }
    if normalized.starts_with("zh") {
        return "zh-Hans";
    }
    if normalized.starts_with("ja") {
        return "ja";
    }
    if normalized.starts_with("ko") {
        return "ko";
    }
    if normalized.starts_with("fr") {
        return "fr";
    }
    if normalized.starts_with("de") {
        return "de";
    }
    if normalized.starts_with("es") {
        return "es";
    }
    if normalized.starts_with("pt-br") {
        return "pt-BR";
    }
    if normalized.starts_with("ru") {
        return "ru";
    }
    if normalized.starts_with("en") {
        return "en";
    }
    "en"
}

#[cfg(target_os = "macos")]
fn macos_sync_process_locale_preference(language: &str) {
    use core_foundation_sys::array::{CFArrayCreate, kCFTypeArrayCallBacks};
    use core_foundation_sys::base::{CFRelease, kCFAllocatorDefault};
    use core_foundation_sys::preferences::{
        CFPreferencesAppSynchronize, CFPreferencesSetAppValue, kCFPreferencesCurrentApplication,
    };
    use core_foundation_sys::propertylist::CFPropertyListRef;
    use core_foundation_sys::string::{CFStringCreateWithCString, kCFStringEncodingUTF8};
    use std::ffi::CString;
    use std::os::raw::c_void;
    use std::ptr;

    let key = CString::new("AppleLanguages").expect("static string contains no nul");
    let key_ref = unsafe {
        CFStringCreateWithCString(kCFAllocatorDefault, key.as_ptr(), kCFStringEncodingUTF8)
    };
    if key_ref.is_null() {
        return;
    }

    unsafe {
        if language == "system" {
            CFPreferencesSetAppValue(
                key_ref,
                ptr::null::<c_void>() as CFPropertyListRef,
                kCFPreferencesCurrentApplication,
            );
            let _ = CFPreferencesAppSynchronize(kCFPreferencesCurrentApplication);
            CFRelease(key_ref.cast());
            return;
        }
    }

    let locale = locale_from_language_setting(language);
    let locale = CString::new(locale).unwrap_or_else(|_| CString::new("en").unwrap());
    let locale_ref = unsafe {
        CFStringCreateWithCString(kCFAllocatorDefault, locale.as_ptr(), kCFStringEncodingUTF8)
    };
    if locale_ref.is_null() {
        unsafe {
            CFRelease(key_ref.cast());
        }
        return;
    }

    let values = [locale_ref.cast::<c_void>()];
    let languages_ref = unsafe {
        CFArrayCreate(
            kCFAllocatorDefault,
            values.as_ptr(),
            values.len() as isize,
            &kCFTypeArrayCallBacks,
        )
    };

    unsafe {
        if !languages_ref.is_null() {
            CFPreferencesSetAppValue(
                key_ref,
                languages_ref.cast(),
                kCFPreferencesCurrentApplication,
            );
            let _ = CFPreferencesAppSynchronize(kCFPreferencesCurrentApplication);
            CFRelease(languages_ref.cast());
        }
        CFRelease(locale_ref.cast());
        CFRelease(key_ref.cast());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_language_settings_map_to_supported_locales() {
        assert_eq!(locale_from_language_setting("simplifiedChinese"), "zh-Hans");
        assert_eq!(
            locale_from_language_setting("traditionalChinese"),
            "zh-Hant"
        );
        assert_eq!(locale_from_language_setting("portugueseBrazil"), "pt-BR");
    }

    #[test]
    fn system_locale_mapping_matches_frontend_locale_mapping() {
        assert_eq!(locale_from_system_locale("zh_CN"), "zh-Hans");
        assert_eq!(locale_from_system_locale("zh-Hans-CN"), "zh-Hans");
        assert_eq!(locale_from_system_locale("zh_TW"), "zh-Hant");
        assert_eq!(locale_from_system_locale("pt_BR"), "pt-BR");
        assert_eq!(locale_from_system_locale("en_US"), "en");
    }
}
