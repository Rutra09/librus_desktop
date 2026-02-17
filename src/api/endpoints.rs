use anyhow::Result;

use super::client::LibrusClient;
use super::models::*;

impl LibrusClient {
    // -----------------------------------------------------------------------
    // User info
    // -----------------------------------------------------------------------

    /// Fetch current user information (`GET Me`).
    pub async fn fetch_me(&self) -> Result<MeResponse> {
        let json = self.api_get("Me").await?;
        let me: MeResponse = serde_json::from_value(json)?;
        Ok(me)
    }

    // -----------------------------------------------------------------------
    // Grades
    // -----------------------------------------------------------------------

    /// Fetch all grades (`GET Grades`).
    pub async fn fetch_grades(&self) -> Result<Vec<Grade>> {
        let json = self.api_get("Grades").await?;
        let resp: GradesResponse = serde_json::from_value(json)?;
        Ok(resp.grades.unwrap_or_default())
    }

    /// Fetch grade categories (`GET Grades/Categories`).
    pub async fn fetch_grade_categories(&self) -> Result<Vec<GradeCategory>> {
        let json = self.api_get("Grades/Categories").await?;
        let resp: GradeCategoriesResponse = serde_json::from_value(json)?;
        Ok(resp.categories.unwrap_or_default())
    }

    /// Fetch grade comments (`GET Grades/Comments`).
    pub async fn fetch_grade_comments(&self) -> Result<Vec<GradeComment>> {
        let json = self.api_get("Grades/Comments").await?;
        let resp: GradeCommentsResponse = serde_json::from_value(json)?;
        Ok(resp.comments.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Subjects & Teachers
    // -----------------------------------------------------------------------

    /// Fetch all subjects (`GET Subjects`).
    pub async fn fetch_subjects(&self) -> Result<Vec<Subject>> {
        let json = self.api_get("Subjects").await?;
        let resp: SubjectsResponse = serde_json::from_value(json)?;
        Ok(resp.subjects.unwrap_or_default())
    }

    /// Fetch all teachers/users (`GET Users`).
    pub async fn fetch_teachers(&self) -> Result<Vec<Teacher>> {
        let json = self.api_get("Users").await?;
        let resp: UsersResponse = serde_json::from_value(json)?;
        Ok(resp.users.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Timetable
    // -----------------------------------------------------------------------

    /// Fetch timetable for a given week (`GET Timetables?weekStart=YYYY-MM-DD`).
    ///
    /// The response contains a map of dates → array of lesson slots.
    /// Each slot is an array of lessons (usually 0 or 1, but can be more for overlapping).
    pub async fn fetch_timetable(&self, week_start: &str) -> Result<serde_json::Value> {
        let json = self
            .api_get(&format!("Timetables?weekStart={}", week_start))
            .await?;
        Ok(json
            .get("Timetable")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Parse a raw timetable JSON value into a structured map of date → Vec<Lesson>.
    pub fn parse_timetable(
        timetable: &serde_json::Value,
    ) -> Vec<(String, Vec<Lesson>)> {
        let mut days = Vec::new();

        if let Some(obj) = timetable.as_object() {
            let mut sorted_dates: Vec<_> = obj.keys().collect();
            sorted_dates.sort();

            for date in sorted_dates {
                if let Some(day_slots) = obj.get(date).and_then(|v| v.as_array()) {
                    let mut lessons = Vec::new();
                    for slot in day_slots {
                        if let Some(slot_lessons) = slot.as_array() {
                            for lesson_val in slot_lessons {
                            if let Ok(lesson) =
                                serde_json::from_value::<Lesson>(lesson_val.clone())
                            {
                                lessons.push(lesson);
                            }
                            }
                        }
                    }
                    days.push((date.clone(), lessons));
                }
            }
        }

        days
    }

    // -----------------------------------------------------------------------
    // Attendance
    // -----------------------------------------------------------------------

    /// Fetch all attendances (`GET Attendances`).
    pub async fn fetch_attendances(&self) -> Result<Vec<Attendance>> {
        let json = self.api_get("Attendances").await?;
        let resp: AttendancesResponse = serde_json::from_value(json)?;
        Ok(resp.attendances.unwrap_or_default())
    }

    /// Fetch attendance types (`GET Attendances/Types`).
    pub async fn fetch_attendance_types(&self) -> Result<Vec<AttendanceType>> {
        let json = self.api_get("Attendances/Types").await?;
        let resp: AttendanceTypesResponse = serde_json::from_value(json)?;
        Ok(resp.types.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Events / Homework
    // -----------------------------------------------------------------------

    /// Fetch all events/homework (`GET HomeWorks`).
    pub async fn fetch_events(&self) -> Result<Vec<Event>> {
        let json = self.api_get("HomeWorks").await?;
        let resp: EventsResponse = serde_json::from_value(json)?;
        Ok(resp.homeworks.unwrap_or_default())
    }

    /// Fetch event/homework categories (`GET HomeWorks/Categories`).
    pub async fn fetch_event_types(&self) -> Result<Vec<EventType>> {
        let json = self.api_get("HomeWorks/Categories").await?;
        let resp: EventTypesResponse = serde_json::from_value(json)?;
        Ok(resp.categories.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Announcements
    // -----------------------------------------------------------------------

    /// Fetch all school announcements (`GET SchoolNotices`).
    pub async fn fetch_announcements(&self) -> Result<Vec<Announcement>> {
        let json = self.api_get("SchoolNotices").await?;
        let resp: AnnouncementsResponse = serde_json::from_value(json)?;
        Ok(resp.school_notices.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Lucky Number
    // -----------------------------------------------------------------------

    /// Fetch today's lucky number (`GET LuckyNumbers`).
    pub async fn fetch_lucky_number(&self) -> Result<Option<LuckyNumber>> {
        let json = self.api_get("LuckyNumbers").await?;
        let resp: LuckyNumberResponse = serde_json::from_value(json)?;
        Ok(resp.lucky_number)
    }

    // -----------------------------------------------------------------------
    // School info
    // -----------------------------------------------------------------------

    /// Fetch school information (`GET Schools`).
    pub async fn fetch_school(&self) -> Result<Option<School>> {
        let json = self.api_get("Schools").await?;
        let resp: SchoolResponse = serde_json::from_value(json)?;
        Ok(resp.school)
    }

    // -----------------------------------------------------------------------
    // Class info
    // -----------------------------------------------------------------------

    /// Fetch class info (`GET Classes`).
    pub async fn fetch_class(&self) -> Result<Option<Class>> {
        let json = self.api_get("Classes").await?;
        let resp: ClassesResponse = serde_json::from_value(json)?;
        Ok(resp.class)
    }

    // -----------------------------------------------------------------------
    // Classrooms
    // -----------------------------------------------------------------------

    /// Fetch all classrooms (`GET Classrooms`).
    pub async fn fetch_classrooms(&self) -> Result<Vec<Classroom>> {
        let json = self.api_get("Classrooms").await?;
        let resp: ClassroomsResponse = serde_json::from_value(json)?;
        Ok(resp.classrooms.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Notices (behaviour notes)
    // -----------------------------------------------------------------------

    /// Fetch student behaviour notices (`GET Notes`).
    pub async fn fetch_notices(&self) -> Result<Vec<Notice>> {
        let json = self.api_get("Notes").await?;
        let resp: NoticesResponse = serde_json::from_value(json)?;
        Ok(resp.notes.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Lesson ranges (bell schedule)
    // -----------------------------------------------------------------------

    /// Fetch lesson time ranges / bell schedule (`GET Lessons`).
    pub async fn fetch_lesson_ranges(&self) -> Result<Vec<LessonRange>> {
        let json = self.api_get("Lessons").await?;
        let resp: LessonRangesResponse = serde_json::from_value(json)?;
        Ok(resp.lessons.unwrap_or_default())
    }

    /// Fetch details for specific lessons (`GET Lessons/id1,id2,...`).
    pub async fn fetch_lessons(&self, ids: Vec<i64>) -> Result<Vec<Lesson>> {
        if ids.is_empty() { return Ok(vec![]); }
        
        let ids_str = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        let json = self.api_get(&format!("Lessons/{}", ids_str)).await?;
        let resp: LessonsResponse = serde_json::from_value(json)?;
        Ok(resp.lessons.unwrap_or_default())
    }

    // -----------------------------------------------------------------------
    // Messages (Inbox only)
    // -----------------------------------------------------------------------

    /// Fetch messages from inbox using the XML API.
    /// Requires messages session to be initialized.
    pub async fn fetch_messages(&self) -> Result<Vec<super::models::Message>> {
        use super::messages_auth;
        use super::models::{Message, MessageType};
        
        // Ensure messages session is valid
        let now = chrono::Utc::now().timestamp();
        
        let session_id = if let Some((sid, exp)) = self.get_messages_session().await {
            if exp > now {
                sid
            } else {
                // Session expired, re-authenticate
                let auth_state = self.get_auth_state().await;
                let (new_sid, new_syn, new_exp) = messages_auth::login_messages(&auth_state).await?;
                self.set_messages_session(new_sid.clone(), new_syn, new_exp).await;
                new_sid
            }
        } else {
            // No session, authenticate
            let auth_state = self.get_auth_state().await;
            let (new_sid, new_syn, new_exp) = messages_auth::login_messages(&auth_state).await?;
            self.set_messages_session(new_sid.clone(), new_syn, new_exp).await;
            new_sid
        };

        // Build XML request
        let xml_body = r#"<service><header/><data><archive>0</archive></data></service>"#;

        // Make request to messages API
        let client = reqwest::Client::builder()
            .user_agent(super::constants::SYNERGIA_USER_AGENT)
            .cookie_store(true)
            .build()?;

        let response = client
            .post(&format!("{}/Inbox/action/GetList", super::constants::LIBRUS_MESSAGES_URL))
            .header("Content-Type", "application/xml")
            .header("Cookie", session_id)  // session_id now ONLY contains DZIENNIKSID (sanitized)
            .body(xml_body)
            .send()
            .await?;

        // println!("[Messages Fetch] Response status: {}", response.status());
        let mut text = response.text().await?;
        // println!("[Messages Fetch] Response body:\n{}", text);

        // Check for eAccessDeny or invalid session
        if text.contains("eAccessDeny") || text.contains("Brak dostępu") {
             println!("[Messages Fetch] eAccessDeny detected. Refreshing session...");
             // Refresh session
             let new_session_id = self.force_refresh_messages_session().await?;
             
             // Retry request
              let client = reqwest::Client::builder()
                .user_agent(super::constants::SYNERGIA_USER_AGENT)
                .cookie_store(true)
                .build()?;

            let response = client
                .post(&format!("{}/Inbox/action/GetList", super::constants::LIBRUS_MESSAGES_URL))
                .header("Content-Type", "application/xml")
                .header("Cookie", new_session_id) 
                .body(xml_body)
                .send()
                .await?;
            
             // println!("[Messages Fetch] Retry response status: {}", response.status());
             text = response.text().await?;
        }
        
        // println!("[Messages Fetch] Final Response body:\n{}", text);

        // Parse XML response
        let mut messages = Vec::new();
        
        if let Ok(doc) = roxmltree::Document::parse(&text) {
            // println!("[Messages Fetch] Successfully parsed XML");
            for node in doc.descendants() {
                if node.tag_name().name() == "item" || node.tag_name().name() == "ArrayItem" {
                    let id = node.descendants()
                        .find(|n| n.tag_name().name() == "messageId")
                        .and_then(|n| n.text())
                        .and_then(|t| t.parse::<i64>().ok())
                        .unwrap_or(0);
                    
                    let subject = node.descendants()
                        .find(|n| n.tag_name().name() == "topic")
                        .and_then(|n| n.text())
                        .unwrap_or("")
                        .to_string();
                    
                    let sender_first = node.descendants()
                        .find(|n| n.tag_name().name() == "senderFirstName")
                        .and_then(|n| n.text())
                        .unwrap_or("");
                    
                    let sender_last = node.descendants()
                        .find(|n| n.tag_name().name() == "senderLastName")
                        .and_then(|n| n.text())
                        .unwrap_or("");
                    
                    let sender_name = format!("{} {}", sender_first, sender_last).trim().to_string();
                    
                    let send_date = node.descendants()
                        .find(|n| n.tag_name().name() == "sendDate")
                        .and_then(|n| n.text())
                        .unwrap_or("")
                        .to_string();
                    
                    let read_date = node.descendants()
                        .find(|n| n.tag_name().name() == "readDate")
                        .and_then(|n| n.text())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());
                    
                    let has_attachments = node.descendants()
                        .find(|n| n.tag_name().name() == "isAnyFileAttached")
                        .and_then(|n| n.text())
                        .map(|t| t == "1")
                        .unwrap_or(false);
                    
                    if id > 0 {
                        messages.push(Message {
                            id,
                            subject,
                            body: None,
                            sender_name,
                            send_date,
                            read_date,
                            has_attachments,
                            message_type: MessageType::Received,
                        });
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Fetch a single message content by ID
    pub async fn fetch_message_content(&self, message_id: i32) -> Result<String> {
        let (session_id, _) = self.get_messages_session().await
            .ok_or_else(|| anyhow::anyhow!("Messages session not initialized"))?;

        let xml_body = format!(
            r#"<service><header/><data><messageId>{}</messageId><archive>0</archive></data></service>"#,
            message_id
        );

        let client = reqwest::Client::builder()
            .user_agent(super::constants::SYNERGIA_USER_AGENT)
            .build()?;

        let response = client
            .post(&format!("{}/GetMessage", super::constants::LIBRUS_MESSAGES_URL))
            .header("Content-Type", "application/xml")
            .header("Cookie", session_id)
            .body(xml_body)
            .send()
            .await?;

        // Fetch raw bytes to avoid reqwest's automatic (and potentially incorrect) encoding detection
        let bytes = response.bytes().await?;
        
        // Librus XMLs are generally UTF-8
        let (xml_str, _, _) = encoding_rs::UTF_8.decode(&bytes);
        
        // Parse XML to get content
        let doc = roxmltree::Document::parse(&xml_str)?;
        let content_node = doc.descendants()
            .find(|n| n.has_tag_name("Message"))
            .ok_or_else(|| anyhow::anyhow!("Message content not found in response"))?;

        let base64_content = content_node.text().unwrap_or("");
        
        // Decode Base64
        use base64::{Engine as _, engine::general_purpose};
        let decoded_bytes = general_purpose::STANDARD.decode(base64_content.trim())?;
        
        // Try decoding as UTF-8 first (most common and safe)
        let (decoded_str, _, malformed) = encoding_rs::UTF_8.decode(&decoded_bytes);
        let mut final_text = if malformed {
            // If UTF-8 is malformed, try Windows-1250 (common for Polish legacy systems)
            let (win_str, _, _) = encoding_rs::WINDOWS_1250.decode(&decoded_bytes);
            win_str.into_owned()
        } else {
            decoded_str.into_owned()
        };

        // Clean up nested XML or CDATA if present
        if final_text.contains("<Content>") {
            if let Ok(inner_doc) = roxmltree::Document::parse(&final_text) {
                if let Some(content) = inner_doc.descendants().find(|n| n.has_tag_name("Content")).and_then(|n| n.text()) {
                    final_text = content.to_string();
                }
            }
        }

        let cleaned_text = final_text
            .replace("<![CDATA[", "")
            .replace("]]>", "")
            .replace("<br>", "\n")
            .replace("<br />", "\n")
            .replace("&nbsp;", " ")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'");

        // Basic HTML tag stripping (e.g. for <a> tags)
        let mut result = String::new();
        let mut in_tag = false;
        for c in cleaned_text.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                result.push(c);
            }
        }

        Ok(result.trim().to_string())
    }
}
