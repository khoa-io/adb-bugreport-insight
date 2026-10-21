use bugreport_insight_core::{Archive, EntryId, PatternMode, ScanRequest, ScanSession, ScanStep};
use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};

#[test]
fn byte_budget_resumes_utf8_and_unterminated_lines_without_replay() {
    let path = scratch_dir().join("scan-pages-utf8.zip");
    let text = "café\nother\ncafé";
    std::fs::write(
        &path,
        ZipBuilder::new().add("text", text.as_bytes()).finish(),
    )
    .unwrap();
    let archive = Archive::open(path).unwrap();
    let mut scan = ScanSession::new(
        &archive,
        ScanRequest::new("café", PatternMode::Literal).with_scope(vec![EntryId(0), EntryId(0)]),
    )
    .unwrap();
    let mut hits = Vec::new();
    for _ in 0..100 {
        let before = scan.total_scanned_bytes;
        let mut budget = 1;
        while let ScanStep::Match(m) = scan.next_match(&mut budget).unwrap() {
            hits.push((m.line, m.byte_offset, m.excerpt.text));
        }
        assert!(scan.total_scanned_bytes - before <= 1);
        if scan.exhausted() {
            break;
        }
    }
    assert_eq!(hits, vec![(1, 0, "café".into()), (3, 12, "café".into())]);
    assert_eq!(scan.total_scanned_bytes, text.len() as u64);
    assert_eq!(scan.scanned_entries, 1);
    assert!(scan.coverage_complete());
}

#[test]
fn incomplete_lines_never_produce_false_anchored_glob_matches() {
    let path = scratch_dir().join("scan-pages-glob.zip");
    std::fs::write(
        &path,
        ZipBuilder::new().add("text", b"FATALITY\nFATAL\n").finish(),
    )
    .unwrap();
    let archive = Archive::open(path).unwrap();
    let mut scan =
        ScanSession::new(&archive, ScanRequest::new("FATAL", PatternMode::Glob)).unwrap();
    let mut first = 5;
    assert!(matches!(
        scan.next_match(&mut first).unwrap(),
        ScanStep::Paused(_)
    ));
    assert!(!scan.coverage_complete());
    let mut rest = 100;
    let ScanStep::Match(m) = scan.next_match(&mut rest).unwrap() else {
        panic!("expected hit");
    };
    assert_eq!(m.line, 2);
    assert!(matches!(
        scan.next_match(&mut rest).unwrap(),
        ScanStep::Finished
    ));
    assert!(scan.coverage_complete());
}

#[test]
fn overlong_lines_are_explicit_gaps_and_later_lines_remain_searchable() {
    let path = scratch_dir().join("scan-pages-long.zip");
    let mut text = vec![b'x'; bugreport_insight_core::caps::MAX_LINE_BYTES as usize + 1];
    text.extend_from_slice(b"needle\nneedle\n");
    std::fs::write(&path, ZipBuilder::new().add("text", &text).finish()).unwrap();
    let archive = Archive::open(path).unwrap();
    let mut scan =
        ScanSession::new(&archive, ScanRequest::new("needle", PatternMode::Literal)).unwrap();
    let mut hits = Vec::new();
    for _ in 0..100 {
        let before = scan.total_scanned_bytes;
        let mut budget = 16384;
        while let ScanStep::Match(m) = scan.next_match(&mut budget).unwrap() {
            hits.push(m.line);
        }
        assert!(scan.total_scanned_bytes - before <= 16384);
        if scan.exhausted() {
            break;
        }
    }
    assert!(scan.exhausted());
    assert_eq!(hits, vec![2]);
    assert_eq!(scan.skipped_lines, 1);
    assert!(!scan.coverage_complete());
}

#[test]
fn deflated_decoder_survives_pages_and_enforces_cumulative_entry_limit() {
    use std::io::{Cursor, Write};
    let path = scratch_dir().join("scan-pages-deflated.zip");
    let text = (0..900)
        .map(|i| format!("{i}: café needle\n"))
        .collect::<String>();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "text",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(text.as_bytes()).unwrap();
    std::fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
    let archive = Archive::open(path).unwrap();
    let mut scan =
        ScanSession::new(&archive, ScanRequest::new("needle", PatternMode::Literal)).unwrap();
    let mut hits = 0;
    for _ in 0..200 {
        let mut budget = 257;
        while let ScanStep::Match(m) = scan.next_match(&mut budget).unwrap() {
            hits += 1;
            assert_eq!(m.line, hits);
        }
        if scan.exhausted() {
            break;
        }
    }
    assert_eq!(hits, 900);
    assert_eq!(scan.total_scanned_bytes, text.len() as u64);
    assert!(scan.coverage_complete());

    let request = ScanRequest::new("needle", PatternMode::Literal).with_budget(
        bugreport_insight_core::ScanBudget {
            max_entry_bytes: 300,
            ..Default::default()
        },
    );
    let mut scan = ScanSession::new(&archive, request).unwrap();
    let mut failed = false;
    for _ in 0..10 {
        let mut budget = 100;
        loop {
            match scan.next_match(&mut budget) {
                Ok(ScanStep::Match(_)) => (),
                Ok(ScanStep::Paused(_)) => break,
                Err(bugreport_insight_core::ArchiveError::LimitExceeded(_)) => {
                    failed = true;
                    break;
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        if failed {
            break;
        }
    }
    assert!(failed, "entry ceilings must not reset at page boundaries");
}
