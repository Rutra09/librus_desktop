use serde::Deserialize;

// ---------------------------------------------------------------------------
// Generic ID reference used in many Librus API responses: { "Id": 123 }
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IdRef {
    #[serde(alias = "Id", default, deserialize_with = "deserialize_id")]
    pub id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lesson {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "LessonNo", default, deserialize_with = "deserialize_int")]
    pub lesson_no: Option<i32>,
    #[serde(alias = "HourFrom")]
    pub hour_from: Option<String>,
    #[serde(alias = "HourTo")]
    pub hour_to: Option<String>,
    #[serde(alias = "Subject")]
    pub subject: Option<IdRef>,
    #[serde(alias = "Teacher")]
    pub teacher: Option<IdRef>,
    #[serde(alias = "Classroom")]
    pub classroom: Option<IdRef>,
    #[serde(alias = "Class")]
    pub class: Option<IdRef>,
    #[serde(alias = "VirtualClass")]
    pub virtual_class: Option<IdRef>,
    #[serde(alias = "IsCanceled")]
    pub is_canceled: Option<bool>,
    #[serde(alias = "IsSubstitutionClass")]
    pub is_substitution_class: Option<bool>,
    // Substitution original data
    #[serde(alias = "OrgSubject")]
    pub org_subject: Option<IdRef>,
    #[serde(alias = "OrgTeacher")]
    pub org_teacher: Option<IdRef>,
    #[serde(alias = "OrgDate")]
    pub org_date: Option<String>,
    #[serde(alias = "OrgLessonNo", default, deserialize_with = "deserialize_int")]
    pub org_lesson_no: Option<i32>,
    #[serde(alias = "OrgHourFrom")]
    pub org_hour_from: Option<String>,
    #[serde(alias = "OrgHourTo")]
    pub org_hour_to: Option<String>,
    // Shifted lesson data
    #[serde(alias = "NewDate")]
    pub new_date: Option<String>,
    #[serde(alias = "NewLessonNo", default, deserialize_with = "deserialize_int")]
    pub new_lesson_no: Option<i32>,
    #[serde(alias = "NewHourFrom")]
    pub new_hour_from: Option<String>,
    #[serde(alias = "NewHourTo")]
    pub new_hour_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Me (user info)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct MeResponse {
    #[serde(alias = "Me")]
    pub me: MeData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MeData {
    #[serde(alias = "Account")]
    pub account: Option<AccountInfo>,
    #[serde(alias = "User")]
    pub user: Option<UserInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountInfo {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "GroupId")]
    pub group_id: Option<i32>,
    #[serde(alias = "FirstName")]
    pub first_name: Option<String>,
    #[serde(alias = "LastName")]
    pub last_name: Option<String>,
    #[serde(alias = "Login")]
    pub login: Option<String>,
    #[serde(alias = "IsPremium")]
    pub is_premium: Option<bool>,
    #[serde(alias = "IsPremiumDemo")]
    pub is_premium_demo: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    #[serde(alias = "FirstName")]
    pub first_name: Option<String>,
    #[serde(alias = "LastName")]
    pub last_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Grades
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GradesResponse {
    #[serde(alias = "Grades")]
    pub grades: Option<Vec<Grade>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Grade {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Grade")]
    pub grade: Option<String>,
    #[serde(alias = "Semester")]
    pub semester: Option<i32>,
    #[serde(alias = "IsConstituent")]
    pub is_constituent: Option<bool>,
    #[serde(alias = "IsSemester")]
    pub is_semester: Option<bool>,
    #[serde(alias = "IsSemesterProposition")]
    pub is_semester_proposition: Option<bool>,
    #[serde(alias = "IsFinal")]
    pub is_final: Option<bool>,
    #[serde(alias = "IsFinalProposition")]
    pub is_final_proposition: Option<bool>,
    #[serde(alias = "AddDate")]
    pub add_date: Option<String>,
    #[serde(alias = "Category")]
    pub category: Option<IdRef>,
    #[serde(alias = "Subject")]
    pub subject: Option<IdRef>,
    #[serde(alias = "AddedBy")]
    pub added_by: Option<IdRef>,
    #[serde(alias = "Comments")]
    pub comments: Option<Vec<IdRef>>,
    #[serde(alias = "Improvement")]
    pub improvement: Option<IdRef>,
}

impl Grade {
    /// Determine the grade type based on flags.
    pub fn grade_type(&self) -> GradeType {
        let semester = self.semester.unwrap_or(1);
        if self.is_semester.unwrap_or(false) {
            if semester == 1 { GradeType::Semester1Final } else { GradeType::Semester2Final }
        } else if self.is_semester_proposition.unwrap_or(false) {
            if semester == 1 { GradeType::Semester1Proposed } else { GradeType::Semester2Proposed }
        } else if self.is_final.unwrap_or(false) {
            GradeType::YearFinal
        } else if self.is_final_proposition.unwrap_or(false) {
            GradeType::YearProposed
        } else {
            GradeType::Normal
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GradeType {
    Normal,
    Semester1Final,
    Semester1Proposed,
    Semester2Final,
    Semester2Proposed,
    YearFinal,
    YearProposed,
}

// ---------------------------------------------------------------------------
// Grade Categories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GradeCategoriesResponse {
    #[serde(alias = "Categories")]
    pub categories: Option<Vec<GradeCategory>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradeCategory {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "Weight")]
    pub weight: Option<f32>,
    #[serde(alias = "Color")]
    pub color: Option<IdRef>,
}

// ---------------------------------------------------------------------------
// Grade Comments
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GradeCommentsResponse {
    #[serde(alias = "Comments")]
    pub comments: Option<Vec<GradeComment>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradeComment {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Text")]
    pub text: Option<String>,
}

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct SubjectsResponse {
    #[serde(alias = "Subjects")]
    pub subjects: Option<Vec<Subject>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subject {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "Short")]
    pub short: Option<String>,
}

// ---------------------------------------------------------------------------
// Teachers / Users
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct UsersResponse {
    #[serde(alias = "Users")]
    pub users: Option<Vec<Teacher>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Teacher {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "FirstName")]
    pub first_name: Option<String>,
    #[serde(alias = "LastName")]
    pub last_name: Option<String>,
}

impl Teacher {
    pub fn full_name(&self) -> String {
        format!(
            "{} {}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or("")
        )
        .trim()
        .to_string()
    }
}

// ---------------------------------------------------------------------------
// Timetable
// ---------------------------------------------------------------------------

/// The timetable response: { "Timetable": { "2024-01-01": [ [lesson, ...], ... ], ... } }
/// Each day is an array of slots; each slot is an array of lessons (for overlapping classes).
#[derive(Debug, Clone, Deserialize)]
pub struct TimetableResponse {
    #[serde(alias = "Timetable")]
    pub timetable: Option<serde_json::Value>,
}


// ---------------------------------------------------------------------------
// Classrooms
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ClassroomsResponse {
    #[serde(alias = "Classrooms")]
    pub classrooms: Option<Vec<Classroom>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Classroom {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "Symbol")]
    pub symbol: Option<String>,
}

// ---------------------------------------------------------------------------
// Attendances
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct AttendancesResponse {
    #[serde(alias = "Attendances")]
    pub attendances: Option<Vec<Attendance>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Attendance {
    #[serde(alias = "Id")]
    pub id: Option<serde_json::Value>, // can be string or number
    #[serde(alias = "LessonNo")]
    pub lesson_no: Option<i32>,
    #[serde(alias = "Date")]
    pub date: Option<String>,
    #[serde(alias = "AddDate")]
    pub add_date: Option<String>,
    #[serde(alias = "Semester")]
    pub semester: Option<i32>,
    #[serde(alias = "Type")]
    pub attendance_type: Option<IdRef>,
    #[serde(alias = "Lesson")]
    pub lesson: Option<IdRef>,
    #[serde(alias = "Subject")]
    pub subject: Option<IdRef>,
    #[serde(alias = "AddedBy")]
    pub added_by: Option<IdRef>,
}

// ---------------------------------------------------------------------------
// Attendance Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct AttendanceTypesResponse {
    #[serde(alias = "Types")]
    pub types: Option<Vec<AttendanceType>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttendanceType {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "Short")]
    pub short: Option<String>,
    #[serde(alias = "IsPresenceKind")]
    pub is_presence_kind: Option<bool>,
    #[serde(alias = "ColorRGB")]
    pub color_rgb: Option<String>,
}

// ---------------------------------------------------------------------------
// Events / Homework
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct EventsResponse {
    #[serde(alias = "HomeWorks")]
    pub homeworks: Option<Vec<Event>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    #[serde(alias = "Id", default, deserialize_with = "deserialize_id")]
    pub id: Option<i64>,
    #[serde(alias = "Date")]
    pub date: Option<String>,
    #[serde(alias = "Content")]
    pub content: Option<String>,
    #[serde(alias = "LessonNo", default, deserialize_with = "deserialize_int")]
    pub lesson_no: Option<i32>,
    #[serde(alias = "TimeFrom")]
    pub time_from: Option<String>,
    #[serde(alias = "AddDate")]
    pub add_date: Option<String>,
    #[serde(alias = "Category")]
    pub category: Option<IdRef>,
    #[serde(alias = "Subject")]
    pub subject: Option<IdRef>,
    #[serde(alias = "CreatedBy")]
    pub created_by: Option<IdRef>,
    #[serde(alias = "Class")]
    pub class: Option<IdRef>,
    
    // Transient fields for scraped data
    #[serde(skip)]
    pub scraped_subject: Option<String>,
    #[serde(skip)]
    pub scraped_teacher: Option<String>,
    #[serde(skip)]
    pub scraped_category: Option<String>,
}

fn deserialize_id<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_i64()),
        serde_json::Value::String(s) => Ok(s.parse::<i64>().ok()),
        _ => Ok(None),
    }
}

fn deserialize_int<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_i64().map(|x| x as i32)),
        serde_json::Value::String(s) => Ok(s.parse::<i32>().ok()),
        _ => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Event Types / Homework Categories
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct EventTypesResponse {
    #[serde(alias = "Categories")]
    pub categories: Option<Vec<EventType>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventType {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomeworkCategoryResponse {
    #[serde(alias = "Categories")]
    pub categories: Option<Vec<HomeworkCategory>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomeworkCategory {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "Color")]
    pub color: Option<IdRef>,
}

// ---------------------------------------------------------------------------
// Announcements (SchoolNotices)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct AnnouncementsResponse {
    #[serde(alias = "SchoolNotices")]
    pub school_notices: Option<Vec<Announcement>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Announcement {
    #[serde(alias = "Id")]
    pub id: Option<String>,
    #[serde(alias = "Subject")]
    pub subject: Option<String>,
    #[serde(alias = "Content")]
    pub content: Option<String>,
    #[serde(alias = "StartDate")]
    pub start_date: Option<String>,
    #[serde(alias = "EndDate")]
    pub end_date: Option<String>,
    #[serde(alias = "AddedBy")]
    pub added_by: Option<IdRef>,
    #[serde(alias = "CreationDate")]
    pub creation_date: Option<String>,
    #[serde(alias = "WasRead")]
    pub was_read: Option<bool>,
}

// ---------------------------------------------------------------------------
// Lucky Number
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct LuckyNumberResponse {
    #[serde(alias = "LuckyNumber")]
    pub lucky_number: Option<LuckyNumber>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LuckyNumber {
    #[serde(alias = "LuckyNumber")]
    pub lucky_number: Option<i32>,
    #[serde(alias = "LuckyNumberDay")]
    pub lucky_number_day: Option<String>,
}

// ---------------------------------------------------------------------------
// Schools
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct SchoolResponse {
    #[serde(alias = "School")]
    pub school: Option<School>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct School {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "Town")]
    pub town: Option<String>,
}

// ---------------------------------------------------------------------------
// Classes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ClassesResponse {
    #[serde(alias = "Class")]
    pub class: Option<Class>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Class {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Number")]
    pub number: Option<i32>,
    #[serde(alias = "Symbol")]
    pub symbol: Option<String>,
}

// ---------------------------------------------------------------------------
// Notices (student behaviour notes)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct NoticesResponse {
    #[serde(alias = "Notes")]
    pub notes: Option<Vec<Notice>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Notice {
    #[serde(alias = "Id")]
    pub id: Option<i64>,
    #[serde(alias = "Text")]
    pub text: Option<String>,
    #[serde(alias = "Category")]
    pub category: Option<IdRef>,
    #[serde(alias = "AddedBy")]
    pub added_by: Option<IdRef>,
    #[serde(alias = "Date")]
    pub date: Option<String>,
    #[serde(alias = "Positive")]
    pub positive: Option<bool>,
}

// ---------------------------------------------------------------------------
// Lesson Ranges (bell schedule)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct LessonRangesResponse {
    #[serde(alias = "Lessons")]
    pub lessons: Option<Vec<LessonRange>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LessonRange {
    #[serde(alias = "LessonNo")]
    pub lesson_no: Option<i32>,
    #[serde(alias = "HourFrom")]
    pub hour_from: Option<String>,
    #[serde(alias = "HourTo")]
    pub hour_to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LessonsResponse {
    #[serde(alias = "Lessons")]
    pub lessons: Option<Vec<Lesson>>,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub subject: String,
    pub body: Option<String>,
    pub sender_name: String,
    pub send_date: String,
    pub read_date: Option<String>,
    pub has_attachments: bool,
    pub message_type: MessageType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Received,
    Sent,
}
