use std::collections::HashSet;

#[test]
fn all_locale_files_have_same_keys() {
    let locales = ["en", "es", "ja", "ko", "ru", "zh-CN"];
    let mut keys: Option<HashSet<String>> = None;
    for locale in &locales {
        let path = format!("locales/{}.yml", locale);
        let content = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {}", path));
        let map: serde_yaml::Mapping =
            serde_yaml::from_str(&content).unwrap_or_else(|_| panic!("parse {}", path));
        let current: HashSet<String> = map
            .keys()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
        match keys {
            None => keys = Some(current),
            Some(ref expected) => {
                let missing: Vec<_> = expected.difference(&current).collect();
                let extra: Vec<_> = current.difference(expected).collect();
                assert!(missing.is_empty(), "{} missing keys {:?}", locale, missing);
                assert!(extra.is_empty(), "{} extra keys {:?}", locale, extra);
            }
        }
    }
}
