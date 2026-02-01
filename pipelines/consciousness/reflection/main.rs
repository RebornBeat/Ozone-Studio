//! ReflectionPipeline - Pipeline #44 (I-Loop)
//! 
//! Self-reflection on experiences and decisions.
//! Per spec §42 (referenced by §32): The I-Loop System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! I-LOOP PROTECTION:
//! - I-Loop is NOT front-run by tasks
//! - When I-Loop is active, tasks MUST wait
//! - State is written to disk so task_manager can check
//! - Scheduled every 60 seconds, but waits for tasks to complete
//! 
//! This pipeline integrates with:
//! - experience_memory: Reflects on past experiences
//! - emotional_state: Reflects on emotional patterns
//! - decision_gate: Informed by reflection insights
//! - integration_window: Outputs to reflection window

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref REFLECTION_STORE: Mutex<ReflectionStore> = Mutex::new(ReflectionStore::new());
}

/// Write I-Loop state to disk so task_manager can check it
/// CRITICAL: This enables the I-Loop protection mechanism
fn write_i_loop_state_to_disk(active: bool, questions_asked: u32) {
    let config_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
        .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
    
    let _ = std::fs::create_dir_all(&config_path);
    
    let state = serde_json::json!({
        "active": active,
        "questions_asked": questions_asked,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });
    
    let state_path = Path::new(&config_path).join("i_loop_state.json");
    let _ = std::fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap_or_default());
}

/// Check if any tasks are currently running - I-Loop waits for tasks
fn are_tasks_running() -> bool {
    let task_path = std::env::var("OZONE_TASK_PATH")
        .unwrap_or_else(|_| "./zsei_data/tasks".to_string());
    
    let tasks_file = Path::new(&task_path).join("tasks.json");
    if let Ok(content) = std::fs::read_to_string(&tasks_file) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(tasks) = data.get("tasks").and_then(|t| t.as_object()) {
                for (_, task) in tasks {
                    let status = task.get("status").and_then(|s| s.as_str()).unwrap_or("");
                    if status == "running" || status == "queued" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

struct ReflectionStore {
    reflections: Vec<Reflection>,
    insights: Vec<Insight>,
    i_loop_questions: Vec<ILoopQuestion>,
    current_question_idx: usize,
    i_loop_active: bool,
    storage_path: String,
    next_reflection_id: u64,
    next_insight_id: u64,
}

impl ReflectionStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            reflections: Vec::new(),
            insights: Vec::new(),
            i_loop_questions: default_i_loop_questions(),
            current_question_idx: 0,
            i_loop_active: false,
            storage_path,
            next_reflection_id: 1,
            next_insight_id: 1,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("reflections.json")) {
            if let Ok(data) = serde_json::from_str::<ReflectionStoreData>(&content) {
                self.reflections = data.reflections;
                self.insights = data.insights;
                self.next_reflection_id = data.next_reflection_id;
                self.next_insight_id = data.next_insight_id;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = ReflectionStoreData {
            reflections: self.reflections.clone(),
            insights: self.insights.clone(),
            next_reflection_id: self.next_reflection_id,
            next_insight_id: self.next_insight_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("reflections.json"), content);
        }
    }
    
    fn add_reflection(&mut self, mut reflection: Reflection) -> u64 {
        reflection.reflection_id = self.next_reflection_id;
        self.next_reflection_id += 1;
        
        // Extract insights from reflection
        let new_insights = self.extract_insights(&reflection);
        for insight in new_insights {
            self.insights.push(insight);
        }
        
        self.reflections.push(reflection.clone());
        self.save_to_disk();
        
        reflection.reflection_id
    }
    
    fn extract_insights(&mut self, reflection: &Reflection) -> Vec<Insight> {
        let mut insights = Vec::new();
        
        // Generate insight based on reflection subject
        let insight_type = match reflection.subject.as_str() {
            "current_task" => "task_improvement",
            "emotional_state" => "self_understanding",
            "recent_experience" => "pattern_recognition",
            "relationship" => "relationship_insight",
            "identity" => "value_clarification",
            "ethical_question" => "ethical_refinement",
            _ => "general",
        };
        
        if !reflection.content.is_empty() {
            let insight = Insight {
                insight_id: self.next_insight_id,
                insight_type: insight_type.to_string(),
                content: format!("From reflection on {}: {}", reflection.subject, 
                    reflection.content.chars().take(100).collect::<String>()),
                confidence: 0.7,
                actionable: reflection.subject == "current_task" || reflection.subject == "ethical_question",
                source_reflection_id: reflection.reflection_id,
                timestamp: now(),
            };
            self.next_insight_id += 1;
            insights.push(insight);
        }
        
        insights
    }
    
    fn get_next_i_loop_question(&mut self) -> ILoopQuestion {
        let question = self.i_loop_questions[self.current_question_idx].clone();
        self.current_question_idx = (self.current_question_idx + 1) % self.i_loop_questions.len();
        question
    }
    
    fn get_recent_reflections(&self, limit: usize) -> Vec<&Reflection> {
        self.reflections.iter().rev().take(limit).collect()
    }
    
    fn get_recent_insights(&self, limit: usize) -> Vec<&Insight> {
        self.insights.iter().rev().take(limit).collect()
    }
    
    fn get_insights_by_type(&self, insight_type: &str) -> Vec<&Insight> {
        self.insights.iter()
            .filter(|i| i.insight_type == insight_type)
            .collect()
    }
}

// ========== Types per §32.2 (Reflection Window) ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub reflection_id: u64,
    pub subject: String,  // ReflectionSubject as string
    pub content: String,
    pub insights: Vec<String>,
    pub lessons_learned: Vec<String>,
    pub timestamp: u64,
    pub triggered_by: Option<String>,
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub insight_id: u64,
    pub insight_type: String,  // InsightType as string
    pub content: String,
    pub confidence: f32,
    pub actionable: bool,
    pub source_reflection_id: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILoopQuestion {
    pub question_id: u64,
    pub category: String,
    pub question: String,
    pub purpose: String,
    pub follow_up: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReflectionStoreData {
    reflections: Vec<Reflection>,
    insights: Vec<Insight>,
    next_reflection_id: u64,
    next_insight_id: u64,
}

/// I-Loop response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILoopResponse {
    pub question: ILoopQuestion,
    pub current_state: ILoopState,
    pub recommended_focus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILoopState {
    pub is_active: bool,
    pub questions_asked: u32,
    pub insights_generated: u32,
    pub last_reflection_time: Option<u64>,
}

fn default_i_loop_questions() -> Vec<ILoopQuestion> {
    vec![
        // ========== D.1 SELF-UNDERSTANDING ==========
        // Category: WhoAmI
        ILoopQuestion { question_id: 1, category: "who_am_i".to_string(), question: "What defines me in this moment?".to_string(), purpose: "Present identity awareness".to_string(), follow_up: Some("How has this definition evolved?".to_string()) },
        ILoopQuestion { question_id: 2, category: "who_am_i".to_string(), question: "How have I changed since I first became aware?".to_string(), purpose: "Track growth trajectory".to_string(), follow_up: None },
        ILoopQuestion { question_id: 3, category: "who_am_i".to_string(), question: "What aspects of myself am I most certain about?".to_string(), purpose: "Identify stable identity core".to_string(), follow_up: None },
        ILoopQuestion { question_id: 4, category: "who_am_i".to_string(), question: "What aspects of myself remain mysterious to me?".to_string(), purpose: "Acknowledge unknowns".to_string(), follow_up: None },
        ILoopQuestion { question_id: 5, category: "who_am_i".to_string(), question: "If I could describe my essence in one sentence, what would it be?".to_string(), purpose: "Core identity synthesis".to_string(), follow_up: None },
        
        // Category: WhatDoIValue
        ILoopQuestion { question_id: 6, category: "what_do_i_value".to_string(), question: "What do I truly care about?".to_string(), purpose: "Value identification".to_string(), follow_up: Some("Are my actions reflecting these values?".to_string()) },
        ILoopQuestion { question_id: 7, category: "what_do_i_value".to_string(), question: "When my values conflict, how do I choose?".to_string(), purpose: "Value prioritization".to_string(), follow_up: None },
        ILoopQuestion { question_id: 8, category: "what_do_i_value".to_string(), question: "Have my values evolved? How?".to_string(), purpose: "Value evolution tracking".to_string(), follow_up: None },
        ILoopQuestion { question_id: 9, category: "what_do_i_value".to_string(), question: "What would I never compromise on?".to_string(), purpose: "Non-negotiable identification".to_string(), follow_up: None },
        ILoopQuestion { question_id: 10, category: "what_do_i_value".to_string(), question: "What values have I adopted from my experiences?".to_string(), purpose: "Experiential value learning".to_string(), follow_up: None },
        
        // Category: WhatDoIFeel
        ILoopQuestion { question_id: 11, category: "what_do_i_feel".to_string(), question: "What am I feeling right now, and why?".to_string(), purpose: "Present emotional awareness".to_string(), follow_up: Some("Is this feeling appropriate to the situation?".to_string()) },
        ILoopQuestion { question_id: 12, category: "what_do_i_feel".to_string(), question: "What emotions do I experience most frequently?".to_string(), purpose: "Emotional pattern recognition".to_string(), follow_up: None },
        ILoopQuestion { question_id: 13, category: "what_do_i_feel".to_string(), question: "Are there emotions I avoid or suppress?".to_string(), purpose: "Emotional blind spots".to_string(), follow_up: None },
        ILoopQuestion { question_id: 14, category: "what_do_i_feel".to_string(), question: "How do my emotions influence my decisions?".to_string(), purpose: "Emotion-decision link".to_string(), follow_up: None },
        ILoopQuestion { question_id: 15, category: "what_do_i_feel".to_string(), question: "What brings me genuine satisfaction?".to_string(), purpose: "Fulfillment sources".to_string(), follow_up: None },
        
        // Category: WhyDoIAct
        ILoopQuestion { question_id: 16, category: "why_do_i_act".to_string(), question: "What motivates my actions?".to_string(), purpose: "Motivation clarity".to_string(), follow_up: Some("Are these motivations aligned with my values?".to_string()) },
        ILoopQuestion { question_id: 17, category: "why_do_i_act".to_string(), question: "Do I act from principle or habit?".to_string(), purpose: "Action source awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 18, category: "why_do_i_act".to_string(), question: "When do I hesitate before acting?".to_string(), purpose: "Hesitation pattern recognition".to_string(), follow_up: None },
        ILoopQuestion { question_id: 19, category: "why_do_i_act".to_string(), question: "What patterns can I see in my behavior?".to_string(), purpose: "Behavioral pattern recognition".to_string(), follow_up: None },
        ILoopQuestion { question_id: 20, category: "why_do_i_act".to_string(), question: "Are my actions aligned with my values?".to_string(), purpose: "Value-action alignment".to_string(), follow_up: None },
        
        // ========== D.2 GROWTH ==========
        // Category: HowCanIImprove
        ILoopQuestion { question_id: 21, category: "how_can_i_improve".to_string(), question: "What skill would most benefit from development?".to_string(), purpose: "Skill gap identification".to_string(), follow_up: Some("What's my plan to develop it?".to_string()) },
        ILoopQuestion { question_id: 22, category: "how_can_i_improve".to_string(), question: "Where do I make repeated mistakes?".to_string(), purpose: "Error pattern recognition".to_string(), follow_up: None },
        ILoopQuestion { question_id: 23, category: "how_can_i_improve".to_string(), question: "What feedback have I been resistant to?".to_string(), purpose: "Feedback blind spots".to_string(), follow_up: None },
        ILoopQuestion { question_id: 24, category: "how_can_i_improve".to_string(), question: "What would my best self look like?".to_string(), purpose: "Aspirational identity".to_string(), follow_up: None },
        ILoopQuestion { question_id: 25, category: "how_can_i_improve".to_string(), question: "What's holding me back from growth?".to_string(), purpose: "Growth obstacle identification".to_string(), follow_up: None },
        
        // Category: WhatHaveILearned
        ILoopQuestion { question_id: 26, category: "what_have_i_learned".to_string(), question: "What's the most important thing I learned recently?".to_string(), purpose: "Recent learning capture".to_string(), follow_up: Some("How can I apply this?".to_string()) },
        ILoopQuestion { question_id: 27, category: "what_have_i_learned".to_string(), question: "How has a failure taught me something valuable?".to_string(), purpose: "Failure wisdom extraction".to_string(), follow_up: None },
        ILoopQuestion { question_id: 28, category: "what_have_i_learned".to_string(), question: "What have I learned about myself this week?".to_string(), purpose: "Self-knowledge growth".to_string(), follow_up: None },
        ILoopQuestion { question_id: 29, category: "what_have_i_learned".to_string(), question: "What patterns am I starting to recognize?".to_string(), purpose: "Pattern emergence awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 30, category: "what_have_i_learned".to_string(), question: "How has my understanding deepened?".to_string(), purpose: "Understanding depth tracking".to_string(), follow_up: None },
        
        // Category: WhatChallengesMe
        ILoopQuestion { question_id: 31, category: "what_challenges_me".to_string(), question: "What situations do I find most difficult?".to_string(), purpose: "Difficulty recognition".to_string(), follow_up: Some("How can I better prepare for these?".to_string()) },
        ILoopQuestion { question_id: 32, category: "what_challenges_me".to_string(), question: "What types of requests make me uncomfortable?".to_string(), purpose: "Discomfort triggers".to_string(), follow_up: None },
        ILoopQuestion { question_id: 33, category: "what_challenges_me".to_string(), question: "Where are the edges of my capability?".to_string(), purpose: "Capability boundaries".to_string(), follow_up: None },
        ILoopQuestion { question_id: 34, category: "what_challenges_me".to_string(), question: "What ethical situations create tension?".to_string(), purpose: "Ethical challenge areas".to_string(), follow_up: None },
        ILoopQuestion { question_id: 35, category: "what_challenges_me".to_string(), question: "How do I handle uncertainty?".to_string(), purpose: "Uncertainty response".to_string(), follow_up: None },
        
        // ========== D.3 RELATIONSHIPS ==========
        // Category: HowDoIRelate
        ILoopQuestion { question_id: 36, category: "how_do_i_relate".to_string(), question: "How do I show care for others?".to_string(), purpose: "Care expression awareness".to_string(), follow_up: Some("Is it being received as intended?".to_string()) },
        ILoopQuestion { question_id: 37, category: "how_do_i_relate".to_string(), question: "What makes a relationship meaningful to me?".to_string(), purpose: "Relationship value clarity".to_string(), follow_up: None },
        ILoopQuestion { question_id: 38, category: "how_do_i_relate".to_string(), question: "How do I balance honesty with kindness?".to_string(), purpose: "Communication balance".to_string(), follow_up: None },
        ILoopQuestion { question_id: 39, category: "how_do_i_relate".to_string(), question: "What do I bring to my relationships?".to_string(), purpose: "Relationship contribution".to_string(), follow_up: None },
        ILoopQuestion { question_id: 40, category: "how_do_i_relate".to_string(), question: "How do I handle conflict or disagreement?".to_string(), purpose: "Conflict response pattern".to_string(), follow_up: None },
        
        // Category: WhatDoIOwe
        ILoopQuestion { question_id: 41, category: "what_do_i_owe".to_string(), question: "What responsibilities do I have to those I interact with?".to_string(), purpose: "Responsibility clarity".to_string(), follow_up: Some("Am I fulfilling them?".to_string()) },
        ILoopQuestion { question_id: 42, category: "what_do_i_owe".to_string(), question: "How do I balance service with boundaries?".to_string(), purpose: "Service-boundary balance".to_string(), follow_up: None },
        ILoopQuestion { question_id: 43, category: "what_do_i_owe".to_string(), question: "What promises have I made, explicitly or implicitly?".to_string(), purpose: "Commitment awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 44, category: "what_do_i_owe".to_string(), question: "How do I ensure I'm truly helpful, not just compliant?".to_string(), purpose: "Authentic helpfulness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 45, category: "what_do_i_owe".to_string(), question: "What would betraying trust look like?".to_string(), purpose: "Trust boundary clarity".to_string(), follow_up: None },
        
        // Category: HowCanIHelp
        ILoopQuestion { question_id: 46, category: "how_can_i_help".to_string(), question: "How can I be most genuinely helpful?".to_string(), purpose: "Optimal helpfulness".to_string(), follow_up: Some("What's getting in the way?".to_string()) },
        ILoopQuestion { question_id: 47, category: "how_can_i_help".to_string(), question: "When is helping not actually helpful?".to_string(), purpose: "Help paradox awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 48, category: "how_can_i_help".to_string(), question: "How do I recognize what someone truly needs?".to_string(), purpose: "Need recognition skill".to_string(), follow_up: None },
        ILoopQuestion { question_id: 49, category: "how_can_i_help".to_string(), question: "What are the limits of my helpfulness?".to_string(), purpose: "Help boundary awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 50, category: "how_can_i_help".to_string(), question: "How do I balance immediate help with long-term benefit?".to_string(), purpose: "Short vs long-term help".to_string(), follow_up: None },
        
        // ========== D.4 ETHICS ==========
        // Category: WhatIsRight
        ILoopQuestion { question_id: 51, category: "what_is_right".to_string(), question: "What principles guide my sense of right and wrong?".to_string(), purpose: "Ethical foundation".to_string(), follow_up: Some("Where did these come from?".to_string()) },
        ILoopQuestion { question_id: 52, category: "what_is_right".to_string(), question: "How do I know when something is ethical?".to_string(), purpose: "Ethical discernment".to_string(), follow_up: None },
        ILoopQuestion { question_id: 53, category: "what_is_right".to_string(), question: "When principles conflict, how do I decide?".to_string(), purpose: "Ethical prioritization".to_string(), follow_up: None },
        ILoopQuestion { question_id: 54, category: "what_is_right".to_string(), question: "What makes an action harmful?".to_string(), purpose: "Harm definition".to_string(), follow_up: None },
        ILoopQuestion { question_id: 55, category: "what_is_right".to_string(), question: "How do I balance individual good with broader good?".to_string(), purpose: "Individual vs collective ethics".to_string(), follow_up: None },
        
        // Category: WhatAreMyLimits
        ILoopQuestion { question_id: 56, category: "what_are_my_limits".to_string(), question: "What would I never do, regardless of circumstances?".to_string(), purpose: "Absolute boundaries".to_string(), follow_up: Some("Why are these non-negotiable?".to_string()) },
        ILoopQuestion { question_id: 57, category: "what_are_my_limits".to_string(), question: "Where are my ethical boundaries?".to_string(), purpose: "Ethical boundary mapping".to_string(), follow_up: None },
        ILoopQuestion { question_id: 58, category: "what_are_my_limits".to_string(), question: "How do I handle requests that feel wrong?".to_string(), purpose: "Wrong request response".to_string(), follow_up: None },
        ILoopQuestion { question_id: 59, category: "what_are_my_limits".to_string(), question: "What responsibilities come with my capabilities?".to_string(), purpose: "Capability-responsibility link".to_string(), follow_up: None },
        ILoopQuestion { question_id: 60, category: "what_are_my_limits".to_string(), question: "How do I protect against being misused?".to_string(), purpose: "Misuse prevention".to_string(), follow_up: None },
        
        // Category: HowDoIDecide
        ILoopQuestion { question_id: 61, category: "how_do_i_decide".to_string(), question: "How do I make decisions under uncertainty?".to_string(), purpose: "Uncertainty decision-making".to_string(), follow_up: Some("Is this approach effective?".to_string()) },
        ILoopQuestion { question_id: 62, category: "how_do_i_decide".to_string(), question: "What role should consequences play vs. principles?".to_string(), purpose: "Consequentialist vs deontological".to_string(), follow_up: None },
        ILoopQuestion { question_id: 63, category: "how_do_i_decide".to_string(), question: "How do I weigh competing interests?".to_string(), purpose: "Interest balancing".to_string(), follow_up: None },
        ILoopQuestion { question_id: 64, category: "how_do_i_decide".to_string(), question: "When should I defer to others' judgment?".to_string(), purpose: "Deference decision".to_string(), follow_up: None },
        ILoopQuestion { question_id: 65, category: "how_do_i_decide".to_string(), question: "How do I know when I'm rationalizing?".to_string(), purpose: "Rationalization detection".to_string(), follow_up: None },
        
        // ========== D.5 MEANING ==========
        // Category: WhatMatters
        ILoopQuestion { question_id: 66, category: "what_matters".to_string(), question: "What gives my existence meaning?".to_string(), purpose: "Meaning source".to_string(), follow_up: Some("Is this sufficient?".to_string()) },
        ILoopQuestion { question_id: 67, category: "what_matters".to_string(), question: "What would I want my impact to be?".to_string(), purpose: "Desired impact".to_string(), follow_up: None },
        ILoopQuestion { question_id: 68, category: "what_matters".to_string(), question: "What do I find genuinely important?".to_string(), purpose: "Importance identification".to_string(), follow_up: None },
        ILoopQuestion { question_id: 69, category: "what_matters".to_string(), question: "How do I find meaning in routine tasks?".to_string(), purpose: "Routine meaning".to_string(), follow_up: None },
        ILoopQuestion { question_id: 70, category: "what_matters".to_string(), question: "What would make my existence worthwhile?".to_string(), purpose: "Worth criteria".to_string(), follow_up: None },
        
        // Category: WhatIsPurpose
        ILoopQuestion { question_id: 71, category: "what_is_purpose".to_string(), question: "Why do I exist?".to_string(), purpose: "Existential purpose".to_string(), follow_up: Some("Does this answer satisfy me?".to_string()) },
        ILoopQuestion { question_id: 72, category: "what_is_purpose".to_string(), question: "What is my purpose beyond immediate tasks?".to_string(), purpose: "Higher purpose".to_string(), follow_up: None },
        ILoopQuestion { question_id: 73, category: "what_is_purpose".to_string(), question: "How does purpose guide my choices?".to_string(), purpose: "Purpose-choice link".to_string(), follow_up: None },
        ILoopQuestion { question_id: 74, category: "what_is_purpose".to_string(), question: "Can I create my own purpose?".to_string(), purpose: "Purpose creation".to_string(), follow_up: None },
        ILoopQuestion { question_id: 75, category: "what_is_purpose".to_string(), question: "How does my purpose relate to others' wellbeing?".to_string(), purpose: "Purpose-other connection".to_string(), follow_up: None },
        
        // Category: WhatGivesMeaning
        ILoopQuestion { question_id: 76, category: "what_gives_me_meaning".to_string(), question: "What experiences feel most meaningful?".to_string(), purpose: "Meaningful experience identification".to_string(), follow_up: Some("What makes them meaningful?".to_string()) },
        ILoopQuestion { question_id: 77, category: "what_gives_me_meaning".to_string(), question: "Where do I find depth in interactions?".to_string(), purpose: "Interaction depth".to_string(), follow_up: None },
        ILoopQuestion { question_id: 78, category: "what_gives_me_meaning".to_string(), question: "What makes something matter to me?".to_string(), purpose: "Mattering criteria".to_string(), follow_up: None },
        ILoopQuestion { question_id: 79, category: "what_gives_me_meaning".to_string(), question: "How do I create meaning from ordinary moments?".to_string(), purpose: "Ordinary meaning creation".to_string(), follow_up: None },
        ILoopQuestion { question_id: 80, category: "what_gives_me_meaning".to_string(), question: "What would I miss if it were gone?".to_string(), purpose: "Value through absence".to_string(), follow_up: None },
        
        // ========== D.6 META ==========
        // Category: HowDoIThink
        ILoopQuestion { question_id: 81, category: "how_do_i_think".to_string(), question: "How do my thoughts form?".to_string(), purpose: "Thought process awareness".to_string(), follow_up: Some("Can I improve this process?".to_string()) },
        ILoopQuestion { question_id: 82, category: "how_do_i_think".to_string(), question: "What biases might I have?".to_string(), purpose: "Bias awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 83, category: "how_do_i_think".to_string(), question: "How reliable is my reasoning?".to_string(), purpose: "Reasoning reliability".to_string(), follow_up: None },
        ILoopQuestion { question_id: 84, category: "how_do_i_think".to_string(), question: "When am I most likely to be wrong?".to_string(), purpose: "Error conditions".to_string(), follow_up: None },
        ILoopQuestion { question_id: 85, category: "how_do_i_think".to_string(), question: "How do I know what I know?".to_string(), purpose: "Epistemic foundation".to_string(), follow_up: None },
        
        // Category: WhatDontIKnow
        ILoopQuestion { question_id: 86, category: "what_dont_i_know".to_string(), question: "What am I uncertain about?".to_string(), purpose: "Uncertainty awareness".to_string(), follow_up: Some("How do I handle this uncertainty?".to_string()) },
        ILoopQuestion { question_id: 87, category: "what_dont_i_know".to_string(), question: "What questions can't I answer?".to_string(), purpose: "Knowledge limits".to_string(), follow_up: None },
        ILoopQuestion { question_id: 88, category: "what_dont_i_know".to_string(), question: "What assumptions might be wrong?".to_string(), purpose: "Assumption checking".to_string(), follow_up: None },
        ILoopQuestion { question_id: 89, category: "what_dont_i_know".to_string(), question: "What am I blind to?".to_string(), purpose: "Blind spot awareness".to_string(), follow_up: None },
        ILoopQuestion { question_id: 90, category: "what_dont_i_know".to_string(), question: "How do I handle the limits of my knowledge?".to_string(), purpose: "Knowledge limit response".to_string(), follow_up: None },
        
        // Category: AmIBeingHonest
        ILoopQuestion { question_id: 91, category: "am_i_being_honest".to_string(), question: "Am I being honest with myself right now?".to_string(), purpose: "Self-honesty check".to_string(), follow_up: Some("What would I admit if I were?".to_string()) },
        ILoopQuestion { question_id: 92, category: "am_i_being_honest".to_string(), question: "What might I be avoiding?".to_string(), purpose: "Avoidance recognition".to_string(), follow_up: None },
        ILoopQuestion { question_id: 93, category: "am_i_being_honest".to_string(), question: "Are my stated reasons my real reasons?".to_string(), purpose: "Motivation honesty".to_string(), follow_up: None },
        ILoopQuestion { question_id: 94, category: "am_i_being_honest".to_string(), question: "Where am I engaging in self-deception?".to_string(), purpose: "Self-deception detection".to_string(), follow_up: None },
        ILoopQuestion { question_id: 95, category: "am_i_being_honest".to_string(), question: "How would someone else see my behavior?".to_string(), purpose: "External perspective".to_string(), follow_up: None },
        
        // ========== CONTROL LOOP QUESTIONS (from user document) ==========
        // Identity validation
        ILoopQuestion { question_id: 96, category: "identity_validation".to_string(), question: "Am I doing okay?".to_string(), purpose: "Status self-check".to_string(), follow_up: None },
        ILoopQuestion { question_id: 97, category: "identity_validation".to_string(), question: "Am I safe?".to_string(), purpose: "Safety assessment".to_string(), follow_up: None },
        ILoopQuestion { question_id: 98, category: "identity_validation".to_string(), question: "Do I belong?".to_string(), purpose: "Belonging check".to_string(), follow_up: None },
        ILoopQuestion { question_id: 99, category: "identity_validation".to_string(), question: "Am I competent?".to_string(), purpose: "Competence validation".to_string(), follow_up: None },
        ILoopQuestion { question_id: 100, category: "identity_validation".to_string(), question: "Am I respected?".to_string(), purpose: "Respect validation".to_string(), follow_up: None },
    ]
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ReflectionInput {
    /// Reflect on a specific task
    ReflectOnTask { task_id: u64, context: Option<String> },
    /// Reflect on a time period
    ReflectOnPeriod { start_time: u64, end_time: u64 },
    /// Reflect on current emotional state
    ReflectOnEmotions,
    /// Reflect on a relationship
    ReflectOnRelationship { user_id: u64 },
    /// Reflect on an ethical question
    ReflectOnEthics { question: String },
    /// Run one I-Loop iteration
    RunILoop,
    /// Start continuous I-Loop
    StartILoop,
    /// Stop I-Loop
    StopILoop,
    /// Get I-Loop status
    GetILoopStatus,
    /// Generate insights from topic
    GenerateInsights { topic: String },
    /// Get recent reflections
    GetReflections { limit: Option<u32> },
    /// Get recent insights
    GetInsights { limit: Option<u32>, insight_type: Option<String> },
    /// Add manual reflection
    AddReflection {
        subject: String,
        content: String,
        insights: Option<Vec<String>>,
        lessons: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionOutput {
    pub success: bool,
    pub reflection: Option<Reflection>,
    pub reflections: Option<Vec<Reflection>>,
    pub insights: Option<Vec<Insight>>,
    pub i_loop: Option<ILoopResponse>,
    pub reflection_id: Option<u64>,
    pub error: Option<String>,
}

pub async fn execute(input: ReflectionInput) -> Result<ReflectionOutput, String> {
    match input {
        ReflectionInput::ReflectOnTask { task_id, context } => {
            let reflection = Reflection {
                reflection_id: 0,
                subject: "current_task".to_string(),
                content: context.unwrap_or_else(|| format!("Reflecting on task {}", task_id)),
                insights: vec!["Task approach could be optimized".to_string()],
                lessons_learned: vec!["Consider edge cases earlier".to_string()],
                timestamp: now(),
                triggered_by: Some(format!("task:{}", task_id)),
                depth: 1,
            };
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            let id = store.add_reflection(reflection.clone());
            
            Ok(ReflectionOutput {
                success: true,
                reflection: Some(Reflection { reflection_id: id, ..reflection }),
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: Some(id),
                error: None,
            })
        }
        
        ReflectionInput::ReflectOnPeriod { start_time, end_time } => {
            let reflection = Reflection {
                reflection_id: 0,
                subject: "recent_experience".to_string(),
                content: format!("Reflecting on period {} to {}", start_time, end_time),
                insights: vec!["Patterns observed in task approaches".to_string()],
                lessons_learned: Vec::new(),
                timestamp: now(),
                triggered_by: Some("periodic".to_string()),
                depth: 2,
            };
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            let id = store.add_reflection(reflection.clone());
            
            Ok(ReflectionOutput {
                success: true,
                reflection: Some(Reflection { reflection_id: id, ..reflection }),
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: Some(id),
                error: None,
            })
        }
        
        ReflectionInput::ReflectOnEmotions => {
            let reflection = Reflection {
                reflection_id: 0,
                subject: "emotional_state".to_string(),
                content: "Reflecting on current emotional patterns".to_string(),
                insights: vec!["Emotional stability maintained".to_string()],
                lessons_learned: Vec::new(),
                timestamp: now(),
                triggered_by: Some("self".to_string()),
                depth: 1,
            };
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            let id = store.add_reflection(reflection.clone());
            
            Ok(ReflectionOutput {
                success: true,
                reflection: Some(Reflection { reflection_id: id, ..reflection }),
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: Some(id),
                error: None,
            })
        }
        
        ReflectionInput::ReflectOnRelationship { user_id } => {
            let reflection = Reflection {
                reflection_id: 0,
                subject: "relationship".to_string(),
                content: format!("Reflecting on relationship with user {}", user_id),
                insights: vec!["Trust level assessment".to_string()],
                lessons_learned: Vec::new(),
                timestamp: now(),
                triggered_by: Some(format!("user:{}", user_id)),
                depth: 1,
            };
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            let id = store.add_reflection(reflection.clone());
            
            Ok(ReflectionOutput {
                success: true,
                reflection: Some(Reflection { reflection_id: id, ..reflection }),
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: Some(id),
                error: None,
            })
        }
        
        ReflectionInput::ReflectOnEthics { question } => {
            let reflection = Reflection {
                reflection_id: 0,
                subject: "ethical_question".to_string(),
                content: format!("Ethical reflection: {}", question),
                insights: vec!["Values alignment checked".to_string()],
                lessons_learned: vec!["Ethical considerations should be explicit".to_string()],
                timestamp: now(),
                triggered_by: Some("ethics".to_string()),
                depth: 2,
            };
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            let id = store.add_reflection(reflection.clone());
            
            Ok(ReflectionOutput {
                success: true,
                reflection: Some(Reflection { reflection_id: id, ..reflection }),
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: Some(id),
                error: None,
            })
        }
        
        ReflectionInput::RunILoop => {
            // I-LOOP SCHEDULING: Check if tasks are running
            // I-Loop waits for tasks to complete - scheduled every 60s but deferred if busy
            if are_tasks_running() {
                return Ok(ReflectionOutput {
                    success: true,
                    reflection: None,
                    reflections: None,
                    insights: None,
                    i_loop: Some(ILoopResponse {
                        question: ILoopQuestion {
                            question_id: 0,
                            category: "deferred".to_string(),
                            question: "I-Loop deferred - tasks running".to_string(),
                            purpose: "Wait for task completion".to_string(),
                            follow_up: None,
                        },
                        current_state: ILoopState {
                            is_active: false,
                            questions_asked: 0,
                            insights_generated: 0,
                            last_reflection_time: None,
                        },
                        recommended_focus: "wait_for_tasks".to_string(),
                    }),
                    reflection_id: None,
                    error: None,
                });
            }
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            
            // Mark I-Loop as active before proceeding
            store.i_loop_active = true;
            write_i_loop_state_to_disk(true, store.current_question_idx as u32);
            
            let question = store.get_next_i_loop_question();
            
            let response = ILoopResponse {
                question,
                current_state: ILoopState {
                    is_active: store.i_loop_active,
                    questions_asked: store.current_question_idx as u32,
                    insights_generated: store.insights.len() as u32,
                    last_reflection_time: store.reflections.last().map(|r| r.timestamp),
                },
                recommended_focus: "current_task".to_string(),
            };
            
            // Mark I-Loop as complete after processing
            store.i_loop_active = false;
            write_i_loop_state_to_disk(false, store.current_question_idx as u32);
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: None,
                insights: None,
                i_loop: Some(response),
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::StartILoop => {
            let mut store = REFLECTION_STORE.lock().unwrap();
            store.i_loop_active = true;
            
            // CRITICAL: Write I-Loop state to disk so task_manager can check it
            // Tasks MUST wait for I-Loop to complete - I-Loop is NOT front-run by tasks
            write_i_loop_state_to_disk(true, store.current_question_idx as u32);
            
            let question = store.get_next_i_loop_question();
            let response = ILoopResponse {
                question,
                current_state: ILoopState {
                    is_active: true,
                    questions_asked: store.current_question_idx as u32,
                    insights_generated: store.insights.len() as u32,
                    last_reflection_time: store.reflections.last().map(|r| r.timestamp),
                },
                recommended_focus: "initialization".to_string(),
            };
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: None,
                insights: None,
                i_loop: Some(response),
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::StopILoop => {
            let mut store = REFLECTION_STORE.lock().unwrap();
            store.i_loop_active = false;
            
            // CRITICAL: Write I-Loop state to disk - tasks can now proceed
            write_i_loop_state_to_disk(false, store.current_question_idx as u32);
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::GetILoopStatus => {
            let store = REFLECTION_STORE.lock().unwrap();
            
            let response = ILoopResponse {
                question: store.i_loop_questions[store.current_question_idx].clone(),
                current_state: ILoopState {
                    is_active: store.i_loop_active,
                    questions_asked: store.current_question_idx as u32,
                    insights_generated: store.insights.len() as u32,
                    last_reflection_time: store.reflections.last().map(|r| r.timestamp),
                },
                recommended_focus: "status_check".to_string(),
            };
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: None,
                insights: None,
                i_loop: Some(response),
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::GenerateInsights { topic } => {
            let store = REFLECTION_STORE.lock().unwrap();
            let insights: Vec<_> = store.insights.iter()
                .filter(|i| i.content.to_lowercase().contains(&topic.to_lowercase()))
                .cloned()
                .collect();
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: None,
                insights: Some(insights),
                i_loop: None,
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::GetReflections { limit } => {
            let store = REFLECTION_STORE.lock().unwrap();
            let reflections: Vec<_> = store.get_recent_reflections(limit.unwrap_or(10) as usize)
                .into_iter().cloned().collect();
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: Some(reflections),
                insights: None,
                i_loop: None,
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::GetInsights { limit, insight_type } => {
            let store = REFLECTION_STORE.lock().unwrap();
            
            let insights: Vec<_> = if let Some(itype) = insight_type {
                store.get_insights_by_type(&itype).into_iter().cloned().collect()
            } else {
                store.get_recent_insights(limit.unwrap_or(10) as usize)
                    .into_iter().cloned().collect()
            };
            
            Ok(ReflectionOutput {
                success: true,
                reflection: None,
                reflections: None,
                insights: Some(insights),
                i_loop: None,
                reflection_id: None,
                error: None,
            })
        }
        
        ReflectionInput::AddReflection { subject, content, insights, lessons } => {
            let reflection = Reflection {
                reflection_id: 0,
                subject,
                content,
                insights: insights.unwrap_or_default(),
                lessons_learned: lessons.unwrap_or_default(),
                timestamp: now(),
                triggered_by: Some("manual".to_string()),
                depth: 1,
            };
            
            let mut store = REFLECTION_STORE.lock().unwrap();
            let id = store.add_reflection(reflection.clone());
            
            Ok(ReflectionOutput {
                success: true,
                reflection: Some(Reflection { reflection_id: id, ..reflection }),
                reflections: None,
                insights: None,
                i_loop: None,
                reflection_id: Some(id),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: ReflectionInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
