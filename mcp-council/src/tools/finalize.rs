use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

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

pub async fn handle_finalize(params: Value) -> Result<Value> {
    let title = params["title"]
        .as_str()
        .context("Missing required parameter: title")?;
    let engine = params["engine"]
        .as_str()
        .unwrap_or("claude");

    let council_base = find_council_dir()?;
    let base_dir = council_base.join(title);
    
    if !base_dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory not found: {} (searched from: {})",
            base_dir.display(),
            env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).display()
        ));
    }

    // Load Stage1 answers (markdown preferred, JSON for backward compatibility)
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

    let mut stage1_results = Vec::new();
    for file_path in &answer_files {
        let parsed = read_stage1_answer(file_path)
            .context(format!("Failed to parse answer file: {}", file_path.display()))?;
        stage1_results.push(parsed);
    }

    // Load Stage2 reviews (markdown preferred, JSON for backward compatibility)
    let review_files: Vec<PathBuf> = fs::read_dir(&base_dir)
        .context(format!("Failed to read directory: {}", base_dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_string_lossy();
            
            if file_name.contains("peer-review") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    let mut stage2_results = Vec::new();
    for file_path in &review_files {
        let parsed = read_stage2_review(file_path)
            .context(format!("Failed to parse review file: {}", file_path.display()))?;
        stage2_results.push(parsed);
    }

    if stage2_results.is_empty() {
        return Err(anyhow::anyhow!(
            "No Stage2 review files found. Please run peer_review first."
        ));
    }

    // Extract user query
    let user_query = extract_user_query(&base_dir)?;

    // Build Stage1 text
    let stage1_text = stage1_results
        .iter()
        .enumerate()
        .map(|(idx, result)| {
            let default_model = format!("Model {}", idx + 1);
            let model = result
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_model);
            let response = format_response_content(result);
            format!("Model: {}\nResponse: {}", model, response)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Build Stage2 text
    let stage2_text = stage2_results
        .iter()
        .enumerate()
        .map(|(idx, result)| {
            let default_reviewer = format!("Reviewer {}", idx + 1);
            let model = result
                .get("engine")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_reviewer);
            let review = result
                .get("review")
                .and_then(|v| v.as_str())
                .unwrap_or("No review content");
            format!("Model: {}\nRanking: {}", model, review)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Build chairman prompt
    let chairman_prompt = format!(
        r#"You are the Chairman of an LLM Council. Multiple AI models have provided responses to a user's question, and then ranked each other's responses.

Original Question: {}

STAGE 1 - Individual Responses:
{}

STAGE 2 - Peer Rankings:
{}

Your task as Chairman is to synthesize all of this information into a single, comprehensive, accurate answer to the user's original question. Consider:
- The individual responses and their insights
- The peer rankings and what they reveal about response quality
- Any patterns of agreement or disagreement

Provide a clear, well-reasoned final answer that represents the council's collective wisdom:"#,
        user_query, stage1_text, stage2_text
    );

    // Run LLM CLI
    let final_output = cli_runner::run_llm(engine, &chairman_prompt)
        .await
        .context("Failed to run LLM CLI for finalization")?;

    // Save markdown
    let markdown = build_final_markdown(
        title,
        engine,
        &user_query,
        stage1_results.len(),
        stage2_results.len(),
        &final_output,
    );
    let final_md_path = base_dir.join(format!("final-answer-by-{}.md", engine));
    fs::write(&final_md_path, &markdown)
        .context(format!("Failed to write final markdown file: {} (current dir: {})", 
            final_md_path.display(),
            env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).display()))?;
    eprintln!("✅ Saved final answer (markdown) to: {}", final_md_path.display());

    Ok(json!({
        "success": true,
        "final_markdown_file": final_md_path.to_string_lossy(),
        "summary": format!(
            "Final answer generated using {} based on {} responses and {} reviews",
            engine,
            stage1_results.len(),
            stage2_results.len()
        ),
        "final_answer_preview": preview_text(&final_output, 300),
        "markdown": markdown
    }))
}

fn build_final_markdown(
    title: &str,
    engine: &str,
    user_query: &str,
    stage1_count: usize,
    stage2_count: usize,
    final_output: &str,
) -> String {
    format!(
        "# Final Answer\n- title: {}\n- engine: {}\n- stage1 responses: {}\n- stage2 reviews: {}\n\n## User Question\n{}\n\n## Final Answer\n{}",
        title,
        engine,
        stage1_count,
        stage2_count,
        user_query,
        final_output
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

    Ok(json!({
        "model": model_from_name,
        "response": content,
        "raw": content
    }))
}

fn read_stage2_review(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read file: {}", path.display()))?;

    let engine_from_name = path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace("peer-review-by-", ""))
        .unwrap_or_else(|| "unknown-engine".to_string());

    if let Ok(json_data) = serde_json::from_str::<Value>(&content) {
        let engine = json_data.get("engine")
            .and_then(|v| v.as_str())
            .unwrap_or(&engine_from_name)
            .to_string();
        let review = json_data.get("review")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format_response_content(&json_data));
        return Ok(json!({
            "engine": engine,
            "review": review,
            "raw": json_data
        }));
    }

    Ok(json!({
        "engine": engine_from_name,
        "review": content,
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

