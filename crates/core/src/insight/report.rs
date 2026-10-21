//! One bounded read of the main text; all domains share sections and clocks.
use super::{InsightError, common::*};
use crate::{Archive, ArchiveIdentity, ArchiveLimits, EntryId};
use chrono::{Duration, FixedOffset};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RETAINED_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_LINES: usize = 2_000_000;

#[derive(Debug, Clone, Copy)]
struct TextBudget {
    bytes: usize,
    lines: usize,
}
impl TextBudget {
    fn selected(archive: &Archive) -> Self {
        Self::for_limits(archive.limits())
    }
    fn for_limits(limits: ArchiveLimits) -> Self {
        if limits == ArchiveLimits::FORCED {
            Self {
                bytes: 512 * 1024 * 1024,
                lines: 8_000_000,
            }
        } else {
            Self {
                bytes: MAX_RETAINED_BYTES,
                lines: MAX_LINES,
            }
        }
    }
    fn check(self, bytes: usize, lines: usize, complete_line: bool) -> Result<(), InsightError> {
        if !complete_line {
            return Err(InsightError::Invalid(
                "analysis requires a complete retained line (1 MiB line limit)".into(),
            ));
        }
        if bytes > self.bytes {
            return Err(InsightError::LimitExceeded {
                resource: "retained text bytes",
                limit: self.bytes,
            });
        }
        if lines > self.lines {
            return Err(InsightError::LimitExceeded {
                resource: "retained lines",
                limit: self.lines,
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Line {
    pub text: String,
    pub no: u64,
    pub offset: u64,
    pub bytes: u64,
    pub section: Option<usize>,
}
#[derive(Debug)]
pub struct Section {
    pub title: String,
    pub command: Option<String>,
    pub ordinal: usize,
    pub start: usize,
    pub end: usize,
    pub duration: Option<f64>,
    pub end_raw: Option<String>,
    pub status: String,
}
#[derive(Debug, Clone)]
pub struct Log {
    pub index: usize,
    pub raw_time: String,
    pub time: Option<Clock>,
    pub uid_raw: String,
    pub pid: i64,
    pub tid: i64,
    pub priority: String,
    pub tag: String,
    pub message: String,
}
const COUNT_PATTERNS: [&str; 18] = [
    "tombstone",
    "signal ",
    "watchdog",
    "am_proc_start",
    "am_proc_bound",
    "am_proc_died",
    "am_kill",
    "ANR in ",
    "am_anr",
    "FATAL EXCEPTION",
    "am_crash",
    "am_low_memory",
    "am_meminfo",
    "am_pss",
    "Kill '",
    "Long monitor contention",
    "dvm_lock_sample",
    "killinfo",
];
#[derive(Debug, Default)]
struct SideCount {
    hits: usize,
    regions: Vec<Value>,
}
#[derive(Debug)]
pub struct Report {
    pub identity: ArchiveIdentity,
    pub main: Option<EntryId>,
    pub path: String,
    pub lines: Vec<Line>,
    pub sections: Vec<Section>,
    pub logs: Vec<Log>,
    pub properties: BTreeMap<String, (String, usize)>,
    pub property_conflicts: BTreeSet<String>,
    pub headers: BTreeMap<String, (String, usize)>,
    pub offset: Option<FixedOffset>,
    pub capture_time: Option<Clock>,
    pub offset_line: Option<usize>,
    pub first: Option<Clock>,
    pub last: Option<Clock>,
    pub newline_count: Option<u64>,
    text_budget: TextBudget,
    side_counts: BTreeMap<String, SideCount>,
    side_scanned: Vec<EntryId>,
    side_skipped: Vec<EntryId>,
}
impl Report {
    pub fn load(archive: &Archive) -> Result<Self, InsightError> {
        let mut identity = archive.identity(true)?;
        if identity.sha256.is_none() {
            identity = archive.with_fresh_digest().identity(true)?;
        }
        let main = identity
            .main_entry
            .as_deref()
            .and_then(|p| archive.entries().iter().find(|e| e.path == p).map(|e| e.id))
            .or_else(|| {
                archive
                    .entries()
                    .iter()
                    .filter(|e| {
                        e.path
                            .rsplit('/')
                            .next()
                            .is_some_and(|p| p.starts_with("bugreport") && p.ends_with(".txt"))
                    })
                    .max_by_key(|e| e.uncompressed_size)
                    .map(|e| e.id)
            });
        if identity.main_entry.is_some() && main.is_none() {
            return Err(InsightError::Invalid(
                "main_entry.txt names an absent archive member".into(),
            ));
        }
        let path = main
            .and_then(|id| archive.entry(id))
            .map(|e| e.path.clone())
            .unwrap_or_else(|| "-".into());
        let mut r = Self {
            identity,
            main,
            path,
            lines: Vec::new(),
            sections: Vec::new(),
            logs: Vec::new(),
            properties: BTreeMap::new(),
            property_conflicts: BTreeSet::new(),
            headers: BTreeMap::new(),
            offset: None,
            capture_time: None,
            offset_line: None,
            first: None,
            last: None,
            newline_count: None,
            text_budget: TextBudget::selected(archive),
            side_counts: BTreeMap::new(),
            side_scanned: Vec::new(),
            side_skipped: Vec::new(),
        };
        if let Some(id) = main {
            let size = archive.entry(id).map(|e| e.uncompressed_size).unwrap_or(0);
            let mut retained = 0usize;
            let mut newlines = 0u64;
            let mut exceeded = None;
            archive.reader(id)?.for_each_line(|line| {
                if let Err(error) = r.text_budget.check(
                    retained.saturating_add(line.bytes.len()),
                    r.lines.len() + 1,
                    line.bytes.len() as u64 == line.byte_len_full,
                ) {
                    exceeded = Some(error);
                    return Ok(false);
                }
                retained += line.bytes.len();
                if line.byte_offset + line.byte_len_full < size {
                    newlines += 1;
                }
                let text = String::from_utf8_lossy(&line.bytes)
                    .trim_end_matches('\r')
                    .to_string();
                r.lines.push(Line {
                    text,
                    no: line.line_no,
                    offset: line.byte_offset,
                    bytes: line.byte_len_full,
                    section: None,
                });
                Ok(true)
            })?;
            if let Some(error) = exceeded {
                return Err(error);
            }
            r.newline_count = Some(newlines);
        }
        r.index();
        if r.sections.len() >= 20_000 {
            return Err(InsightError::Invalid(
                "section inventory reached its 20,000-header budget".into(),
            ));
        }
        r.clocks();
        r.scan_side_counts(archive)?;
        Ok(r)
    }
    fn scan_side_counts(&mut self, archive: &Archive) -> Result<(), InsightError> {
        let mut region_count = 0usize;
        for (index, line) in self.lines.iter().enumerate() {
            for pattern in COUNT_PATTERNS {
                if !line.text.contains(pattern) {
                    continue;
                }
                let section = line.section.and_then(|s| self.sections.get(s));
                let count = self.side_counts.entry(pattern.into()).or_default();
                count.hits += 1;
                if let Some(last) = count.regions.last_mut()
                    && last["entry_id"].as_u64() == self.main.map(|id| id.0 as u64)
                    && last["line_end"].as_u64() == Some(line.no - 1)
                    && last["section_ordinal"].as_u64() == section.map(|s| s.ordinal as u64)
                {
                    last["line_end"] = json!(line.no);
                    last["raw_count"] = json!(last["raw_count"].as_u64().unwrap_or(0) + 1);
                } else {
                    region_count += 1;
                    if region_count > 50000 {
                        return Err(InsightError::Invalid(
                            "raw-count coverage exceeded 50,000 regions".into(),
                        ));
                    }
                    count.regions.push(json!({"archive_path":self.path,"entry_id":self.main.map(|id|id.0),"section_title":section.map(|s|&s.title),"section_ordinal":section.map(|s|s.ordinal),"line_start":line.no,"line_end":line.no,"raw_count":1,"not_emitted_by_platform":0}));
                }
                let _ = index;
            }
        }
        for entry in archive
            .entries()
            .iter()
            .filter(|e| Some(e.id) != self.main && !e.is_dir)
        {
            let detected = archive.detect(entry.id)?;
            if !detected.is_text || detected.nested.is_some() {
                self.side_skipped.push(entry.id);
                continue;
            }
            self.side_scanned.push(entry.id);
            let mut exceeded = false;
            archive.reader(entry.id)?.for_each_line(|line| {
                if line.bytes.len() as u64 != line.byte_len_full {exceeded=true;return Ok(false);}
                let text=String::from_utf8_lossy(&line.bytes);
                for pattern in COUNT_PATTERNS {
                    if !text.contains(pattern){continue;}
                    let count=self.side_counts.entry(pattern.into()).or_default();count.hits+=1;
                    if let Some(last)=count.regions.last_mut() && last["entry_id"].as_u64()==Some(entry.id.0 as u64)&&last["line_end"].as_u64()==Some(line.line_no-1) {
                        last["line_end"]=json!(line.line_no);last["raw_count"]=json!(last["raw_count"].as_u64().unwrap_or(0)+1);
                    }else{
                        region_count+=1;if region_count>50000 {exceeded=true;return Ok(false);}
                        count.regions.push(json!({"archive_path":entry.path,"entry_id":entry.id.0,"section_title":null,"section_ordinal":null,"line_start":line.line_no,"line_end":line.line_no,"raw_count":1,"not_emitted_by_platform":0}));
                    }
                }
                Ok(true)
            })?;
            if exceeded {
                return Err(InsightError::Invalid(
                    "raw-count coverage budget exceeded (50,000 side regions or core line limit)"
                        .into(),
                ));
            }
        }
        Ok(())
    }
    fn index(&mut self) {
        let mut ordinals: BTreeMap<String, usize> = BTreeMap::new();
        let mut current: Option<usize> = None;
        for i in 0..self.lines.len() {
            let text = self.lines[i].text.trim();
            let header = if text.starts_with("------ ")
                && text.ends_with(" ------")
                && !text.contains("was the duration of")
                && text.chars().any(|c| c != '-' && !c.is_whitespace())
            {
                let inside = text.trim_matches('-').trim();
                let (title, command) = inside
                    .split_once(" (")
                    .map(|(a, b)| (a.to_string(), Some(b.trim_end_matches(')').to_string())))
                    .unwrap_or((inside.to_string(), None));
                Some((title, command))
            } else if text.starts_with("DUMP OF SERVICE ") {
                Some((
                    text.trim_end_matches(':')
                        .replace("SERVICE HIGH ", "SERVICE ")
                        .replace("SERVICE CRITICAL ", "SERVICE "),
                    None,
                ))
            } else {
                None
            };
            if let Some((title, command)) = header {
                if let Some(s) = current {
                    self.sections[s].end = i.saturating_sub(1);
                }
                let ordinal = ordinals.entry(title.clone()).or_default();
                if self.sections.len() >= 20_000 {
                    return;
                }
                self.sections.push(Section {
                    title,
                    command,
                    ordinal: *ordinal,
                    start: i,
                    end: self.lines.len().saturating_sub(1),
                    duration: None,
                    end_raw: None,
                    status: "empty".into(),
                });
                *ordinal += 1;
                current = Some(self.sections.len() - 1);
            }
            if let Some(c) = captures(
                r"([0-9.]+)s was the duration of (?:'([^']+)'|dumpsys ([^,]+))(?:, ending at: (.*))?",
                text,
            ) {
                let title = if c[2].is_empty() {
                    format!("DUMP OF SERVICE {}", c[3])
                } else {
                    c[2].clone()
                };
                let found = self.sections.iter().rposition(|s| {
                    s.title == title
                        || s.title.ends_with(&format!(
                            " {}",
                            title.trim_start_matches("DUMP OF SERVICE ")
                        ))
                });
                if let Some(s) = found {
                    self.sections[s].duration =
                        decimal(&c[1]).filter(|v| (0.0..=604800.0).contains(v));
                    self.sections[s].end_raw = normal(&c[4]);
                }
            }
            if let Some(s) = current {
                if self.sections[s].status == "empty"
                    && i > self.sections[s].start
                    && !text.is_empty()
                    && !text.starts_with("------")
                {
                    self.sections[s].status = "ok".into();
                }
                if text.starts_with("Skipping '") && text.contains("on user build") {
                    self.sections[s].status = "skipped_user_build".into();
                } else if text.starts_with("*** Error dumping") {
                    self.sections[s].status = "skipped_missing_source".into();
                } else if (text.starts_with("***") && text.contains("Permission denied"))
                    || text.starts_with("Failed to drop root")
                {
                    self.sections[s].status = "skipped_root_denied".into();
                } else if text.starts_with("***")
                    && (text.contains("timed out") || text.contains("TIMEOUT"))
                {
                    self.sections[s].status = "skipped_budget".into();
                }
            }
            if let Some(c) = captures(r"^\[([^\]]+)\]: \[(.*)\]$", text) {
                if self.properties.get(&c[1]).is_some_and(|(v, _)| v != &c[2]) {
                    self.property_conflicts.insert(c[1].clone());
                }
                self.properties
                    .entry(c[1].clone())
                    .or_insert((c[2].clone(), i));
            }
            if i < 100
                && let Some((key, value)) = text.split_once(": ")
            {
                self.headers
                    .insert(key.to_string(), (value.trim_matches('\'').to_string(), i));
            }
            self.lines[i].section = current;
        }
    }
    fn clocks(&mut self) {
        let banner = self.headers.get("== dumpstate").map(|(v, _)| v.clone());
        self.capture_time = banner.as_deref().and_then(|v| parse_clock(v, None));
        self.offset = self.capture_time.as_ref().map(|t| *t.offset());
        if self.offset.is_none() {
            let date = banner.as_deref().and_then(|b| b.get(..10));
            for (i, line) in self.lines.iter().enumerate() {
                if let Some(c) = captures(r"(\d{4}-\d\d-\d\d [0-9:.]+[+-]\d{4})", &line.text)
                    && date.is_some_and(|d| c[1].starts_with(d))
                    && let Some(t) = parse_clock(&c[1], None)
                {
                    self.offset = Some(*t.offset());
                    self.offset_line = Some(i);
                    break;
                }
            }
            self.capture_time = banner.as_deref().and_then(|b| parse_clock(b, self.offset));
        }
        for (i, line) in self.lines.iter().enumerate() {
            if !line.text.as_bytes().get(2).is_some_and(|b| *b == b'-') {
                continue;
            }
            if let Some(c) = captures(
                r"^(\d\d-\d\d \d\d:\d\d:\d\d\.\d+)\s+(?:(\S+)\s+)?(\d+)\s+(\d+)\s+([VDIWEFAS])\s+([^:]+):\s?(.*)$",
                &line.text,
            ) {
                self.logs.push(Log {
                    index: i,
                    raw_time: c[1].clone(),
                    time: log_clock(&c[1], self.capture_time.as_ref()),
                    uid_raw: c[2].clone(),
                    pid: number(&c[3]).unwrap_or(0),
                    tid: number(&c[4]).unwrap_or(0),
                    priority: c[5].clone(),
                    tag: c[6].trim().to_string(),
                    message: c[7].clone(),
                });
            }
        }
        for log in &self.logs {
            let section = self
                .section(log.index)
                .map(|s| s.title.as_str())
                .unwrap_or("");
            if ["SYSTEM LOG", "EVENT LOG", "RADIO LOG", "STATS LOG"].contains(&section)
                && let Some(t) = log.time
                && self
                    .capture_time
                    .is_some_and(|capture| (capture - t).num_seconds() < 3600)
            {
                self.first = Some(self.first.map_or(t, |old| old.min(t)));
                self.last = Some(self.last.map_or(t, |old| old.max(t)));
            }
        }
    }
    pub fn section(&self, index: usize) -> Option<&Section> {
        self.lines
            .get(index)?
            .section
            .and_then(|s| self.sections.get(s))
    }
    pub fn title(&self, index: usize) -> &str {
        self.section(index).map(|s| s.title.as_str()).unwrap_or("")
    }
    pub fn prop(&self, key: &str) -> Option<String> {
        if self.property_conflicts.contains(key) {
            None
        } else {
            normal(&self.properties.get(key)?.0)
        }
    }
    pub fn header(&self, key: &str) -> Option<String> {
        normal(&self.headers.get(key)?.0)
    }
    pub fn find_section(&self, needle: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.title.contains(needle))
    }
    pub fn section_lines<'a>(
        &'a self,
        needle: &'a str,
    ) -> impl Iterator<Item = (usize, &'a Line)> + 'a {
        self.lines
            .iter()
            .enumerate()
            .filter(move |(i, _)| self.title(*i).contains(needle))
    }
    pub fn evidence(&self, index: usize) -> Value {
        let l = &self.lines[index];
        evidence(&self.path, l.no, l.offset, &l.text, l.bytes)
    }
    pub fn section_evidence(&self, section: &Section) -> Value {
        let mut evidence = self.evidence(section.start);
        let start = &self.lines[section.start];
        let end = &self.lines[section.end];
        let length = end.offset + end.bytes - start.offset;
        evidence["kind"] = json!("section");
        evidence["locator"]["line_end"] = json!(end.no);
        evidence["locator"]["byte_length"] = json!(length);
        let retained = evidence["excerpt_bytes"].as_u64().unwrap_or(0);
        evidence["excerpt_truncated"] = json!(length > retained);
        evidence["excerpt_complete"] = json!(length <= retained && !start.text.contains('\0'));
        evidence
    }
    pub fn section_clock(&self, index: usize) -> Option<Clock> {
        self.section(index)?
            .end_raw
            .as_deref()
            .and_then(|v| parse_clock(v, self.offset))
    }
    pub fn window(&self) -> Value {
        let titles: Vec<_> = self
            .sections
            .iter()
            .filter(|s| {
                [
                    "SYSTEM LOG",
                    "EVENT LOG",
                    "RADIO LOG",
                    "STATS LOG",
                    "LAST LOGCAT",
                ]
                .contains(&s.title.as_str())
            })
            .map(|s| s.title.clone())
            .collect();
        json!({"sections":titles,"first_line_at":self.first.map(|t|t.format("%m-%d %H:%M:%S%.3f").to_string()),"last_line_at":self.last.map(|t|t.format("%m-%d %H:%M:%S%.3f").to_string()),"duration_seconds":self.first.zip(self.last).map(|(a,b)|(b-a).num_milliseconds() as f64/1000.),"is_contiguous":if self.sections.iter().any(|s|s.title=="SYSTEM LOG"){Some(self.sections.iter().filter(|s|s.title=="SYSTEM LOG").count()==1)}else{None},"first_line_minus_boot_seconds":null,"truncated":null})
    }
    pub fn within(&self, time: Option<Clock>) -> bool {
        time.zip(self.first.zip(self.last))
            .is_some_and(|(t, (a, b))| t >= a && t <= b)
    }
    pub fn provenance(
        &self,
        index: usize,
        origin: &str,
        kind: &str,
        time: Option<Clock>,
        time_base: &str,
        time_source: &str,
    ) -> Value {
        let line = self.lines.get(index);
        let sec = line
            .and_then(|l| l.section)
            .and_then(|s| self.sections.get(s));
        let time = time.filter(|_| self.capture_time.is_some());
        let age = time
            .zip(self.capture_time)
            .map(|(a, b)| (b - a).num_seconds().max(0));
        let freshness = match time {
            None => "undated",
            Some(t) if self.within(Some(t)) && age.is_some_and(|a| a < 3600) => {
                if self
                    .capture_time
                    .is_some_and(|c| (c - t).num_milliseconds().abs() < 1000)
                {
                    "now"
                } else {
                    "in_window"
                }
            }
            _ => "stale",
        };
        json!({"evidence_origin":origin,"archive_path":self.path,"section_title":sec.map(|s|&s.title),"section_ordinal":sec.map(|s|s.ordinal),"line_start":line.map(|l|l.no),"line_end":line.map(|l|l.no),"captured_at":time.as_ref().map(stamp),"captured_at_time_base":if time.is_some(){time_base}else{"undetermined"},"evidence_time_source":if time.is_some(){time_source}else{"none"},"age_seconds_at_capture":age,"capture_kind":kind,"freshness":freshness,"freshness_basis":{"input":if time.is_some(){time_base}else{"none"},"basis":if time.is_some(){"Evidence clock compared with the canonical log window and capture clock"}else{"No resolvable evidence/capture clock"},"threshold_seconds":3600},"cross_boot_reference":null,"collector_perturbed":null,"collector_perturbed_basis":if time.is_some(){Some(if kind=="live_log"{"live_log_capture"}else if kind=="disk_artifact"{"read_from_disk"}else{"section_measured_during_dump"})}else{None}})
    }
    pub fn snapshot(&self, index: usize, origin: &str) -> Value {
        self.provenance(
            index,
            origin,
            "point_in_time_snapshot",
            self.section_clock(index),
            "dumpsys_section_snapshot",
            "dumpsys_Timestamp",
        )
    }
    pub fn log_provenance(&self, log: &Log, origin: &str) -> Value {
        self.provenance(
            log.index,
            origin,
            "live_log",
            log.time,
            "logcat_line",
            if log.tag.starts_with("am_") {
                "event_log_entry"
            } else {
                "logcat_line"
            },
        )
    }
    pub fn scope(&self, needles: &[&str], all: bool) -> Value {
        let matched: Vec<_> = self
            .sections
            .iter()
            .filter(|s| all || needles.iter().any(|n| s.title.contains(n)))
            .collect();
        let missing: Vec<_> = needles
            .iter()
            .filter(|n| !self.sections.iter().any(|s| s.title.contains(**n)))
            .copied()
            .collect();
        let mut value = json!({"sections_scanned":self.sections.len(),"sections_matched":matched.len(),"sections_listed":matched.iter().map(|s|&s.title).collect::<Vec<_>>(),"sections_skipped":matched.iter().filter(|s|s.status!="ok"&&s.status!="empty").map(|s|&s.title).collect::<Vec<_>>(),"sections_not_found":missing,"max_lines_read":self.text_budget.lines,"truncated":false});
        if all {
            value["sections"]=json!(self.sections.iter().map(|s|json!({"title":s.title,"ordinal":s.ordinal,"command":s.command,"status":s.status,"start_line":self.lines[s.start].no,"end_line":self.lines[s.end].no,"duration_seconds":s.duration,"header":self.lines[s.start].text})).collect::<Vec<_>>());
        }
        value
    }
    pub fn source(&self, device: Value) -> Value {
        json!({"archive_name":self.identity.basename,"archive_size_bytes":self.identity.size_bytes,"archive_sha256":self.identity.sha256,"format_version":self.identity.format_version,"main_entry":self.identity.main_entry,"main_entry_size_bytes":self.identity.main_entry_size_bytes,"main_entry_line_count":self.newline_count,"zip_entry_count":self.identity.entry_count,"device":device})
    }
    pub fn count(&self, pattern: &str, matched: usize) -> Value {
        let pattern = pattern.strip_suffix(':').unwrap_or(pattern);
        let count = self.side_counts.get(pattern);
        let raw = count.map(|c| c.hits).unwrap_or(0);
        let regions = count.map(|c| c.regions.clone()).unwrap_or_default();
        let suppressed = raw.saturating_sub(matched);
        let mut value = json!({"pattern":pattern,"pattern_kind":"substring","case_sensitive":true,"raw_count":raw,"matched_evidence":matched,"suppressed_noise":suppressed,"not_emitted_by_platform":0,"suppressed_noise_note":if raw>matched {Some("Only records passing the artifact's structural validation and de-duplication rules are promoted.")}else{None},"validation":"Structured record and identity/time/content de-duplication","coverage":{"window_start":self.first.as_ref().map(stamp),"window_end":self.last.as_ref().map(stamp),"archive_window":self.window(),"coverage_ratio":self.first.zip(self.last).map(|_|1.),"entry_ids_scanned":self.main.into_iter().chain(self.side_scanned.iter().copied()).map(|id|id.0).collect::<Vec<_>>(),"entry_ids_excluded_binary_or_nested":self.side_skipped.iter().map(|id|id.0).collect::<Vec<_>>()},"regions":regions});
        if pattern.starts_with("am_") || pattern == "dvm_lock_sample" {
            let times: Vec<_> = self
                .logs
                .iter()
                .filter(|l| self.title(l.index) == "EVENT LOG")
                .filter_map(|l| l.time)
                .filter(|t| {
                    self.capture_time
                        .is_some_and(|c| (c - *t).num_seconds() < 3600)
                })
                .collect();
            let first = times.iter().min().copied();
            let last = times.iter().max().copied();
            let duration = first
                .zip(last)
                .map(|(a, b)| (b - a).num_milliseconds() as f64 / 1000.);
            let full = self
                .first
                .zip(self.last)
                .map(|(a, b)| (b - a).num_milliseconds() as f64 / 1000.);
            value["coverage"]["window_start"] = json!(first.as_ref().map(stamp));
            value["coverage"]["window_end"] = json!(last.as_ref().map(stamp));
            value["coverage"]["coverage_ratio"] = json!(ratio(duration, full));
        }
        value
    }
}

/// Read a side text entry under the same retained-text budget and core guards.
pub fn entry_lines(archive: &Archive, id: EntryId) -> Result<Vec<Line>, InsightError> {
    let mut lines = Vec::new();
    let mut bytes = 0usize;
    let budget = TextBudget::selected(archive);
    let mut exceeded = None;
    archive.reader(id)?.for_each_line(|l| {
        bytes = bytes.saturating_add(l.bytes.len());
        if let Err(error) = budget.check(
            bytes,
            lines.len() + 1,
            l.bytes.len() as u64 == l.byte_len_full,
        ) {
            exceeded = Some(error);
            return Ok(false);
        }
        lines.push(Line {
            text: String::from_utf8_lossy(&l.bytes)
                .trim_end_matches('\r')
                .to_string(),
            no: l.line_no,
            offset: l.byte_offset,
            bytes: l.byte_len_full,
            section: None,
        });
        Ok(true)
    })?;
    if let Some(error) = exceeded {
        return Err(error);
    }
    Ok(lines)
}

pub fn before(time: Clock, seconds: f64) -> Clock {
    time - Duration::milliseconds((seconds * 1000.).round() as i64)
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn only_exact_forced_profile_raises_retained_budgets() {
        let default = TextBudget::for_limits(ArchiveLimits::DEFAULT);
        assert_eq!(
            (default.bytes, default.lines),
            (128 * 1024 * 1024, 2_000_000)
        );
        let forced = TextBudget::for_limits(ArchiveLimits::FORCED);
        assert_eq!((forced.bytes, forced.lines), (512 * 1024 * 1024, 8_000_000));
        let custom = TextBudget::for_limits(ArchiveLimits {
            max_entries: 200_000,
            ..ArchiveLimits::FORCED
        });
        assert_eq!((custom.bytes, custom.lines), (default.bytes, default.lines));
    }

    #[test]
    fn retained_budgets_accept_boundaries_and_return_typed_overflow() {
        let budget = TextBudget { bytes: 8, lines: 2 };
        assert!(budget.check(8, 2, true).is_ok());
        assert!(matches!(
            budget.check(9, 2, true),
            Err(InsightError::LimitExceeded {
                resource: "retained text bytes",
                limit: 8
            })
        ));
        assert!(matches!(
            budget.check(8, 3, true),
            Err(InsightError::LimitExceeded {
                resource: "retained lines",
                limit: 2
            })
        ));
        assert!(matches!(
            budget.check(1, 1, false),
            Err(InsightError::Invalid(_))
        ));
    }
}
