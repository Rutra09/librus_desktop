use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;
use std::collections::HashMap;

use crate::api::client::LibrusClient;
use crate::api::models::{MeResponse, LuckyNumber, Lesson, Grade, Announcement};

/// Global application state shared with the UI thread.
#[derive(Clone)]
pub struct AppState {
    pub client: Option<LibrusClient>,
    pub me: Arc<Mutex<Option<MeResponse>>>,
    pub lucky_number: Arc<Mutex<Option<LuckyNumber>>>,
    
    // Cached Metadata
    pub subjects: Arc<Mutex<HashMap<i64, String>>>,
    pub teachers: Arc<Mutex<HashMap<i64, String>>>,
    // Grade Categories (ID -> Category)
    pub grade_categories: Arc<Mutex<HashMap<i64, crate::api::models::GradeCategory>>>,
    // Event Categories (ID -> Name)
    pub event_categories: Arc<Mutex<HashMap<i64, String>>>,
    // Attendance Types (ID -> Type)
    pub attendance_types: Arc<Mutex<HashMap<i64, crate::api::models::AttendanceType>>>,
    // Classrooms (ID -> Classroom)
    pub classrooms: Arc<Mutex<HashMap<i64, crate::api::models::Classroom>>>,
    // Cached Grades
    pub grades: Arc<Mutex<Vec<Grade>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            client: None,
            me: Arc::new(Mutex::new(None)),
            lucky_number: Arc::new(Mutex::new(None)),
            subjects: Arc::new(Mutex::new(HashMap::new())),
            teachers: Arc::new(Mutex::new(HashMap::new())),
            grade_categories: Arc::new(Mutex::new(HashMap::new())),
            event_categories: Arc::new(Mutex::new(HashMap::new())),
            attendance_types: Arc::new(Mutex::new(HashMap::new())),
            classrooms: Arc::new(Mutex::new(HashMap::new())),
            grades: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_client(&mut self, client: LibrusClient) {
        self.client = Some(client);
    }

    pub fn is_logged_in(&self) -> bool {
        self.client.is_some()
    }
}
