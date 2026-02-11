//! OZONE Studio - Text Modality Pipeline (ID: 100)
//! 
//! Analyzes text and creates structural graphs for:
//! - Entities (people, places, organizations)
//! - Topics and themes
//! - Document structure (sections, paragraphs)
//! - Relationships between concepts
//! - Cross-references

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use regex::Regex;

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TextModalityInput {
    pub action: TextModalityAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextModalityAction {
    /// Analyze text and create structural representation
    Analyze {
        text: String,
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default)]
        extract_entities: bool,
        #[serde(default)]
        extract_topics: bool,
        #[serde(default)]
        extract_structure: bool,
    },
    
    /// Create a graph from analysis results
    CreateGraph {
        analysis_result: TextAnalysisResult,
        project_id: u64,
        #[serde(default)]
        link_to_existing: bool,
    },
    
    /// Update existing graph with new text
    UpdateGraph {
        graph_id: u64,
        delta: TextDelta,
    },
    
    /// Query the text graph
    QueryGraph {
        graph_id: u64,
        query: TextGraphQuery,
    },
    
    /// Normalize text (clean, chunk, tokenize)
    Normalize {
        text: String,
        #[serde(default)]
        chunk_size: Option<usize>,
        #[serde(default)]
        overlap: usize,
    },
    
    /// Extract keywords and key phrases
    ExtractKeywords {
        text: String,
        #[serde(default = "default_max_keywords")]
        max_keywords: usize,
    },
    
    /// Suggest applicable methodologies based on content
    SuggestMethodologies {
        text: String,
        available_methodology_ids: Vec<u64>,
    },
    
    /// Trigger ZSEI semantic analysis hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
    },
}

fn default_max_keywords() -> usize { 20 }

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum AnalysisDepth {
    #[default]
    Standard,
    Deep,
    Surface,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextModalityOutput {
    pub success: bool,
    pub error: Option<String>,
    
    // Analysis results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<TextAnalysisResult>,
    
    // Graph results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_id: Option<u64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<TextGraph>,
    
    // Normalization results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_text: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Vec<TextChunk>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<usize>,
    
    // Keyword results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,
    
    // Methodology suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_methodologies: Option<Vec<MethodologySuggestion>>,
    
    // Hook results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_result: Option<HookResult>,
}

impl Default for TextModalityOutput {
    fn default() -> Self {
        Self {
            success: false,
            error: None,
            analysis: None,
            graph_id: None,
            graph: None,
            normalized_text: None,
            chunks: None,
            token_count: None,
            keywords: None,
            suggested_methodologies: None,
            hook_result: None,
        }
    }
}

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextAnalysisResult {
    pub word_count: usize,
    pub sentence_count: usize,
    pub paragraph_count: usize,
    pub entities: Vec<Entity>,
    pub topics: Vec<Topic>,
    pub structure: DocumentStructure,
    pub language: Option<String>,
    pub sentiment: Option<Sentiment>,
    pub readability_score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub text: String,
    pub entity_type: EntityType,
    pub start_offset: usize,
    pub end_offset: usize,
    pub confidence: f32,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Date,
    Time,
    Money,
    Percentage,
    Product,
    Event,
    Technology,
    Concept,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Topic {
    pub name: String,
    pub keywords: Vec<String>,
    pub relevance: f32,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentStructure {
    pub sections: Vec<Section>,
    pub has_title: bool,
    pub has_abstract: bool,
    pub has_toc: bool,
    pub document_type: DocumentType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Section {
    pub id: String,
    pub title: Option<String>,
    pub level: u8,
    pub start_offset: usize,
    pub end_offset: usize,
    pub subsections: Vec<Section>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DocumentType {
    Article,
    Report,
    Email,
    Code,
    Chat,
    Documentation,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sentiment {
    pub overall: f32,  // -1.0 to 1.0
    pub positive: f32,
    pub negative: f32,
    pub neutral: f32,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraph {
    pub graph_id: u64,
    pub modality: String,
    pub nodes: Vec<TextGraphNode>,
    pub edges: Vec<TextGraphEdge>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraphNode {
    pub node_id: u64,
    pub node_type: TextNodeType,
    pub content: String,
    pub position: Option<TextPosition>,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TextNodeType {
    Document,
    Section,
    Paragraph,
    Sentence,
    Entity,
    Topic,
    Keyword,
    Reference,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextPosition {
    pub start_offset: usize,
    pub end_offset: usize,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraphEdge {
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: TextEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TextEdgeType {
    // Structural
    Contains,
    Follows,
    Precedes,
    
    // Semantic (added by ZSEI)
    References,
    RelatesTo,
    Contradicts,
    Supports,
    Elaborates,
    Summarizes,
    
    // Cross-modality
    DescribesCode,
    DescribesImage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextDelta {
    pub operation: DeltaOperation,
    pub position: Option<TextPosition>,
    pub content: Option<String>,
    pub affected_nodes: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeltaOperation {
    Insert,
    Delete,
    Replace,
    Reorder,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextGraphQuery {
    pub query_type: TextQueryType,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TextQueryType {
    FindEntities,
    FindTopics,
    FindReferences,
    GetStructure,
    SemanticSearch,
    PathBetween,
}

// ============================================================================
// OTHER TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextChunk {
    pub id: usize,
    pub text: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub token_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Keyword {
    pub term: String,
    pub frequency: usize,
    pub tfidf_score: f32,
    pub is_phrase: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodologySuggestion {
    pub methodology_id: u64,
    pub relevance: f32,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnEdgeCompletion,
    OnInferRelationships,
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookResult {
    pub hook_type: ZSEIHookType,
    pub success: bool,
    pub edges_added: usize,
    pub nodes_enriched: usize,
}

// ============================================================================
// PIPELINE IMPLEMENTATION
// ============================================================================

pub struct TextModalityPipeline {
    stop_words: Vec<String>,
    entity_patterns: HashMap<EntityType, Vec<Regex>>,
}

impl TextModalityPipeline {
    pub fn new() -> Self {
        Self {
            stop_words: Self::default_stop_words(),
            entity_patterns: Self::default_entity_patterns(),
        }
    }
    
    pub async fn execute(&self, input: TextModalityInput) -> TextModalityOutput {
        match input.action {
            TextModalityAction::Analyze { text, depth, extract_entities, extract_topics, extract_structure } => {
                self.analyze_text(&text, depth, extract_entities, extract_topics, extract_structure)
            }
            
            TextModalityAction::CreateGraph { analysis_result, project_id, link_to_existing } => {
                self.create_graph(analysis_result, project_id, link_to_existing).await
            }
            
            TextModalityAction::UpdateGraph { graph_id, delta } => {
                self.update_graph(graph_id, delta).await
            }
            
            TextModalityAction::QueryGraph { graph_id, query } => {
                self.query_graph(graph_id, query).await
            }
            
            TextModalityAction::Normalize { text, chunk_size, overlap } => {
                self.normalize_text(&text, chunk_size, overlap)
            }
            
            TextModalityAction::ExtractKeywords { text, max_keywords } => {
                self.extract_keywords(&text, max_keywords)
            }
            
            TextModalityAction::SuggestMethodologies { text, available_methodology_ids } => {
                self.suggest_methodologies(&text, &available_methodology_ids)
            }
            
            TextModalityAction::TriggerSemanticHook { graph_id, hook_type } => {
                self.trigger_semantic_hook(graph_id, hook_type).await
            }
        }
    }
    
    fn analyze_text(
        &self,
        text: &str,
        depth: AnalysisDepth,
        extract_entities: bool,
        extract_topics: bool,
        extract_structure: bool,
    ) -> TextModalityOutput {
        // Count basic metrics
        let word_count = text.split_whitespace().count();
        let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();
        let sentence_count = sentences.len();
        let paragraph_count = text.split("\n\n").filter(|p| !p.trim().is_empty()).count();
        
        // Extract entities if requested
        let entities = if extract_entities {
            self.extract_entities(text)
        } else {
            Vec::new()
        };
        
        // Extract topics if requested
        let topics = if extract_topics {
            self.extract_topics(text)
        } else {
            Vec::new()
        };
        
        // Extract structure if requested
        let structure = if extract_structure {
            self.extract_structure(text)
        } else {
            DocumentStructure {
                sections: Vec::new(),
                has_title: false,
                has_abstract: false,
                has_toc: false,
                document_type: DocumentType::Unknown,
            }
        };
        
        // Calculate readability (Flesch-Kincaid approximation)
        let avg_words_per_sentence = if sentence_count > 0 {
            word_count as f32 / sentence_count as f32
        } else {
            0.0
        };
        let syllable_count = self.count_syllables(text);
        let avg_syllables_per_word = if word_count > 0 {
            syllable_count as f32 / word_count as f32
        } else {
            0.0
        };
        let readability_score = 206.835 - (1.015 * avg_words_per_sentence) - (84.6 * avg_syllables_per_word);
        
        let analysis = TextAnalysisResult {
            word_count,
            sentence_count,
            paragraph_count,
            entities,
            topics,
            structure,
            language: Some("en".to_string()), // TODO: Detect language
            sentiment: None, // TODO: Add sentiment analysis
            readability_score: Some(readability_score.clamp(0.0, 100.0)),
        };
        
        TextModalityOutput {
            success: true,
            analysis: Some(analysis),
            ..Default::default()
        }
    }
    
    fn extract_entities(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        
        // Simple pattern-based entity extraction
        // In production, use NER model or external service
        
        // Extract capitalized phrases (potential names/organizations)
        let name_pattern = Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b").unwrap();
        for cap in name_pattern.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                entities.push(Entity {
                    text: m.as_str().to_string(),
                    entity_type: EntityType::Person, // Simplified
                    start_offset: m.start(),
                    end_offset: m.end(),
                    confidence: 0.6,
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Extract dates
        let date_pattern = Regex::new(r"\b(\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\w+ \d{1,2},? \d{4})\b").unwrap();
        for cap in date_pattern.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                entities.push(Entity {
                    text: m.as_str().to_string(),
                    entity_type: EntityType::Date,
                    start_offset: m.start(),
                    end_offset: m.end(),
                    confidence: 0.8,
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Extract money amounts
        let money_pattern = Regex::new(r"\$[\d,]+(?:\.\d{2})?|\d+(?:,\d{3})*(?:\.\d{2})?\s*(?:USD|EUR|GBP)").unwrap();
        for cap in money_pattern.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                entities.push(Entity {
                    text: m.as_str().to_string(),
                    entity_type: EntityType::Money,
                    start_offset: m.start(),
                    end_offset: m.end(),
                    confidence: 0.9,
                    metadata: HashMap::new(),
                });
            }
        }
        
        entities
    }
    
    fn extract_topics(&self, text: &str) -> Vec<Topic> {
        // Extract topics based on keyword frequency and co-occurrence
        let keywords = self.extract_keywords_internal(text, 10);
        
        // Group keywords into topics
        let mut topics = Vec::new();
        for (i, kw) in keywords.iter().enumerate().take(5) {
            topics.push(Topic {
                name: kw.term.clone(),
                keywords: vec![kw.term.clone()],
                relevance: 1.0 - (i as f32 * 0.15),
                category: None,
            });
        }
        
        topics
    }
    
    fn extract_structure(&self, text: &str) -> DocumentStructure {
        let mut sections = Vec::new();
        let mut section_id = 0;
        
        // Detect markdown-style headers
        let header_pattern = Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();
        for cap in header_pattern.captures_iter(text) {
            let level = cap.get(1).map(|m| m.as_str().len()).unwrap_or(1) as u8;
            let title = cap.get(2).map(|m| m.as_str().to_string());
            let start = cap.get(0).map(|m| m.start()).unwrap_or(0);
            
            sections.push(Section {
                id: format!("section_{}", section_id),
                title,
                level,
                start_offset: start,
                end_offset: start + cap.get(0).map(|m| m.len()).unwrap_or(0),
                subsections: Vec::new(),
            });
            section_id += 1;
        }
        
        // Detect document type
        let document_type = if text.contains("```") || text.contains("fn ") || text.contains("def ") {
            DocumentType::Code
        } else if text.starts_with("Subject:") || text.contains("From:") {
            DocumentType::Email
        } else if text.contains("# ") || text.contains("## ") {
            DocumentType::Article
        } else {
            DocumentType::Unknown
        };
        
        DocumentStructure {
            sections,
            has_title: !sections.is_empty(),
            has_abstract: text.to_lowercase().contains("abstract"),
            has_toc: text.to_lowercase().contains("table of contents"),
            document_type,
        }
    }
    
    fn normalize_text(&self, text: &str, chunk_size: Option<usize>, overlap: usize) -> TextModalityOutput {
        // Normalize: lowercase, remove extra whitespace, etc.
        let normalized = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        
        let token_count = normalized.split_whitespace().count();
        
        // Chunk if requested
        let chunks = if let Some(size) = chunk_size {
            self.chunk_text(&normalized, size, overlap)
        } else {
            None
        };
        
        TextModalityOutput {
            success: true,
            normalized_text: Some(normalized),
            token_count: Some(token_count),
            chunks,
            ..Default::default()
        }
    }
    
    fn chunk_text(&self, text: &str, chunk_size: usize, overlap: usize) -> Option<Vec<TextChunk>> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_id = 0;
        
        while start < words.len() {
            let end = (start + chunk_size).min(words.len());
            let chunk_words = &words[start..end];
            let chunk_text = chunk_words.join(" ");
            
            // Calculate offsets (approximate)
            let start_offset = if start > 0 {
                words[..start].join(" ").len() + 1
            } else {
                0
            };
            let end_offset = start_offset + chunk_text.len();
            
            chunks.push(TextChunk {
                id: chunk_id,
                text: chunk_text,
                start_offset,
                end_offset,
                token_count: chunk_words.len(),
            });
            
            chunk_id += 1;
            start = if end >= words.len() {
                words.len()
            } else {
                end - overlap.min(end)
            };
        }
        
        Some(chunks)
    }
    
    fn extract_keywords(&self, text: &str, max_keywords: usize) -> TextModalityOutput {
        let keywords = self.extract_keywords_internal(text, max_keywords);
        TextModalityOutput {
            success: true,
            keywords: Some(keywords),
            ..Default::default()
        }
    }
    
    fn extract_keywords_internal(&self, text: &str, max_keywords: usize) -> Vec<Keyword> {
        let words: Vec<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|w| w.len() > 2 && !self.stop_words.contains(&w.to_string()))
            .map(String::from)
            .collect();
        
        // Count frequencies
        let mut freq_map: HashMap<String, usize> = HashMap::new();
        for word in &words {
            *freq_map.entry(word.clone()).or_insert(0) += 1;
        }
        
        // Sort by frequency
        let mut keywords: Vec<_> = freq_map.into_iter().collect();
        keywords.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Convert to Keyword structs
        let total_words = words.len() as f32;
        keywords.into_iter()
            .take(max_keywords)
            .map(|(term, freq)| Keyword {
                term,
                frequency: freq,
                tfidf_score: freq as f32 / total_words, // Simplified TF
                is_phrase: false,
            })
            .collect()
    }
    
    fn suggest_methodologies(&self, text: &str, available_ids: &[u64]) -> TextModalityOutput {
        // Simple keyword-based methodology suggestion
        let text_lower = text.to_lowercase();
        let mut suggestions = Vec::new();
        
        // Check for document analysis keywords
        if text_lower.contains("document") || text_lower.contains("report") || text_lower.contains("article") {
            if available_ids.contains(&6) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 6,
                    relevance: 0.9,
                    reason: "Content appears to be document-related".to_string(),
                });
            }
        }
        
        // Check for information extraction keywords
        if text_lower.contains("extract") || text_lower.contains("find") || text_lower.contains("identify") {
            if available_ids.contains(&7) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 7,
                    relevance: 0.85,
                    reason: "Task involves information extraction".to_string(),
                });
            }
        }
        
        // Check for conversational content
        if text_lower.contains("conversation") || text_lower.contains("chat") || text_lower.contains("respond") {
            if available_ids.contains(&1) {
                suggestions.push(MethodologySuggestion {
                    methodology_id: 1,
                    relevance: 0.8,
                    reason: "Content is conversational in nature".to_string(),
                });
            }
        }
        
        TextModalityOutput {
            success: true,
            suggested_methodologies: Some(suggestions),
            ..Default::default()
        }
    }
    
    async fn create_graph(&self, analysis: TextAnalysisResult, project_id: u64, _link_to_existing: bool) -> TextModalityOutput {
        // Create graph from analysis
        let graph_id = self.generate_graph_id();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_id = 1u64;
        
        // Create document root node
        let doc_node_id = node_id;
        nodes.push(TextGraphNode {
            node_id: doc_node_id,
            node_type: TextNodeType::Document,
            content: format!("{} words, {} sentences", analysis.word_count, analysis.sentence_count),
            position: None,
            properties: HashMap::new(),
        });
        node_id += 1;
        
        // Create section nodes
        for section in &analysis.structure.sections {
            let section_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: section_node_id,
                node_type: TextNodeType::Section,
                content: section.title.clone().unwrap_or_default(),
                position: Some(TextPosition {
                    start_offset: section.start_offset,
                    end_offset: section.end_offset,
                    line: None,
                    column: None,
                }),
                properties: HashMap::new(),
            });
            
            // Edge from document to section
            edges.push(TextGraphEdge {
                from_node: doc_node_id,
                to_node: section_node_id,
                edge_type: TextEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });
            
            node_id += 1;
        }
        
        // Create entity nodes
        for entity in &analysis.entities {
            let entity_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: entity_node_id,
                node_type: TextNodeType::Entity,
                content: entity.text.clone(),
                position: Some(TextPosition {
                    start_offset: entity.start_offset,
                    end_offset: entity.end_offset,
                    line: None,
                    column: None,
                }),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("entity_type".to_string(), serde_json::to_value(&entity.entity_type).unwrap());
                    props.insert("confidence".to_string(), serde_json::json!(entity.confidence));
                    props
                },
            });
            
            node_id += 1;
        }
        
        // Create topic nodes
        for topic in &analysis.topics {
            let topic_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: topic_node_id,
                node_type: TextNodeType::Topic,
                content: topic.name.clone(),
                position: None,
                properties: {
                    let mut props = HashMap::new();
                    props.insert("relevance".to_string(), serde_json::json!(topic.relevance));
                    props.insert("keywords".to_string(), serde_json::to_value(&topic.keywords).unwrap());
                    props
                },
            });
            
            // Edge from document to topic
            edges.push(TextGraphEdge {
                from_node: doc_node_id,
                to_node: topic_node_id,
                edge_type: TextEdgeType::Contains,
                weight: topic.relevance,
                properties: HashMap::new(),
            });
            
            node_id += 1;
        }
        
        let graph = TextGraph {
            graph_id,
            modality: "text".to_string(),
            nodes,
            edges,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("project_id".to_string(), serde_json::json!(project_id));
                meta.insert("created_at".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
                meta
            },
        };
        
        TextModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            graph: Some(graph),
            ..Default::default()
        }
    }
    
    async fn update_graph(&self, graph_id: u64, _delta: TextDelta) -> TextModalityOutput {
        // TODO: Implement graph update logic
        TextModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            ..Default::default()
        }
    }
    
    async fn query_graph(&self, graph_id: u64, _query: TextGraphQuery) -> TextModalityOutput {
        // TODO: Implement graph query logic
        TextModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            ..Default::default()
        }
    }
    
    async fn trigger_semantic_hook(&self, graph_id: u64, hook_type: ZSEIHookType) -> TextModalityOutput {
        // This would call into ZSEI to trigger semantic analysis
        // The actual hook processing happens in ZSEI
        TextModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            hook_result: Some(HookResult {
                hook_type,
                success: true,
                edges_added: 0, // Filled by ZSEI
                nodes_enriched: 0,
            }),
            ..Default::default()
        }
    }
    
    fn count_syllables(&self, text: &str) -> usize {
        // Simple syllable counting heuristic
        let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
        let mut count = 0;
        let mut prev_was_vowel = false;
        
        for c in text.to_lowercase().chars() {
            let is_vowel = vowels.contains(&c);
            if is_vowel && !prev_was_vowel {
                count += 1;
            }
            prev_was_vowel = is_vowel;
        }
        
        count.max(1)
    }
    
    fn generate_graph_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
    
    fn default_stop_words() -> Vec<String> {
        vec![
            "the", "a", "an", "and", "or", "but", "is", "are", "was", "were",
            "be", "been", "being", "have", "has", "had", "do", "does", "did",
            "will", "would", "could", "should", "may", "might", "must", "shall",
            "can", "need", "dare", "ought", "used", "to", "of", "in", "for",
            "on", "with", "at", "by", "from", "as", "into", "through", "during",
            "before", "after", "above", "below", "between", "under", "again",
            "further", "then", "once", "here", "there", "when", "where", "why",
            "how", "all", "each", "few", "more", "most", "other", "some", "such",
            "no", "nor", "not", "only", "own", "same", "so", "than", "too",
            "very", "just", "also", "now", "this", "that", "these", "those",
            "i", "me", "my", "myself", "we", "our", "ours", "ourselves", "you",
            "your", "yours", "yourself", "yourselves", "he", "him", "his",
            "himself", "she", "her", "hers", "herself", "it", "its", "itself",
            "they", "them", "their", "theirs", "themselves", "what", "which",
            "who", "whom", "whose", "about", "against", "because", "over",
        ].into_iter().map(String::from).collect()
    }
    
    fn default_entity_patterns() -> HashMap<EntityType, Vec<Regex>> {
        HashMap::new() // Extended patterns would go here
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() {
    let input: TextModalityInput = serde_json::from_reader(std::io::stdin())
        .expect("Failed to parse input");
    
    let pipeline = TextModalityPipeline::new();
    let output = pipeline.execute(input).await;
    
    serde_json::to_writer(std::io::stdout(), &output)
        .expect("Failed to write output");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_analyze_text() {
        let pipeline = TextModalityPipeline::new();
        let input = TextModalityInput {
            action: TextModalityAction::Analyze {
                text: "John Smith works at Acme Corp. He joined on January 15, 2024.".to_string(),
                depth: AnalysisDepth::Standard,
                extract_entities: true,
                extract_topics: true,
                extract_structure: true,
            },
        };
        
        let output = pipeline.execute(input).await;
        assert!(output.success);
        assert!(output.analysis.is_some());
        
        let analysis = output.analysis.unwrap();
        assert!(analysis.word_count > 0);
        assert!(!analysis.entities.is_empty());
    }
    
    #[test]
    fn test_normalize_and_chunk() {
        let pipeline = TextModalityPipeline::new();
        let output = pipeline.normalize_text(
            "This is a test. It has multiple sentences. We want to chunk it.",
            Some(5),
            2,
        );
        
        assert!(output.success);
        assert!(output.chunks.is_some());
        assert!(output.chunks.unwrap().len() > 0);
    }
    
    #[test]
    fn test_extract_keywords() {
        let pipeline = TextModalityPipeline::new();
        let output = pipeline.extract_keywords(
            "Machine learning algorithms process data to find patterns. These patterns help machine learning models make predictions.",
            10,
        );
        
        assert!(output.success);
        assert!(output.keywords.is_some());
        
        let keywords = output.keywords.unwrap();
        assert!(keywords.iter().any(|k| k.term.contains("machine") || k.term.contains("learning")));
    }
}
