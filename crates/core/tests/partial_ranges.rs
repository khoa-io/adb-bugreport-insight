use bugreport_insight_core::{Archive, EntryId};
use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};

#[test]
fn partial_reads_keep_eof_content_without_relaxing_strict_reads_or_limits() {
    let path = scratch_dir().join("core-partial-ranges.zip");
    std::fs::write(&path, ZipBuilder::new().add("text", b"one\ntwo\n").finish()).unwrap();
    let archive = Archive::open(path).unwrap();
    let mut reader = archive.reader(EntryId(0)).unwrap();
    let (lines, past_end) = reader.read_line_span_partial(2, u64::MAX).unwrap();
    assert!(past_end);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].line_no, 2);
    let (bytes, past_end) = reader.read_byte_range_partial(6, u64::MAX).unwrap();
    assert!(past_end);
    assert_eq!(bytes, b"o\n");
    assert!(reader.read_line_span(2, 3).is_err());
    assert!(reader.read_byte_range(6, 3).is_err());
    assert!(reader.read_line_span_partial(0, 1).is_err());
    for bytes in [false, true] {
        let mut reader = archive.reader(EntryId(0)).unwrap();
        reader.take_bounded(4);
        let result = if bytes {
            reader.read_byte_range_partial(6, 100).map(|_| ())
        } else {
            reader.read_line_span_partial(2, 100).map(|_| ())
        };
        assert!(
            result.is_err(),
            "EOF policy must not hide actual read limits"
        );
    }
}
