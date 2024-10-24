use std::{
    env,
    fs::{self, File},
    io::Read,
};

use reqwest::Client;
use serde::{Deserialize, Serialize};

const URL_LLM: &str = "http://localhost:11434/api/generate";
const MODEL_LLM: &str = "llama3:instruct";
const PATH_TO_DIR: &str = "./notes";

#[derive(Debug, Deserialize, Serialize)]
struct LLMResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool,
    context: Vec<i32>,
    total_duration: f64,
    load_duration: f64,
    prompt_eval_count: i32,
    prompt_eval_duration: f64,
    eval_count: i32,
    eval_duration: f64,
    err: Option<String>,
}

struct Note {
    frontmatter: String,
    content: String,
}

impl Note {
    fn from_string(note: &str) -> Note {
        if note.starts_with("---") {
            let parts: Vec<&str> = note.splitn(3, "---").collect();
            assert!(
                parts.len() > 2,
                "Invalid note format: not enough parts after split."
            );
            let frontmatter = parts[1].trim().to_string();
            let content = parts[2].trim().to_string();
            Note {
                frontmatter,
                content,
            }
        } else {
            Note {
                frontmatter: String::new(),
                content: note.to_string(),
            }
        }
    }
}

fn get_md_files(pathdir: &str) -> Vec<String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(pathdir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let path_str = path.to_str().unwrap().to_string();
            if path_str.ends_with(".md") {
                files.push(path_str);
            }
        }
    }
    files
}

fn read_md_file(filepath: &str) -> String {
    let mut file = File::open(filepath).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents
}

fn construct_prompt(notes: Vec<Note>) -> String {
    let note_delimiter = "===\n---\n===\n";
    let preprompt_summary = format!(
        "Summarize the following notes delimited by '{}': \n",
        note_delimiter
    );
    let postprompt_summary = "okay now you have all my notes, summarize them for me. and ignore the delimiter please\n";

    let mut prompt = preprompt_summary;
    for note in notes {
        prompt.push_str(&note.content);
        prompt.push_str(note_delimiter);
    }
    prompt.push_str(postprompt_summary);
    prompt
}

async fn llm(
    url_llm: &str,
    model_llm: &str,
    prompt: &str,
) -> Result<LLMResponse, Box<dyn std::error::Error>> {
    let client = Client::new();
    let res = client
        .post(url_llm)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": model_llm,
            "prompt": prompt,
            "stream": false,
        }))
        .send()
        .await?;

    if res.status().is_success() {
        let json_response: LLMResponse = res.json().await?;
        Ok(json_response)
    } else {
        Err("Failed to get a valid response from LLM".into())
    }
}

fn main() {
    let url_llm = env::var("URL_LLM").unwrap_or_else(|_| URL_LLM.to_string());
    let model_llm = env::var("MODEL_LLM").unwrap_or_else(|_| MODEL_LLM.to_string());
    let path_to_dir = env::var("PATH_NOTES").unwrap_or_else(|_| PATH_TO_DIR.to_string());

    println!("loading notes from: {}", path_to_dir);

    let md_files = get_md_files(&path_to_dir);
    let mut notes = Vec::new();
    for file in md_files {
        let content = read_md_file(&file);
        let note = Note::from_string(&content);
        notes.push(note);
    }

    println!("got {} notes", notes.len());

    let prompt = construct_prompt(notes);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let response = runtime
        .block_on(llm(&url_llm, &model_llm, &prompt))
        .unwrap();
    println!("{}", response.response);
}
