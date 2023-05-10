use reqwest::Error as ReqwestError;
use dotenv::dotenv;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use spinners::{Spinner, Spinners};
use std::env;
use std::fs;
use pdf_extract::extract_text_from_mem;
use dialoguer::{Select};
use regex::Regex;

#[derive(Debug, Deserialize, Serialize)]
struct ChatCompletion {
    id: String,
    object: String,
    created: i64,
    model: String,
    usage: Usage,
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Usage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Choice {
    message: Message,
    finish_reason: String,
    index: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), ReqwestError> {

    dotenv().ok();

    env::set_var("RUST_BACKTRACE", "1");
    
    let rapid_api_key = env::var("RAPID_API_KEY").expect("Missing Rapid API key");

    let current_dir = std::env::current_dir().unwrap();

    // Go to pdf folder
    let pdf_dir = current_dir.join("pdf");

    // Choose a PDF file to summarize
    let paths = fs::read_dir(pdf_dir).unwrap();
    let mut file_names: Vec<String> = Vec::new();
    let mut file_name: String;

    for path in paths {
        file_name = path.unwrap().path().file_name().unwrap().to_str().unwrap().to_string();
        file_names.push(file_name);
    }

    println!("SELECT A FILE:\n");
    
    let selected_file = Select::new()
        .items(&file_names)
        .default(0)
        .clear(false)
        .interact()
        .unwrap();

    println!();

    let selected_file_path = format!("D:/Project/Programming/Rust/reviewer/pdf/{}", file_names[selected_file]);

    // Extract text from PDF
    let bytes = fs::read(selected_file_path).unwrap();
    let text = extract_text_from_mem(&bytes).unwrap();

    let regex = Regex::new(r"[^a-zA-Z0-9[:punct:] ]").unwrap();
    let clean_text = regex.replace_all(&text, "");

    // Use GPT 3.5 to summarize
    let summary_preamble = "Summarize the following: ".to_string();

    let ai_summary = chatgpt(clean_text.to_string(), summary_preamble, &rapid_api_key).await?;
    let ai_summary_text = &ai_summary.choices[0].message.content;

    println!("\n");
    println!("SUMMARY:\n");
    println!("{}", ai_summary_text);
    println!();

    // Use GPT 3.5 to generate a quiz based on the summarized topic
    let quiz_preamble = "Make a 10 item quiz based on the following: ".to_string();

    let ai_quiz = chatgpt(clean_text.to_string(), quiz_preamble, &rapid_api_key).await?;
    let ai_quiz_text = &ai_quiz.choices[0].message.content;

    println!("\n");
    println!("QUIZ:\n");
    println!("{}", ai_quiz_text);
    println!();

    Ok(())
}

async fn chatgpt(text: String, preamble: String, api_key: &String) -> Result<ChatCompletion, ReqwestError> {

    let client = reqwest::Client::new();

    let mut sp = Spinner::new(Spinners::Dots12, "\t OpenAI is generating...".into());

    let mut open_ai_headers = reqwest::header::HeaderMap::new();
    open_ai_headers.insert("X-RapidAPI-Key", api_key.parse().unwrap());
    open_ai_headers.insert("X-RapidAPI-Host", "openai80.p.rapidapi.com".parse().unwrap());

    let open_ai_req_opts = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {
                "role": "system",
                "content": preamble
            },
            {
                "role": "user",
                "content": text
            }
        ]
    });

    let open_ai_summary_res = client
        .post("https://openai80.p.rapidapi.com/chat/completions")
        .headers(open_ai_headers)
        .json(&open_ai_req_opts)
        .send()
        .await?
        .text()
        .await?;

    let chat_completion: ChatCompletion = serde_json::from_str(&open_ai_summary_res).unwrap();

    sp.stop();

    Ok(chat_completion)
}