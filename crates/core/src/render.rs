use crate::{
    error::Error,
    model::{Finding, Manifest, Session},
    text::{capitalize, markdown_escape},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

pub fn render(
    output: &Path,
    input: &Path,
    source: &str,
    findings: &[Finding],
    sessions: &[Session],
) -> Result<(), Error> {
    let insight = output.join("insight");
    let mut counts = BTreeMap::new();
    for finding in findings {
        *counts.entry(finding.severity.clone()).or_insert(0) += 1;
    }
    write_json(insight.join("findings.json"), findings)?;
    let mut ndjson = File::create(insight.join("findings.ndjson"))?;
    for finding in findings {
        serde_json::to_writer(&mut ndjson, finding)?;
        ndjson.write_all(b"\n")?;
    }
    for session in sessions {
        let directory = insight.join(format!("logcat_{}", session.session_index));
        fs::create_dir_all(&directory)?;
        write_json(directory.join("metadata.json"), session)?;
    }
    let index: Vec<_> = sessions
        .iter()
        .map(|session| {
            serde_json::json!({
                "session_index": session.session_index,
                "artifact": format!("logcat_{}/metadata.json", session.session_index),
                "time_range": session.time_range,
            })
        })
        .collect();
    write_json(
        insight.join("sessions.json"),
        &serde_json::json!({"schema_version": 1, "sessions": index}),
    )?;
    let manifest = Manifest {
        schema_version: 1,
        tool_version: env!("CARGO_PKG_VERSION").into(),
        input_archive: input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        primary_source: source.into(),
        artifacts: vec![
            "README.md".into(),
            "manifest.json".into(),
            "sessions.json".into(),
            "findings.json".into(),
            "findings.ndjson".into(),
        ],
        logcat_sessions: sessions.len(),
        observed_reboot_boundaries: sessions.len().saturating_sub(1),
        severity_counts: counts.clone(),
    };
    write_json(insight.join("manifest.json"), &manifest)?;
    let mut markdown = format!(
        "# Bugreport insight\n\n## Summary\n\n- Source: `{source}`\n- Findings: {} high, {} medium, {} informational\n- Logcat sessions: {}; observed reboot boundaries: {}\n- Analysis caveat: Findings are evidence from a captured snapshot, not proof of causality.\n\n## Findings\n",
        counts.get("high").unwrap_or(&0),
        counts.get("medium").unwrap_or(&0),
        counts.get("informational").unwrap_or(&0),
        sessions.len(),
        sessions.len().saturating_sub(1)
    );
    for finding in findings {
        markdown.push_str(&format!(
            "\n### {} — {}\n\n{}\n\n**Evidence**\n\n",
            capitalize(&finding.severity),
            finding.title,
            finding.summary
        ));
        for evidence in &finding.evidence {
            markdown.push_str(&format!(
                "- `{}:{}-{}` — `{}`\n",
                evidence.source,
                evidence.line_start,
                evidence.line_end,
                markdown_escape(&evidence.excerpt)
            ));
        }
        if !finding.metadata.is_empty() {
            let metadata = finding
                .metadata
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(", ");
            markdown.push_str(&format!("\n_Metadata: {}_\n", markdown_escape(&metadata)));
        }
    }
    markdown.push_str("\n## Logcat sessions\n\n| Session | Metadata |\n| --- | --- |\n");
    for session in sessions {
        markdown.push_str(&format!(
            "| `logcat_{}` | [`metadata.json`](logcat_{}/metadata.json) |\n",
            session.session_index, session.session_index
        ));
    }
    markdown.push_str("\n## Generated artifacts\n\n- `manifest.json`\n- `sessions.json`\n- `findings.json`\n- `findings.ndjson`\n");
    fs::write(insight.join("README.md"), markdown)?;
    Ok(())
}

fn write_json(path: PathBuf, value: &(impl Serialize + ?Sized)) -> Result<(), Error> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, value)?;
    Ok(())
}
