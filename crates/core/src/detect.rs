//! Sample-based content detection. v1 **detects and reports**; it never opens
//! a nested archive — that is `DRAFT-REQ-GEN-03`.

use serde::{Deserialize, Serialize};

use crate::identity::ArchiveNote;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContentType {
    Text,
    Json,
    Xml,
    Html,
    Csv,
    ImagePng,
    ImageJpeg,
    ImageGif,
    ImageWebp,
    ImageBmp,
    ArchiveZip,
    ArchiveGzip,
    ArchiveBzip2,
    ArchiveXz,
    ArchiveSevenZ,
    Binary,
    Unknown,
}

/// How the classification was reached.
///
/// `None` is the **only** basis legal with [`ContentType::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DetectionBasis {
    MagicBytes,
    ZipEntryName,
    UniformExtension,
    None,
}

/// A sample-based classification, and it says so. Never a bare confident label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionReport {
    pub content_type: ContentType,
    pub basis: DetectionBasis,
    /// How much was actually inspected — always 8 KiB unless the entry is
    /// smaller. A classification of a 256 MiB entry from an 8 KiB sample is a
    /// sample, and this field is what makes that visible.
    pub sample_bytes: u64,
    pub entry_size: u64,
    pub is_text: bool,
    pub contains_nul: bool,
    pub nested: Option<NestedKind>,
    pub notes: Vec<ArchiveNote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NestedKind {
    Zip,
    Gzip,
    Bzip2,
    Xz,
    SevenZ,
}

/// Nested-archive magic on the *decompressed* sample.
pub fn nested_archive_magic(sample: &[u8]) -> Option<NestedKind> {
    if sample.starts_with(b"PK\x03\x04") {
        Some(NestedKind::Zip)
    } else if sample.starts_with(&[0x1f, 0x8b]) {
        Some(NestedKind::Gzip)
    } else if sample.starts_with(b"BZh") {
        Some(NestedKind::Bzip2)
    } else if sample.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        Some(NestedKind::Xz)
    } else if sample.starts_with(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]) {
        Some(NestedKind::SevenZ)
    } else {
        None
    }
}

fn magic_type(sample: &[u8]) -> Option<ContentType> {
    if sample.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(ContentType::ImagePng)
    } else if sample.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(ContentType::ImageJpeg)
    } else if sample.starts_with(b"GIF87a") || sample.starts_with(b"GIF89a") {
        Some(ContentType::ImageGif)
    } else if sample.len() >= 12 && sample.starts_with(b"RIFF") && &sample[8..12] == b"WEBP" {
        Some(ContentType::ImageWebp)
    } else if sample.starts_with(b"BM") {
        Some(ContentType::ImageBmp)
    } else if sample.starts_with(b"PK\x03\x04") {
        Some(ContentType::ArchiveZip)
    } else if sample.starts_with(&[0x1f, 0x8b]) {
        Some(ContentType::ArchiveGzip)
    } else if sample.starts_with(b"BZh") {
        Some(ContentType::ArchiveBzip2)
    } else if sample.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        Some(ContentType::ArchiveXz)
    } else if sample.starts_with(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]) {
        Some(ContentType::ArchiveSevenZ)
    } else {
        None
    }
}

fn extension_of(name: &str) -> Option<String> {
    let tail = name.rsplit('/').next()?;
    let (_, ext) = tail.rsplit_once('.')?;
    if ext.is_empty() {
        None
    } else {
        Some(ext.to_ascii_lowercase())
    }
}

fn extension_type(ext: &str) -> Option<ContentType> {
    match ext {
        "txt" | "log" | "text" | "md" | "prop" | "rc" | "conf" => Some(ContentType::Text),
        "json" => Some(ContentType::Json),
        "xml" => Some(ContentType::Xml),
        "html" | "htm" => Some(ContentType::Html),
        "csv" => Some(ContentType::Csv),
        "png" => Some(ContentType::ImagePng),
        "jpg" | "jpeg" => Some(ContentType::ImageJpeg),
        "gif" => Some(ContentType::ImageGif),
        "webp" => Some(ContentType::ImageWebp),
        "bmp" => Some(ContentType::ImageBmp),
        "zip" => Some(ContentType::ArchiveZip),
        "gz" => Some(ContentType::ArchiveGzip),
        "bz2" => Some(ContentType::ArchiveBzip2),
        "xz" => Some(ContentType::ArchiveXz),
        "7z" => Some(ContentType::ArchiveSevenZ),
        _ => None,
    }
}

/// A NUL byte in the sample is the strongest binary signal available, and the
/// reference archive's main entry has four of them.
fn looks_textual(sample: &[u8]) -> bool {
    if sample.is_empty() {
        return false;
    }
    if sample.contains(&0) {
        return false;
    }
    std::str::from_utf8(sample).is_ok()
}

pub fn detect(sample: &[u8], entry_name: &str, entry_size: u64) -> DetectionReport {
    let sample_bytes = sample.len() as u64;
    let contains_nul = sample.contains(&0);
    let nested = nested_archive_magic(sample);
    let mut notes = Vec::new();
    if contains_nul {
        notes.push(ArchiveNote::ContainsNul);
    }

    // Magic bytes beat any name: content is evidence, metadata is a claim.
    if let Some(t) = magic_type(sample) {
        return DetectionReport {
            content_type: t,
            basis: DetectionBasis::MagicBytes,
            sample_bytes,
            entry_size,
            is_text: false,
            contains_nul,
            nested,
            notes,
        };
    }

    // A `.zip` name with no PK magic: reported as a name-derived claim, not a
    // content-derived one.
    if let Some(ext) = extension_of(entry_name) {
        if ext == "zip" {
            return DetectionReport {
                content_type: ContentType::ArchiveZip,
                basis: DetectionBasis::ZipEntryName,
                sample_bytes,
                entry_size,
                is_text: false,
                contains_nul,
                nested,
                notes,
            };
        }
        if let Some(t) = extension_type(&ext) {
            let text_ish = matches!(
                t,
                ContentType::Text
                    | ContentType::Json
                    | ContentType::Xml
                    | ContentType::Html
                    | ContentType::Csv
            );
            // Bytes beat a name. A `.txt` extension is a claim; NUL bytes are
            // evidence of binary content, and evidence wins.
            if text_ish && (contains_nul || !looks_textual(sample)) {
                return DetectionReport {
                    content_type: ContentType::Binary,
                    basis: DetectionBasis::MagicBytes,
                    sample_bytes,
                    entry_size,
                    is_text: false,
                    contains_nul,
                    nested,
                    notes,
                };
            }
            return DetectionReport {
                content_type: t,
                basis: DetectionBasis::UniformExtension,
                sample_bytes,
                entry_size,
                is_text: text_ish,
                contains_nul,
                nested,
                notes,
            };
        }
    }

    if looks_textual(sample) {
        return DetectionReport {
            content_type: ContentType::Text,
            basis: DetectionBasis::MagicBytes,
            sample_bytes,
            entry_size,
            is_text: true,
            contains_nul,
            nested,
            notes,
        };
    }

    if contains_nul || !sample.is_empty() {
        return DetectionReport {
            content_type: ContentType::Binary,
            basis: DetectionBasis::MagicBytes,
            sample_bytes,
            entry_size,
            is_text: false,
            contains_nul,
            nested,
            notes,
        };
    }

    // Empty sample with no usable name: nothing was determined.
    DetectionReport {
        content_type: ContentType::Unknown,
        basis: DetectionBasis::None,
        sample_bytes: 0,
        entry_size,
        is_text: false,
        contains_nul: false,
        nested: None,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_beats_name() {
        // A .txt name over PNG bytes is reported as PNG, on magic evidence.
        let r = detect(b"\x89PNG\r\n\x1a\n rest", "a.txt", 100);
        assert_eq!(r.content_type, ContentType::ImagePng);
        assert_eq!(r.basis, DetectionBasis::MagicBytes);
    }

    #[test]
    fn nested_archives_are_detected_not_opened() {
        assert_eq!(
            nested_archive_magic(b"\x1f\x8b\x08"),
            Some(NestedKind::Gzip)
        );
        assert_eq!(nested_archive_magic(b"PK\x03\x04"), Some(NestedKind::Zip));
        assert_eq!(nested_archive_magic(b"plain text"), None);
    }

    #[test]
    fn unknown_is_legal_only_with_basis_none() {
        let r = detect(b"", "", 0);
        assert_eq!(r.content_type, ContentType::Unknown);
        assert_eq!(r.basis, DetectionBasis::None);
    }

    #[test]
    fn nul_marks_binary_and_raises_a_note() {
        let r = detect(b"text\0more", "a.txt", 9);
        assert_eq!(r.content_type, ContentType::Binary);
        assert!(r.contains_nul);
        assert!(r.notes.contains(&ArchiveNote::ContainsNul));
    }

    #[test]
    fn sample_size_is_reported_honestly() {
        let r = detect(b"abc", "a.txt", 64 * 1024 * 1024);
        assert_eq!(r.sample_bytes, 3);
        assert_eq!(r.entry_size, 64 * 1024 * 1024);
    }
}
