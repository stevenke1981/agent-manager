use std::{fs, io, path::Path};

pub const BUILTIN_CATEGORIES: &[&str] = &[
    "01-醫療健康",
    "02-法律司法",
    "03-科技資訊",
    "04-金融商業",
    "05-教育學術",
    "06-媒體傳播",
    "07-政府公務",
    "08-藝術文創",
    "09-服務飲食",
    "10-商業策略",
    "11-製造工程",
    "12-醫美時尚",
    "13-宗教靈性",
    "14-爭議灰色行業",
    "15-成人娛樂業",
    "16-犯罪偵查",
    "17-極端組織分析",
    "18-網路地下",
    "19-特殊職業",
    "20-新興職業",
];

pub fn list_categories(agents_root: &Path) -> io::Result<Vec<String>> {
    if !agents_root.exists() {
        return Ok(BUILTIN_CATEGORIES.iter().map(ToString::to_string).collect());
    }
    let mut categories = Vec::new();
    for entry in fs::read_dir(agents_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name
            .get(..2)
            .is_some_and(|prefix| prefix.chars().all(|c| c.is_ascii_digit()))
        {
            categories.push(name);
        }
    }
    categories.sort_by_key(|name| {
        name.split_once('-')
            .and_then(|(prefix, _)| prefix.parse::<u16>().ok())
            .unwrap_or(u16::MAX)
    });
    Ok(categories)
}

pub fn ensure_category(agents_root: &Path, category: &str) -> io::Result<std::path::PathBuf> {
    crate::storage::validate_relative_component(category)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    fs::create_dir_all(agents_root)?;
    let canonical_root = agents_root.canonicalize()?;
    let path = agents_root.join(category);
    fs::create_dir_all(&path)?;
    let canonical_path = path.canonicalize()?;
    if canonical_path.parent() != Some(canonical_root.as_path()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "category path escapes agents root",
        ));
    }
    Ok(path)
}

#[must_use]
pub fn category_label(category: &str) -> &str {
    category
        .split_once('-')
        .map_or(category, |(_, label)| label)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn ensure_category_rejects_portable_traversal_and_separators() {
        let temp = tempdir().unwrap();
        let root = temp.path().join("agents");
        for category in ["", ".", "..", "../outside", "a/b", "a\\b"] {
            assert!(ensure_category(&root, category).is_err(), "{category}");
        }
        assert!(!temp.path().join("outside").exists());
    }
}
