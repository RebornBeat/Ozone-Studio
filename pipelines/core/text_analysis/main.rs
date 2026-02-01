//! TextAnalysisPipeline - Pipeline #20
//! 
//! STRUCTURAL text analysis - extracts stats, keywords, patterns.
//! This is a GENERAL-PURPOSE pipeline - NO direct LLM calls.
//! 
//! IMPORTANT: LLM-based semantic analysis is defined in BLUEPRINTS.
//! Pipelines are general-purpose building blocks.
//! Blueprints define the steps and can call prompt pipeline for LLM.
//! 
//! Per spec §9: Text Document Analysis System
//! 
//! WHAT THIS PIPELINE DOES (structural only):
//! - Calculate text statistics (words, sentences, paragraphs)
//! - Extract keywords via TF-IDF-like scoring
//! - Detect language via heuristics
//! - Calculate readability scores
//! - Extract basic entities via regex patterns
//! - Store analysis in ZSEI
//! - Build/update AMT/ATMT structure
//! 
//! WHAT BLUEPRINTS DO (semantic):
//! - LLM-based summarization
//! - Intelligent NER
//! - Sentiment analysis
//! - Topic classification
//! - Translation
//! 
//! AMT/ATMT: This pipeline creates the Abstract Meaning Tree structure
//! which is then refined by blueprints for semantic understanding.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

// ========== Input Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TextAnalysisInput {
    /// Structural analysis (NO LLM)
    Analyze { 
        text: String, 
        extract_entities: bool, 
        extract_topics: bool,
    },
    /// Extract keywords via TF-IDF
    ExtractKeywords { text: String, limit: Option<u32> },
    /// Calculate text statistics
    CalculateStats { text: String },
    /// Detect language
    DetectLanguage { text: String },
    /// Calculate readability
    CalculateReadability { text: String },
    /// Find similar documents in ZSEI
    FindSimilar { text: String, limit: Option<u32> },
    /// Analyze document from ZSEI reference
    AnalyzeDocument { document_ref_id: u64 },
    /// Store analysis in ZSEI
    StoreAnalysis { analysis: TextAnalysis, project_id: Option<u64> },
    /// Build AMT/ATMT structure
    BuildAMT { text: String, depth: Option<u32> },
    /// Extract basic entities via regex
    ExtractBasicEntities { text: String },
}

// ========== Output Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnalysis {
    pub word_count: u32,
    pub sentence_count: u32,
    pub paragraph_count: u32,
    pub char_count: u32,
    pub avg_sentence_length: f32,
    pub avg_word_length: f32,
    pub keywords: Vec<KeywordScore>,
    pub topics: Vec<String>,  // Detected via patterns, not LLM
    pub entities: Vec<Entity>,  // Basic regex extraction
    pub language: String,
    pub readability: ReadabilityScores,
    // These are populated by BLUEPRINTS, not this pipeline
    pub semantic_summary: Option<String>,
    pub sentiment: Option<SentimentResult>,
    pub amt: Option<AMTNode>,  // Abstract Meaning Tree root
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordScore {
    pub keyword: String,
    pub score: f32,
    pub frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub text: String,
    pub entity_type: String,  // "PERSON", "ORG", "LOCATION", "DATE", "EMAIL", "URL"
    pub start_pos: u32,
    pub end_pos: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadabilityScores {
    pub flesch_kincaid_grade: f32,
    pub flesch_reading_ease: f32,
    pub gunning_fog: f32,
    pub automated_readability_index: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentResult {
    pub score: f32,  // -1.0 to 1.0
    pub label: String,  // "positive", "negative", "neutral"
}

/// Abstract Meaning Tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMTNode {
    pub id: u64,
    pub node_type: String,  // "root", "topic", "subtopic", "entity", "action", "detail"
    pub content: String,
    pub children: Vec<AMTNode>,
    pub relationships: Vec<AMTRelation>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMTRelation {
    pub target_id: u64,
    pub relation_type: String,  // "hierarchy", "sequence", "parallel", "linkage"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarDocument {
    pub container_id: u64,
    pub similarity_score: f32,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnalysisOutput {
    pub success: bool,
    pub analysis: Option<TextAnalysis>,
    pub similar: Option<Vec<SimilarDocument>>,
    pub amt: Option<AMTNode>,
    pub container_id: Option<u64>,
    pub error: Option<String>,
}

// ========== ZSEI Integration ==========

fn call_zsei_query(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    let zsei_data_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let action = input.get("action").and_then(|a| a.as_str()).unwrap_or("");
    
    match action {
        "GetContainer" => {
            let container_id = input.get("container_id").and_then(|c| c.as_u64()).unwrap_or(0);
            let container_path = format!("{}/local/{}.json", zsei_data_path, container_id);
            
            if let Ok(content) = std::fs::read_to_string(&container_path) {
                if let Ok(container) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Ok(serde_json::json!({"success": true, "container": container}));
                }
            }
            Ok(serde_json::json!({"success": false, "error": "Container not found"}))
        }
        "Search" => {
            let query = input.get("query").and_then(|q| q.as_str()).unwrap_or("");
            let index_path = format!("{}/indices/text_index.json", zsei_data_path);
            
            if let Ok(content) = std::fs::read_to_string(&index_path) {
                if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                    let containers = index.get("documents").and_then(|c| c.as_array());
                    if let Some(containers) = containers {
                        let matches: Vec<_> = containers.iter()
                            .filter(|c| {
                                let keywords = c.get("keywords").and_then(|k| k.as_array());
                                keywords.map(|kw| kw.iter().any(|k| {
                                    k.as_str().map(|s| s.to_lowercase().contains(&query.to_lowercase())).unwrap_or(false)
                                })).unwrap_or(false)
                            })
                            .cloned()
                            .collect();
                        return Ok(serde_json::json!({"success": true, "documents": matches}));
                    }
                }
            }
            Ok(serde_json::json!({"success": true, "documents": []}))
        }
        _ => Ok(serde_json::json!({"success": false, "error": "Unknown action"}))
    }
}

fn call_zsei_write(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    let zsei_data_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    
    let container_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let local_dir = format!("{}/local", zsei_data_path);
    std::fs::create_dir_all(&local_dir).ok();
    
    let container_path = format!("{}/{}.json", local_dir, container_id);
    let container_data = serde_json::json!({
        "container_id": container_id,
        "container_type": "TextAnalysis",
        "content": input,
        "created_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    
    std::fs::write(&container_path, serde_json::to_string_pretty(&container_data).unwrap())
        .map_err(|e| format!("Write failed: {}", e))?;
    
    // Update text index
    update_text_index(&zsei_data_path, container_id, input)?;
    
    Ok(serde_json::json!({"success": true, "container_id": container_id}))
}

fn update_text_index(base_path: &str, container_id: u64, analysis: &serde_json::Value) -> Result<(), String> {
    let index_dir = format!("{}/indices", base_path);
    std::fs::create_dir_all(&index_dir).ok();
    
    let index_path = format!("{}/text_index.json", index_dir);
    let mut index: serde_json::Value = std::fs::read_to_string(&index_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| serde_json::json!({"documents": []}));
    
    let keywords: Vec<String> = analysis.get("keywords")
        .and_then(|k| k.as_array())
        .map(|arr| arr.iter()
            .filter_map(|k| k.get("keyword").and_then(|kw| kw.as_str()).map(|s| s.to_string()))
            .collect())
        .unwrap_or_default();
    
    let entry = serde_json::json!({
        "container_id": container_id,
        "keywords": keywords,
        "word_count": analysis.get("word_count"),
        "language": analysis.get("language"),
    });
    
    if let Some(docs) = index.get_mut("documents").and_then(|c| c.as_array_mut()) {
        docs.push(entry);
    }
    
    std::fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap())
        .map_err(|e| format!("Index update failed: {}", e))?;
    
    Ok(())
}

// ========== Analysis Functions (Structural Only - NO LLM) ==========

fn calculate_stats(text: &str) -> (u32, u32, u32, u32, f32, f32) {
    let chars = text.len() as u32;
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len() as u32;
    
    // Count sentences (basic: .!? followed by space or end)
    let sentences = text.chars()
        .filter(|c| *c == '.' || *c == '!' || *c == '?')
        .count() as u32;
    let sentence_count = sentences.max(1);
    
    // Count paragraphs
    let paragraphs = text.split("\n\n").filter(|p| !p.trim().is_empty()).count() as u32;
    let paragraph_count = paragraphs.max(1);
    
    let avg_sentence_length = if sentence_count > 0 {
        word_count as f32 / sentence_count as f32
    } else { 0.0 };
    
    let avg_word_length = if word_count > 0 {
        words.iter().map(|w| w.len()).sum::<usize>() as f32 / word_count as f32
    } else { 0.0 };
    
    (word_count, sentence_count, paragraph_count, chars, avg_sentence_length, avg_word_length)
}

fn detect_language(text: &str) -> String {
    let sample = text.to_lowercase();
    
    // Simple heuristic language detection
    let english_words = ["the", "is", "are", "was", "have", "has", "will", "would", "could", "should"];
    let spanish_words = ["el", "la", "los", "las", "es", "son", "fue", "tiene", "está"];
    let french_words = ["le", "la", "les", "est", "sont", "était", "avoir", "être"];
    let german_words = ["der", "die", "das", "ist", "sind", "war", "haben", "werden"];
    let portuguese_words = ["o", "a", "os", "as", "é", "são", "foi", "tem", "está"];
    
    let mut scores: HashMap<&str, u32> = HashMap::new();
    
    for word in sample.split_whitespace() {
        if english_words.contains(&word) { *scores.entry("en").or_insert(0) += 1; }
        if spanish_words.contains(&word) { *scores.entry("es").or_insert(0) += 1; }
        if french_words.contains(&word) { *scores.entry("fr").or_insert(0) += 1; }
        if german_words.contains(&word) { *scores.entry("de").or_insert(0) += 1; }
        if portuguese_words.contains(&word) { *scores.entry("pt").or_insert(0) += 1; }
    }
    
    scores.into_iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, _)| k.to_string())
        .unwrap_or_else(|| "en".to_string())  // Default to English
}

fn calculate_readability(text: &str) -> ReadabilityScores {
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len() as f32;
    let sentence_count = text.chars()
        .filter(|c| *c == '.' || *c == '!' || *c == '?')
        .count()
        .max(1) as f32;
    
    // Count syllables (simplified)
    let syllable_count: f32 = words.iter()
        .map(|w| count_syllables(w) as f32)
        .sum();
    
    // Count complex words (3+ syllables)
    let complex_words = words.iter()
        .filter(|w| count_syllables(w) >= 3)
        .count() as f32;
    
    // Flesch Reading Ease: 206.835 - 1.015*(words/sentences) - 84.6*(syllables/words)
    let flesch_reading_ease = 206.835 
        - 1.015 * (word_count / sentence_count)
        - 84.6 * (syllable_count / word_count.max(1.0));
    
    // Flesch-Kincaid Grade: 0.39*(words/sentences) + 11.8*(syllables/words) - 15.59
    let flesch_kincaid_grade = 0.39 * (word_count / sentence_count)
        + 11.8 * (syllable_count / word_count.max(1.0))
        - 15.59;
    
    // Gunning Fog: 0.4*((words/sentences) + 100*(complex_words/words))
    let gunning_fog = 0.4 * ((word_count / sentence_count) 
        + 100.0 * (complex_words / word_count.max(1.0)));
    
    // ARI: 4.71*(chars/words) + 0.5*(words/sentences) - 21.43
    let char_count = text.len() as f32;
    let automated_readability_index = 4.71 * (char_count / word_count.max(1.0))
        + 0.5 * (word_count / sentence_count)
        - 21.43;
    
    ReadabilityScores {
        flesch_kincaid_grade: flesch_kincaid_grade.max(0.0),
        flesch_reading_ease: flesch_reading_ease.clamp(0.0, 100.0),
        gunning_fog: gunning_fog.max(0.0),
        automated_readability_index: automated_readability_index.max(0.0),
    }
}

fn count_syllables(word: &str) -> u32 {
    let word = word.to_lowercase();
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut count = 0u32;
    let mut prev_vowel = false;
    
    for c in word.chars() {
        let is_vowel = vowels.contains(&c);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }
    
    // Handle silent 'e'
    if word.ends_with('e') && count > 1 {
        count -= 1;
    }
    
    count.max(1)
}

fn extract_keywords(text: &str, limit: u32) -> Vec<KeywordScore> {
    // Stop words to exclude
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
        "be", "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "must", "shall", "can", "need", "this", "that",
        "these", "those", "i", "you", "he", "she", "it", "we", "they", "what",
        "which", "who", "whom", "its", "his", "her", "their", "my", "your", "our",
    ].iter().cloned().collect();
    
    // Count word frequencies
    let mut word_counts: HashMap<String, u32> = HashMap::new();
    for word in text.split_whitespace() {
        let cleaned = word.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        
        if cleaned.len() >= 3 && !stop_words.contains(cleaned.as_str()) {
            *word_counts.entry(cleaned).or_insert(0) += 1;
        }
    }
    
    let total_words = word_counts.values().sum::<u32>().max(1) as f32;
    
    // Convert to scores (TF-IDF-like)
    let mut keywords: Vec<KeywordScore> = word_counts.into_iter()
        .map(|(keyword, frequency)| KeywordScore {
            score: (frequency as f32 / total_words) * (1.0 + (frequency as f32).ln()),
            keyword,
            frequency,
        })
        .collect();
    
    // Sort by score descending
    keywords.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    keywords.truncate(limit as usize);
    
    keywords
}

fn extract_basic_entities(text: &str) -> Vec<Entity> {
    let mut entities = Vec::new();
    
    // Email pattern
    if let Ok(re) = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}") {
        for mat in re.find_iter(text) {
            entities.push(Entity {
                text: mat.as_str().to_string(),
                entity_type: "EMAIL".to_string(),
                start_pos: mat.start() as u32,
                end_pos: mat.end() as u32,
            });
        }
    }
    
    // URL pattern
    if let Ok(re) = regex::Regex::new(r"https?://[^\s]+") {
        for mat in re.find_iter(text) {
            entities.push(Entity {
                text: mat.as_str().to_string(),
                entity_type: "URL".to_string(),
                start_pos: mat.start() as u32,
                end_pos: mat.end() as u32,
            });
        }
    }
    
    // Date patterns
    if let Ok(re) = regex::Regex::new(r"\b\d{1,2}[/\-]\d{1,2}[/\-]\d{2,4}\b|\b\d{4}[/\-]\d{1,2}[/\-]\d{1,2}\b") {
        for mat in re.find_iter(text) {
            entities.push(Entity {
                text: mat.as_str().to_string(),
                entity_type: "DATE".to_string(),
                start_pos: mat.start() as u32,
                end_pos: mat.end() as u32,
            });
        }
    }
    
    // Phone number pattern
    if let Ok(re) = regex::Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b|\+\d{1,3}[-.\s]?\d{1,4}[-.\s]?\d{1,4}[-.\s]?\d{1,4}") {
        for mat in re.find_iter(text) {
            entities.push(Entity {
                text: mat.as_str().to_string(),
                entity_type: "PHONE".to_string(),
                start_pos: mat.start() as u32,
                end_pos: mat.end() as u32,
            });
        }
    }
    
    // Capitalized words (potential names/orgs) - simplified
    if let Ok(re) = regex::Regex::new(r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)+\b") {
        for mat in re.find_iter(text) {
            let entity_text = mat.as_str().to_string();
            // Exclude common phrases
            if !["The", "This", "That", "These", "Those"].iter().any(|s| entity_text.starts_with(s)) {
                entities.push(Entity {
                    text: entity_text,
                    entity_type: "PROPER_NOUN".to_string(),
                    start_pos: mat.start() as u32,
                    end_pos: mat.end() as u32,
                });
            }
        }
    }
    
    entities
}

fn detect_topics(text: &str) -> Vec<String> {
    let mut topics = Vec::new();
    let lower = text.to_lowercase();
    
    // Topic detection via keywords (NO LLM)
    let topic_patterns = [
        ("technology", vec!["software", "computer", "app", "digital", "tech", "ai", "machine learning"]),
        ("business", vec!["company", "market", "revenue", "profit", "investment", "startup"]),
        ("health", vec!["health", "medical", "doctor", "patient", "disease", "treatment"]),
        ("science", vec!["research", "study", "experiment", "scientist", "discovery", "data"]),
        ("education", vec!["school", "university", "student", "teacher", "learning", "course"]),
        ("finance", vec!["money", "bank", "stock", "trading", "investment", "financial"]),
        ("politics", vec!["government", "election", "vote", "policy", "political", "president"]),
        ("sports", vec!["game", "team", "player", "score", "championship", "match"]),
        ("entertainment", vec!["movie", "music", "show", "celebrity", "concert", "entertainment"]),
        ("travel", vec!["travel", "trip", "vacation", "hotel", "flight", "destination"]),
    ];
    
    for (topic, keywords) in topic_patterns {
        if keywords.iter().any(|kw| lower.contains(kw)) {
            topics.push(topic.to_string());
        }
    }
    
    topics
}

fn build_amt(text: &str, depth: u32) -> AMTNode {
    let mut next_id = 1u64;
    
    // Root node
    let mut root = AMTNode {
        id: next_id,
        node_type: "root".to_string(),
        content: if text.len() > 100 { format!("{}...", &text[..100]) } else { text.to_string() },
        children: vec![],
        relationships: vec![],
        metadata: HashMap::new(),
    };
    next_id += 1;
    
    // Split into paragraphs as top-level branches
    let paragraphs: Vec<&str> = text.split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();
    
    for (i, para) in paragraphs.iter().enumerate() {
        if depth < 1 { break; }
        
        let para_node = AMTNode {
            id: next_id,
            node_type: "paragraph".to_string(),
            content: if para.len() > 200 { format!("{}...", &para[..200]) } else { para.to_string() },
            children: if depth > 1 {
                // Split paragraph into sentences
                let sentences: Vec<&str> = para.split(|c| c == '.' || c == '!' || c == '?')
                    .filter(|s| !s.trim().is_empty())
                    .collect();
                
                sentences.iter().enumerate().map(|(j, sent)| {
                    let sent_id = next_id + 100 + j as u64;
                    AMTNode {
                        id: sent_id,
                        node_type: "sentence".to_string(),
                        content: sent.trim().to_string(),
                        children: vec![],
                        relationships: if j > 0 {
                            vec![AMTRelation {
                                target_id: sent_id - 1,
                                relation_type: "sequence".to_string(),
                            }]
                        } else { vec![] },
                        metadata: HashMap::new(),
                    }
                }).collect()
            } else { vec![] },
            relationships: if i > 0 {
                vec![AMTRelation {
                    target_id: next_id - 1,
                    relation_type: "sequence".to_string(),
                }]
            } else { vec![] },
            metadata: HashMap::new(),
        };
        
        root.children.push(para_node);
        next_id += 1;
    }
    
    root
}

// ========== Main Execution ==========

pub async fn execute(input: TextAnalysisInput) -> Result<TextAnalysisOutput, String> {
    match input {
        TextAnalysisInput::Analyze { text, extract_entities, extract_topics } => {
            let (word_count, sentence_count, paragraph_count, char_count, avg_sentence_length, avg_word_length) = calculate_stats(&text);
            let language = detect_language(&text);
            let readability = calculate_readability(&text);
            let keywords = extract_keywords(&text, 20);
            
            let entities = if extract_entities {
                extract_basic_entities(&text)
            } else { vec![] };
            
            let topics = if extract_topics {
                detect_topics(&text)
            } else { vec![] };
            
            // Build basic AMT
            let amt = build_amt(&text, 2);
            
            let analysis = TextAnalysis {
                word_count,
                sentence_count,
                paragraph_count,
                char_count,
                avg_sentence_length,
                avg_word_length,
                keywords,
                topics,
                entities,
                language,
                readability,
                semantic_summary: None,  // Populated by BLUEPRINT
                sentiment: None,          // Populated by BLUEPRINT
                amt: Some(amt),
            };
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(analysis),
                similar: None,
                amt: None,
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::ExtractKeywords { text, limit } => {
            let keywords = extract_keywords(&text, limit.unwrap_or(20));
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(TextAnalysis {
                    word_count: 0,
                    sentence_count: 0,
                    paragraph_count: 0,
                    char_count: 0,
                    avg_sentence_length: 0.0,
                    avg_word_length: 0.0,
                    keywords,
                    topics: vec![],
                    entities: vec![],
                    language: "unknown".to_string(),
                    readability: ReadabilityScores {
                        flesch_kincaid_grade: 0.0,
                        flesch_reading_ease: 0.0,
                        gunning_fog: 0.0,
                        automated_readability_index: 0.0,
                    },
                    semantic_summary: None,
                    sentiment: None,
                    amt: None,
                }),
                similar: None,
                amt: None,
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::CalculateStats { text } => {
            let (word_count, sentence_count, paragraph_count, char_count, avg_sentence_length, avg_word_length) = calculate_stats(&text);
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(TextAnalysis {
                    word_count,
                    sentence_count,
                    paragraph_count,
                    char_count,
                    avg_sentence_length,
                    avg_word_length,
                    keywords: vec![],
                    topics: vec![],
                    entities: vec![],
                    language: "unknown".to_string(),
                    readability: ReadabilityScores {
                        flesch_kincaid_grade: 0.0,
                        flesch_reading_ease: 0.0,
                        gunning_fog: 0.0,
                        automated_readability_index: 0.0,
                    },
                    semantic_summary: None,
                    sentiment: None,
                    amt: None,
                }),
                similar: None,
                amt: None,
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::DetectLanguage { text } => {
            let language = detect_language(&text);
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(TextAnalysis {
                    word_count: 0,
                    sentence_count: 0,
                    paragraph_count: 0,
                    char_count: 0,
                    avg_sentence_length: 0.0,
                    avg_word_length: 0.0,
                    keywords: vec![],
                    topics: vec![],
                    entities: vec![],
                    language,
                    readability: ReadabilityScores {
                        flesch_kincaid_grade: 0.0,
                        flesch_reading_ease: 0.0,
                        gunning_fog: 0.0,
                        automated_readability_index: 0.0,
                    },
                    semantic_summary: None,
                    sentiment: None,
                    amt: None,
                }),
                similar: None,
                amt: None,
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::CalculateReadability { text } => {
            let readability = calculate_readability(&text);
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(TextAnalysis {
                    word_count: 0,
                    sentence_count: 0,
                    paragraph_count: 0,
                    char_count: 0,
                    avg_sentence_length: 0.0,
                    avg_word_length: 0.0,
                    keywords: vec![],
                    topics: vec![],
                    entities: vec![],
                    language: "unknown".to_string(),
                    readability,
                    semantic_summary: None,
                    sentiment: None,
                    amt: None,
                }),
                similar: None,
                amt: None,
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::FindSimilar { text, limit } => {
            let keywords = extract_keywords(&text, 10);
            let query_str = keywords.iter().map(|k| k.keyword.clone()).collect::<Vec<_>>().join(" ");
            
            let query = serde_json::json!({
                "action": "Search",
                "query": query_str,
            });
            
            let result = call_zsei_query(&query)?;
            let similar: Vec<SimilarDocument> = result.get("documents")
                .and_then(|c| c.as_array())
                .map(|arr| arr.iter().take(limit.unwrap_or(10) as usize).filter_map(|v| {
                    Some(SimilarDocument {
                        container_id: v.get("container_id")?.as_u64()?,
                        similarity_score: 0.8,
                        preview: "".to_string(),
                    })
                }).collect())
                .unwrap_or_default();
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: None,
                similar: Some(similar),
                amt: None,
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::AnalyzeDocument { document_ref_id } => {
            let query = serde_json::json!({"action": "GetContainer", "container_id": document_ref_id});
            let result = call_zsei_query(&query)?;
            
            if let Some(container) = result.get("container") {
                let text = container.get("content")
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                
                return Box::pin(execute(TextAnalysisInput::Analyze {
                    text: text.to_string(),
                    extract_entities: true,
                    extract_topics: true,
                })).await;
            }
            
            Ok(TextAnalysisOutput {
                success: false,
                analysis: None,
                similar: None,
                amt: None,
                container_id: None,
                error: Some("Document not found".to_string()),
            })
        }
        
        TextAnalysisInput::StoreAnalysis { analysis, project_id } => {
            let data = serde_json::to_value(&analysis).map_err(|e| e.to_string())?;
            let result = call_zsei_write(&data)?;
            let container_id = result.get("container_id").and_then(|c| c.as_u64());
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(analysis),
                similar: None,
                amt: None,
                container_id,
                error: None,
            })
        }
        
        TextAnalysisInput::BuildAMT { text, depth } => {
            let amt = build_amt(&text, depth.unwrap_or(3));
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: None,
                similar: None,
                amt: Some(amt),
                container_id: None,
                error: None,
            })
        }
        
        TextAnalysisInput::ExtractBasicEntities { text } => {
            let entities = extract_basic_entities(&text);
            
            Ok(TextAnalysisOutput {
                success: true,
                analysis: Some(TextAnalysis {
                    word_count: 0,
                    sentence_count: 0,
                    paragraph_count: 0,
                    char_count: 0,
                    avg_sentence_length: 0.0,
                    avg_word_length: 0.0,
                    keywords: vec![],
                    topics: vec![],
                    entities,
                    language: "unknown".to_string(),
                    readability: ReadabilityScores {
                        flesch_kincaid_grade: 0.0,
                        flesch_reading_ease: 0.0,
                        gunning_fog: 0.0,
                        automated_readability_index: 0.0,
                    },
                    semantic_summary: None,
                    sentiment: None,
                    amt: None,
                }),
                similar: None,
                amt: None,
                container_id: None,
                error: None,
            })
        }
    }
}

// ========== CLI Entry Point ==========

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: TextAnalysisInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
