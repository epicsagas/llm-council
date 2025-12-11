use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_runner;

fn find_council_dir() -> Result<PathBuf> {
    // Try current directory first
    let current_dir = env::current_dir()?;
    let council_in_current = current_dir.join(".council");
    if council_in_current.exists() {
        return Ok(council_in_current);
    }

    // Try parent directories (up to 5 levels)
    let mut dir = current_dir.clone();
    for _ in 0..5 {
        let council_dir = dir.join(".council");
        if council_dir.exists() {
            return Ok(council_dir);
        }
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback: use current directory
    Ok(current_dir.join(".council"))
}

pub async fn handle_peer_review(params: Value) -> Result<Value> {
    let title = params["title"]
        .as_str()
        .context("Missing required parameter: title")?;
    let engine_raw = params["engine"]
        .as_str()
        .unwrap_or("claude");
    let engine_trimmed = engine_raw.trim();
    let engine = if engine_trimmed.is_empty() { "claude" } else { engine_trimmed };
    let engine_for_file: String = {
        let sanitized: String = engine
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let cleaned = sanitized.trim_matches('-');
        if cleaned.is_empty() {
            "claude".to_string()
        } else {
            sanitized
        }
    };
    let self_model = params.get("self_model").and_then(|v| v.as_str());

    let council_base = find_council_dir()?;
    let base_dir = council_base.join(title);
    
    if !base_dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory not found: {} (searched from: {})",
            base_dir.display(),
            env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).display()
        ));
    }

    // Find all Stage1 answer files (markdown preferred, JSON for backward compatibility)
    let answer_files: Vec<PathBuf> = fs::read_dir(&base_dir)
        .context(format!("Failed to read directory: {}", base_dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_string_lossy();
            
            if file_name.contains("-answer.md") || file_name.ends_with("answer.md")
                || file_name.contains("-answer.json") || file_name.ends_with("answer.json") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if answer_files.is_empty() {
        return Err(anyhow::anyhow!(
            "No Stage1 answer files found in {}",
            base_dir.display()
        ));
    }

    // Load and parse all answer files, optionally excluding self_model
    let mut answers = Vec::new();
    let mut labels = Vec::new();
    for file_path in answer_files.iter() {
        let content_value = read_stage1_answer(file_path)
            .context(format!("Failed to parse answer file: {}", file_path.display()))?;

        let model_name = content_value
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-model");

        if let Some(self_model_name) = self_model {
            if model_name.eq_ignore_ascii_case(self_model_name) {
                eprintln!(
                    "ℹ️ Skipping self_model '{}' from peer review",
                    self_model_name
                );
                continue;
            }
        }

        answers.push(json!({
            "file": file_path.file_name().unwrap().to_string_lossy(),
            "content": content_value
        }));
    }

    if answers.is_empty() {
        return Err(anyhow::anyhow!(
            "No Stage1 answers available after applying self_model exclusion"
        ));
    }

    // Re-label responses after exclusion to keep labels consecutive
    for (idx, answer) in answers.iter_mut().enumerate() {
        let label = format!("Response {}", char::from(b'A' + idx as u8));
        labels.push(label.clone());
        answer["label"] = json!(label);
    }

    // Build review prompt
    let user_query = extract_user_query(&base_dir)?;
    
    let responses_text = answers
        .iter()
        .map(|a| {
            format!(
                "{}:\n{}",
                a["label"].as_str().unwrap(),
                format_response_content(&a["content"])
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let ranking_prompt = format!(
        r#"You are evaluating different responses to the following question:

Question: {}

Here are the responses from different models (anonymized):

{}

Your task:
1. First, evaluate each response individually. For each response, explain what it does well and what it does poorly.
2. Then, at the very end of your response, provide a final ranking.

IMPORTANT: Your final ranking MUST be formatted EXACTLY as follows:
- Start with the line "FINAL RANKING:" (all caps, with colon)
- Then list the responses from best to worst as a numbered list
- Each line should be: number, period, space, then ONLY the response label (e.g., "1. Response A")
- Do not add any other text or explanations in the ranking section

Example of the correct format for your ENTIRE response:

Response A provides good detail on X but misses Y...
Response B is accurate but lacks depth on Z...
Response C offers the most comprehensive answer...

FINAL RANKING:
1. Response C
2. Response A
3. Response B

Now provide your evaluation and ranking:"#,
        user_query, responses_text
    );

    // Run LLM CLI
    let review_output = cli_runner::run_llm(engine, &ranking_prompt)
        .await
        .context("Failed to run LLM CLI for peer review")?;

    // Save markdown
    let markdown = build_review_markdown(title, engine, &user_query, answers.len(), &review_output);
    let review_md_path = base_dir.join(format!("peer-review-by-{}.md", engine_for_file));
    let legacy_review_md = base_dir.join("peer-review.md");
    if legacy_review_md.exists() && !review_md_path.exists() {
        // Migrate old file name to engine-suffixed variant if present
        fs::rename(&legacy_review_md, &review_md_path).or_else(|_| {
            let legacy_content = fs::read_to_string(&legacy_review_md)?;
            fs::write(&review_md_path, legacy_content)
        }).ok();
    }
    // Migrate legacy pattern: peer-review-<engine>.md -> peer-review-by-<engine>.md
    for entry in fs::read_dir(&base_dir)? {
        if let Ok(dir_entry) = entry {
            let path = dir_entry.path();
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.starts_with("peer-review-")
                    && file_name.ends_with(".md")
                    && !file_name.contains("peer-review-by-")
                {
                    let engine_part = file_name
                        .trim_start_matches("peer-review-")
                        .trim_end_matches(".md");
                    if !engine_part.is_empty() {
                        let new_path = base_dir.join(format!("peer-review-by-{}.md", engine_part));
                        if !new_path.exists() {
                            fs::rename(&path, &new_path).or_else(|_| {
                                let legacy_content = fs::read_to_string(&path)?;
                                fs::write(&new_path, legacy_content)
                            }).ok();
                        }
                    }
                }
            }
        }
    }
    fs::write(&review_md_path, &markdown)
        .context(format!("Failed to write review markdown file: {} (current dir: {})",
            review_md_path.display(),
            env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).display()))?;
    eprintln!("✅ Saved peer review (markdown) to: {}", review_md_path.display());

    Ok(json!({
        "success": true,
        "review_markdown_file": review_md_path.to_string_lossy(),
        "summary": format!("Peer review completed for {} answers using {}", answers.len(), engine),
        "review_preview": preview_text(&review_output, 200),
        "markdown": markdown
    }))
}

fn build_review_markdown(title: &str, engine: &str, user_query: &str, answer_count: usize, review_output: &str) -> String {
    format!(
        "# Peer Review\n- title: {}\n- engine: {}\n- answers reviewed: {}\n\n## User Question\n{}\n\n## Review\n{}",
        title,
        engine,
        answer_count,
        user_query,
        review_output
    )
}

fn extract_user_query(base_dir: &Path) -> Result<String> {
    // Try to find the original query in various possible locations
    let possible_files = [
        "query.txt",
        "user_query.txt",
        "question.txt",
        "input.txt",
    ];

    for file_name in &possible_files {
        let file_path = base_dir.join(file_name);
        if file_path.exists() {
            return Ok(fs::read_to_string(&file_path)?
                .trim()
                .to_string());
        }
    }

    // Try to extract from answer files
    let answer_files: Vec<PathBuf> = fs::read_dir(base_dir)
        .context("Failed to read directory")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_string_lossy();
            if file_name.contains("-answer.json") || file_name.ends_with("answer.json") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if let Some(first_file) = answer_files.first() {
        let content = fs::read_to_string(first_file)?;
        if let Ok(json_data) = serde_json::from_str::<Value>(&content) {
            if let Some(query) = json_data.get("query").or(json_data.get("user_query")) {
                if let Some(query_str) = query.as_str() {
                    return Ok(query_str.to_string());
                }
            }
        }
    }

    Ok("Unknown query".to_string())
}

fn read_stage1_answer(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read file: {}", path.display()))?;

    let model_from_name = path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace("-answer", ""))
        .unwrap_or_else(|| "unknown-model".to_string());

    if let Ok(json_data) = serde_json::from_str::<Value>(&content) {
        let model = json_data.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&model_from_name)
            .to_string();
        let response = format_response_content(&json_data);
        return Ok(json!({
            "model": model,
            "response": response,
            "raw": json_data
        }));
    }

    // Treat as markdown/plain text
    Ok(json!({
        "model": model_from_name,
        "response": content,
        "raw": content
    }))
}

fn format_response_content(content: &Value) -> String {
    // Try to extract the actual response text from various possible JSON structures
    if let Some(text) = content.get("response").and_then(|v| v.as_str()) {
        return text.to_string();
    }
    if let Some(text) = content.get("content").and_then(|v| v.as_str()) {
        return text.to_string();
    }
    if let Some(text) = content.as_str() {
        return text.to_string();
    }
    
    // Fallback: pretty print the JSON
    serde_json::to_string_pretty(content).unwrap_or_else(|_| "Invalid content".to_string())
}

fn preview_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

