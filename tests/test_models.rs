//! Tests for Librus API model deserialization.
//!
//! These use realistic JSON payloads matching what api.librus.pl/2.0 returns.

use librus_front::api::models::*;

// ---------------------------------------------------------------------------
// Me
// ---------------------------------------------------------------------------

#[test]
fn deserialize_me_response() {
    let json = serde_json::json!({
        "Me": {
            "Account": {
                "Id": 1234567,
                "GroupId": 7,
                "FirstName": "Jan",
                "LastName": "Kowalski",
                "Login": "1234567u",
                "IsPremium": false,
                "IsPremiumDemo": false
            },
            "User": {
                "FirstName": "Jan",
                "LastName": "Kowalski"
            }
        }
    });

    let me: MeResponse = serde_json::from_value(json).unwrap();
    let account = me.me.account.unwrap();
    let user = me.me.user.unwrap();

    assert_eq!(account.id, Some(1234567));
    assert_eq!(account.group_id, Some(7));
    assert_eq!(account.first_name.as_deref(), Some("Jan"));
    assert_eq!(account.last_name.as_deref(), Some("Kowalski"));
    assert_eq!(account.login.as_deref(), Some("1234567u"));
    assert_eq!(account.is_premium, Some(false));
    assert_eq!(user.first_name.as_deref(), Some("Jan"));
    assert_eq!(user.last_name.as_deref(), Some("Kowalski"));
}

#[test]
fn deserialize_me_parent_account() {
    let json = serde_json::json!({
        "Me": {
            "Account": {
                "Id": 9999999,
                "GroupId": 5,
                "FirstName": "Anna",
                "LastName": "Kowalska",
                "Login": "9999999r",
                "IsPremium": true
            },
            "User": {
                "FirstName": "Jan",
                "LastName": "Kowalski"
            }
        }
    });

    let me: MeResponse = serde_json::from_value(json).unwrap();
    let account = me.me.account.unwrap();
    // GroupId 5 = parent
    assert_eq!(account.group_id, Some(5));
    assert_eq!(account.is_premium, Some(true));
}

// ---------------------------------------------------------------------------
// Grades
// ---------------------------------------------------------------------------

#[test]
fn deserialize_grades_response() {
    let json = serde_json::json!({
        "Grades": [
            {
                "Id": 101,
                "Grade": "5",
                "Semester": 1,
                "IsConstituent": true,
                "IsSemester": false,
                "IsSemesterProposition": false,
                "IsFinal": false,
                "IsFinalProposition": false,
                "AddDate": "2024-09-15 08:30:00",
                "Category": { "Id": 10 },
                "Subject": { "Id": 3 },
                "AddedBy": { "Id": 42 },
                "Comments": [{ "Id": 201 }]
            },
            {
                "Id": 102,
                "Grade": "4+",
                "Semester": 1,
                "IsConstituent": true,
                "IsSemester": false,
                "IsSemesterProposition": false,
                "IsFinal": false,
                "IsFinalProposition": false,
                "AddDate": "2024-09-20 10:00:00",
                "Category": { "Id": 11 },
                "Subject": { "Id": 5 },
                "AddedBy": { "Id": 43 }
            }
        ]
    });

    let resp: GradesResponse = serde_json::from_value(json).unwrap();
    let grades = resp.grades.unwrap();

    assert_eq!(grades.len(), 2);
    assert_eq!(grades[0].id, Some(101));
    assert_eq!(grades[0].grade.as_deref(), Some("5"));
    assert_eq!(grades[0].semester, Some(1));
    assert_eq!(grades[0].is_constituent, Some(true));
    assert_eq!(grades[0].subject.as_ref().unwrap().id, Some(3));
    assert_eq!(grades[0].added_by.as_ref().unwrap().id, Some(42));
    assert!(grades[0].comments.is_some());
    assert_eq!(grades[0].comments.as_ref().unwrap().len(), 1);

    assert_eq!(grades[1].id, Some(102));
    assert_eq!(grades[1].grade.as_deref(), Some("4+"));
    assert!(grades[1].comments.is_none());
}

#[test]
fn grade_type_normal() {
    let grade = Grade {
        id: Some(1),
        grade: Some("5".into()),
        semester: Some(1),
        is_constituent: Some(true),
        is_semester: Some(false),
        is_semester_proposition: Some(false),
        is_final: Some(false),
        is_final_proposition: Some(false),
        add_date: None,
        category: None,
        subject: None,
        added_by: None,
        comments: None,
        improvement: None,
    };
    assert_eq!(grade.grade_type(), GradeType::Normal);
}

#[test]
fn grade_type_semester1_final() {
    let grade = Grade {
        id: Some(2),
        grade: Some("4".into()),
        semester: Some(1),
        is_constituent: Some(false),
        is_semester: Some(true),
        is_semester_proposition: Some(false),
        is_final: Some(false),
        is_final_proposition: Some(false),
        add_date: None,
        category: None,
        subject: None,
        added_by: None,
        comments: None,
        improvement: None,
    };
    assert_eq!(grade.grade_type(), GradeType::Semester1Final);
}

#[test]
fn grade_type_semester2_proposed() {
    let grade = Grade {
        id: Some(3),
        grade: Some("3".into()),
        semester: Some(2),
        is_constituent: Some(false),
        is_semester: Some(false),
        is_semester_proposition: Some(true),
        is_final: Some(false),
        is_final_proposition: Some(false),
        add_date: None,
        category: None,
        subject: None,
        added_by: None,
        comments: None,
        improvement: None,
    };
    assert_eq!(grade.grade_type(), GradeType::Semester2Proposed);
}

#[test]
fn grade_type_year_final() {
    let grade = Grade {
        id: Some(4),
        grade: Some("5".into()),
        semester: Some(2),
        is_constituent: Some(false),
        is_semester: Some(false),
        is_semester_proposition: Some(false),
        is_final: Some(true),
        is_final_proposition: Some(false),
        add_date: None,
        category: None,
        subject: None,
        added_by: None,
        comments: None,
        improvement: None,
    };
    assert_eq!(grade.grade_type(), GradeType::YearFinal);
}

#[test]
fn grade_type_year_proposed() {
    let grade = Grade {
        id: Some(5),
        grade: Some("4".into()),
        semester: Some(2),
        is_constituent: Some(false),
        is_semester: Some(false),
        is_semester_proposition: Some(false),
        is_final: Some(false),
        is_final_proposition: Some(true),
        add_date: None,
        category: None,
        subject: None,
        added_by: None,
        comments: None,
        improvement: None,
    };
    assert_eq!(grade.grade_type(), GradeType::YearProposed);
}

// ---------------------------------------------------------------------------
// Grade Categories
// ---------------------------------------------------------------------------

#[test]
fn deserialize_grade_categories() {
    let json = serde_json::json!({
        "Categories": [
            {
                "Id": 10,
                "Name": "Sprawdzian",
                "Weight": 3.0,
                "Color": { "Id": 1 }
            },
            {
                "Id": 11,
                "Name": "Kartkówka",
                "Weight": 1.0,
                "Color": { "Id": 2 }
            }
        ]
    });

    let resp: GradeCategoriesResponse = serde_json::from_value(json).unwrap();
    let cats = resp.categories.unwrap();

    assert_eq!(cats.len(), 2);
    assert_eq!(cats[0].id, Some(10));
    assert_eq!(cats[0].name.as_deref(), Some("Sprawdzian"));
    assert_eq!(cats[0].weight, Some(3.0));
    assert_eq!(cats[1].name.as_deref(), Some("Kartkówka"));
    assert_eq!(cats[1].weight, Some(1.0));
}

// ---------------------------------------------------------------------------
// Grade Comments
// ---------------------------------------------------------------------------

#[test]
fn deserialize_grade_comments() {
    let json = serde_json::json!({
        "Comments": [
            { "Id": 201, "Text": "Bardzo dobra praca" },
            { "Id": 202, "Text": "Poprawić błędy" }
        ]
    });

    let resp: GradeCommentsResponse = serde_json::from_value(json).unwrap();
    let comments = resp.comments.unwrap();

    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].id, Some(201));
    assert_eq!(comments[0].text.as_deref(), Some("Bardzo dobra praca"));
}

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

#[test]
fn deserialize_subjects() {
    let json = serde_json::json!({
        "Subjects": [
            { "Id": 1, "Name": "Matematyka", "Short": "mat" },
            { "Id": 2, "Name": "Język polski", "Short": "pol" },
            { "Id": 3, "Name": "Fizyka", "Short": "fiz" }
        ]
    });

    let resp: SubjectsResponse = serde_json::from_value(json).unwrap();
    let subjects = resp.subjects.unwrap();

    assert_eq!(subjects.len(), 3);
    assert_eq!(subjects[0].name.as_deref(), Some("Matematyka"));
    assert_eq!(subjects[0].short.as_deref(), Some("mat"));
    assert_eq!(subjects[2].name.as_deref(), Some("Fizyka"));
}

// ---------------------------------------------------------------------------
// Teachers
// ---------------------------------------------------------------------------

#[test]
fn deserialize_teachers() {
    let json = serde_json::json!({
        "Users": [
            { "Id": 42, "FirstName": "Maria", "LastName": "Nowak" },
            { "Id": 43, "FirstName": "Piotr", "LastName": "Wiśniewski" }
        ]
    });

    let resp: UsersResponse = serde_json::from_value(json).unwrap();
    let teachers = resp.users.unwrap();

    assert_eq!(teachers.len(), 2);
    assert_eq!(teachers[0].full_name(), "Maria Nowak");
    assert_eq!(teachers[1].full_name(), "Piotr Wiśniewski");
}

#[test]
fn teacher_full_name_handles_missing() {
    let teacher = Teacher {
        id: Some(1),
        first_name: None,
        last_name: Some("Nowak".into()),
    };
    assert_eq!(teacher.full_name(), "Nowak");

    let teacher2 = Teacher {
        id: Some(2),
        first_name: Some("Maria".into()),
        last_name: None,
    };
    assert_eq!(teacher2.full_name(), "Maria");
}

// ---------------------------------------------------------------------------
// Timetable / Lessons
// ---------------------------------------------------------------------------

#[test]
fn deserialize_lesson() {
    let json = serde_json::json!({
        "LessonNo": 3,
        "HourFrom": "10:15",
        "HourTo": "11:00",
        "Subject": { "Id": 1 },
        "Teacher": { "Id": 42 },
        "Classroom": { "Id": 5 },
        "IsCanceled": false,
        "IsSubstitutionClass": false
    });

    let lesson: Lesson = serde_json::from_value(json).unwrap();
    assert_eq!(lesson.lesson_no, Some(3));
    assert_eq!(lesson.hour_from.as_deref(), Some("10:15"));
    assert_eq!(lesson.hour_to.as_deref(), Some("11:00"));
    assert_eq!(lesson.subject.as_ref().unwrap().id, Some(1));
    assert_eq!(lesson.teacher.as_ref().unwrap().id, Some(42));
    assert_eq!(lesson.is_canceled, Some(false));
    assert_eq!(lesson.is_substitution_class, Some(false));
}

#[test]
fn deserialize_cancelled_lesson() {
    let json = serde_json::json!({
        "LessonNo": 1,
        "HourFrom": "08:00",
        "HourTo": "08:45",
        "Subject": { "Id": 2 },
        "Teacher": { "Id": 43 },
        "IsCanceled": true,
        "IsSubstitutionClass": false
    });

    let lesson: Lesson = serde_json::from_value(json).unwrap();
    assert_eq!(lesson.is_canceled, Some(true));
    assert_eq!(lesson.is_substitution_class, Some(false));
}

#[test]
fn deserialize_substitution_lesson() {
    let json = serde_json::json!({
        "LessonNo": 2,
        "HourFrom": "09:00",
        "HourTo": "09:45",
        "Subject": { "Id": 3 },
        "Teacher": { "Id": 44 },
        "IsCanceled": false,
        "IsSubstitutionClass": true,
        "OrgSubject": { "Id": 1 },
        "OrgTeacher": { "Id": 42 }
    });

    let lesson: Lesson = serde_json::from_value(json).unwrap();
    assert_eq!(lesson.is_substitution_class, Some(true));
    assert_eq!(lesson.org_subject.as_ref().unwrap().id, Some(1));
    assert_eq!(lesson.org_teacher.as_ref().unwrap().id, Some(42));
}

#[test]
fn parse_timetable_structure() {
    use librus_front::api::client::LibrusClient;

    let timetable = serde_json::json!({
        "2024-09-16": [
            [
                {
                    "LessonNo": 1,
                    "HourFrom": "08:00",
                    "HourTo": "08:45",
                    "Subject": { "Id": 1 },
                    "Teacher": { "Id": 42 },
                    "IsCanceled": false,
                    "IsSubstitutionClass": false
                }
            ],
            [
                {
                    "LessonNo": 2,
                    "HourFrom": "09:00",
                    "HourTo": "09:45",
                    "Subject": { "Id": 2 },
                    "Teacher": { "Id": 43 },
                    "IsCanceled": false,
                    "IsSubstitutionClass": false
                }
            ],
            []
        ],
        "2024-09-17": [
            [
                {
                    "LessonNo": 1,
                    "HourFrom": "08:00",
                    "HourTo": "08:45",
                    "Subject": { "Id": 3 },
                    "Teacher": { "Id": 44 },
                    "IsCanceled": true,
                    "IsSubstitutionClass": false
                }
            ]
        ]
    });

    let days = LibrusClient::parse_timetable(&timetable);

    assert_eq!(days.len(), 2);
    // sorted by date
    assert_eq!(days[0].0, "2024-09-16");
    assert_eq!(days[0].1.len(), 2); // 2 lessons (empty slots ignored)
    assert_eq!(days[0].1[0].lesson_no, Some(1));
    assert_eq!(days[0].1[1].lesson_no, Some(2));

    assert_eq!(days[1].0, "2024-09-17");
    assert_eq!(days[1].1.len(), 1);
    assert_eq!(days[1].1[0].is_canceled, Some(true));
}

// ---------------------------------------------------------------------------
// Attendances
// ---------------------------------------------------------------------------

#[test]
fn deserialize_attendances() {
    let json = serde_json::json!({
        "Attendances": [
            {
                "Id": "12345_6",
                "LessonNo": 3,
                "Date": "2024-09-16",
                "AddDate": "2024-09-16 10:30:00",
                "Semester": 1,
                "Type": { "Id": 1 },
                "Lesson": { "Id": 100 },
                "AddedBy": { "Id": 42 }
            }
        ]
    });

    let resp: AttendancesResponse = serde_json::from_value(json).unwrap();
    let attendances = resp.attendances.unwrap();

    assert_eq!(attendances.len(), 1);
    // Id is a string in Librus API
    assert_eq!(
        attendances[0].id.as_ref().unwrap().as_str(),
        Some("12345_6")
    );
    assert_eq!(attendances[0].lesson_no, Some(3));
    assert_eq!(attendances[0].date.as_deref(), Some("2024-09-16"));
    assert_eq!(attendances[0].semester, Some(1));
    assert_eq!(
        attendances[0].attendance_type.as_ref().unwrap().id,
        Some(1)
    );
}

// ---------------------------------------------------------------------------
// Attendance Types
// ---------------------------------------------------------------------------

#[test]
fn deserialize_attendance_types() {
    let json = serde_json::json!({
        "Types": [
            {
                "Id": 1,
                "Name": "Obecność",
                "Short": "ob",
                "IsPresenceKind": true,
                "ColorRGB": "00FF00"
            },
            {
                "Id": 2,
                "Name": "Nieobecność",
                "Short": "nb",
                "IsPresenceKind": false,
                "ColorRGB": "FF0000"
            },
            {
                "Id": 3,
                "Name": "Spóźnienie",
                "Short": "sp",
                "IsPresenceKind": false,
                "ColorRGB": "FFAA00"
            }
        ]
    });

    let resp: AttendanceTypesResponse = serde_json::from_value(json).unwrap();
    let types = resp.types.unwrap();

    assert_eq!(types.len(), 3);
    assert_eq!(types[0].name.as_deref(), Some("Obecność"));
    assert_eq!(types[0].is_presence_kind, Some(true));
    assert_eq!(types[1].name.as_deref(), Some("Nieobecność"));
    assert_eq!(types[1].is_presence_kind, Some(false));
    assert_eq!(types[2].short.as_deref(), Some("sp"));
}

// ---------------------------------------------------------------------------
// Events / Homework
// ---------------------------------------------------------------------------

#[test]
fn deserialize_events() {
    let json = serde_json::json!({
        "HomeWorks": [
            {
                "Id": 500,
                "Date": "2024-10-01",
                "Content": "Sprawdzian z rozdziału 3",
                "LessonNo": 2,
                "TimeFrom": "09:00",
                "AddDate": "2024-09-25 12:00:00",
                "Category": { "Id": 1 },
                "Subject": { "Id": 1 },
                "CreatedBy": { "Id": 42 },
                "Class": { "Id": 10 }
            },
            {
                "Id": 501,
                "Date": "2024-10-05",
                "Content": "Zadanie domowe str. 45 zad. 1-5",
                "AddDate": "2024-09-30 15:00:00",
                "Category": { "Id": 2 },
                "Subject": { "Id": 2 },
                "CreatedBy": { "Id": 43 }
            }
        ]
    });

    let resp: EventsResponse = serde_json::from_value(json).unwrap();
    let events = resp.homeworks.unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, Some(500));
    assert_eq!(events[0].date.as_deref(), Some("2024-10-01"));
    assert_eq!(
        events[0].content.as_deref(),
        Some("Sprawdzian z rozdziału 3")
    );
    assert_eq!(events[0].lesson_no, Some(2));
    assert_eq!(events[0].category.as_ref().unwrap().id, Some(1));

    assert_eq!(events[1].id, Some(501));
    assert!(events[1].lesson_no.is_none());
}

// ---------------------------------------------------------------------------
// Event Types
// ---------------------------------------------------------------------------

#[test]
fn deserialize_event_types() {
    let json = serde_json::json!({
        "Categories": [
            { "Id": 1, "Name": "Sprawdzian" },
            { "Id": 2, "Name": "Zadanie domowe" },
            { "Id": 3, "Name": "Kartkówka" }
        ]
    });

    let resp: EventTypesResponse = serde_json::from_value(json).unwrap();
    let types = resp.categories.unwrap();

    assert_eq!(types.len(), 3);
    assert_eq!(types[0].name.as_deref(), Some("Sprawdzian"));
    assert_eq!(types[2].name.as_deref(), Some("Kartkówka"));
}

// ---------------------------------------------------------------------------
// Announcements
// ---------------------------------------------------------------------------

#[test]
fn deserialize_announcements() {
    let json = serde_json::json!({
        "SchoolNotices": [
            {
                "Id": "abc-def-123",
                "Subject": "Dni wolne od zajęć",
                "Content": "Informujemy, że 1 i 2 listopada szkoła jest zamknięta.",
                "StartDate": "2024-10-28",
                "EndDate": "2024-11-03",
                "AddedBy": { "Id": 42 },
                "CreationDate": "2024-10-25 09:00:00",
                "WasRead": false
            },
            {
                "Id": "xyz-789",
                "Subject": "Zebranie rodziców",
                "Content": "Zebranie odbędzie się 5 listopada o 17:00",
                "StartDate": "2024-11-01",
                "EndDate": "2024-11-05",
                "AddedBy": { "Id": 43 },
                "CreationDate": "2024-10-30 14:00:00",
                "WasRead": true
            }
        ]
    });

    let resp: AnnouncementsResponse = serde_json::from_value(json).unwrap();
    let announcements = resp.school_notices.unwrap();

    assert_eq!(announcements.len(), 2);
    assert_eq!(announcements[0].id.as_deref(), Some("abc-def-123"));
    assert_eq!(
        announcements[0].subject.as_deref(),
        Some("Dni wolne od zajęć")
    );
    assert_eq!(announcements[0].was_read, Some(false));
    assert_eq!(
        announcements[0].start_date.as_deref(),
        Some("2024-10-28")
    );

    assert_eq!(announcements[1].was_read, Some(true));
}

// ---------------------------------------------------------------------------
// Lucky Number
// ---------------------------------------------------------------------------

#[test]
fn deserialize_lucky_number() {
    let json = serde_json::json!({
        "LuckyNumber": {
            "LuckyNumber": 13,
            "LuckyNumberDay": "2024-09-16"
        }
    });

    let resp: LuckyNumberResponse = serde_json::from_value(json).unwrap();
    let ln = resp.lucky_number.unwrap();

    assert_eq!(ln.lucky_number, Some(13));
    assert_eq!(ln.lucky_number_day.as_deref(), Some("2024-09-16"));
}

#[test]
fn deserialize_lucky_number_null() {
    // When lucky number is not active, the response may be null
    let json = serde_json::json!({
        "LuckyNumber": null
    });

    let resp: LuckyNumberResponse = serde_json::from_value(json).unwrap();
    assert!(resp.lucky_number.is_none());
}

// ---------------------------------------------------------------------------
// School
// ---------------------------------------------------------------------------

#[test]
fn deserialize_school() {
    let json = serde_json::json!({
        "School": {
            "Id": 100,
            "Name": "Liceum Ogólnokształcące nr 1",
            "Town": "Warszawa"
        }
    });

    let resp: SchoolResponse = serde_json::from_value(json).unwrap();
    let school = resp.school.unwrap();

    assert_eq!(school.id, Some(100));
    assert_eq!(
        school.name.as_deref(),
        Some("Liceum Ogólnokształcące nr 1")
    );
    assert_eq!(school.town.as_deref(), Some("Warszawa"));
}

// ---------------------------------------------------------------------------
// Class
// ---------------------------------------------------------------------------

#[test]
fn deserialize_class() {
    let json = serde_json::json!({
        "Class": {
            "Id": 10,
            "Number": 3,
            "Symbol": "a"
        }
    });

    let resp: ClassesResponse = serde_json::from_value(json).unwrap();
    let class = resp.class.unwrap();

    assert_eq!(class.id, Some(10));
    assert_eq!(class.number, Some(3));
    assert_eq!(class.symbol.as_deref(), Some("a"));
}

// ---------------------------------------------------------------------------
// Classrooms
// ---------------------------------------------------------------------------

#[test]
fn deserialize_classrooms() {
    let json = serde_json::json!({
        "Classrooms": [
            { "Id": 1, "Name": "Sala 101", "Symbol": "101" },
            { "Id": 2, "Name": "Sala gimnastyczna", "Symbol": "SG" }
        ]
    });

    let resp: ClassroomsResponse = serde_json::from_value(json).unwrap();
    let classrooms = resp.classrooms.unwrap();

    assert_eq!(classrooms.len(), 2);
    assert_eq!(classrooms[0].name.as_deref(), Some("Sala 101"));
    assert_eq!(classrooms[1].symbol.as_deref(), Some("SG"));
}

// ---------------------------------------------------------------------------
// Notices (behaviour notes)
// ---------------------------------------------------------------------------

#[test]
fn deserialize_notices() {
    let json = serde_json::json!({
        "Notes": [
            {
                "Id": 300,
                "Text": "Uczeń był bardzo aktywny na lekcji",
                "Category": { "Id": 1 },
                "AddedBy": { "Id": 42 },
                "Date": "2024-09-20",
                "Positive": true
            },
            {
                "Id": 301,
                "Text": "Brak zadania domowego",
                "Category": { "Id": 2 },
                "AddedBy": { "Id": 43 },
                "Date": "2024-09-22",
                "Positive": false
            }
        ]
    });

    let resp: NoticesResponse = serde_json::from_value(json).unwrap();
    let notices = resp.notes.unwrap();

    assert_eq!(notices.len(), 2);
    assert_eq!(notices[0].positive, Some(true));
    assert_eq!(
        notices[0].text.as_deref(),
        Some("Uczeń był bardzo aktywny na lekcji")
    );
    assert_eq!(notices[1].positive, Some(false));
}

// ---------------------------------------------------------------------------
// Lesson Ranges
// ---------------------------------------------------------------------------

#[test]
fn deserialize_lesson_ranges() {
    let json = serde_json::json!({
        "Lessons": [
            { "LessonNo": 1, "HourFrom": "08:00", "HourTo": "08:45" },
            { "LessonNo": 2, "HourFrom": "08:55", "HourTo": "09:40" },
            { "LessonNo": 3, "HourFrom": "09:50", "HourTo": "10:35" }
        ]
    });

    let resp: LessonRangesResponse = serde_json::from_value(json).unwrap();
    let ranges = resp.lessons.unwrap();

    assert_eq!(ranges.len(), 3);
    assert_eq!(ranges[0].lesson_no, Some(1));
    assert_eq!(ranges[0].hour_from.as_deref(), Some("08:00"));
    assert_eq!(ranges[0].hour_to.as_deref(), Some("08:45"));
    assert_eq!(ranges[2].lesson_no, Some(3));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_grades_response() {
    let json = serde_json::json!({
        "Grades": []
    });

    let resp: GradesResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.grades.unwrap().len(), 0);
}

#[test]
fn missing_optional_fields_in_grade() {
    let json = serde_json::json!({
        "Grades": [
            {
                "Id": 999,
                "Grade": "nb",
                "Semester": 1
            }
        ]
    });

    let resp: GradesResponse = serde_json::from_value(json).unwrap();
    let grades = resp.grades.unwrap();

    assert_eq!(grades.len(), 1);
    assert_eq!(grades[0].id, Some(999));
    assert_eq!(grades[0].grade.as_deref(), Some("nb"));
    assert!(grades[0].category.is_none());
    assert!(grades[0].subject.is_none());
    assert!(grades[0].added_by.is_none());
    assert!(grades[0].comments.is_none());
    // All boolean fields default to None
    assert_eq!(grades[0].grade_type(), GradeType::Normal);
}

#[test]
fn idref_default() {
    let id: IdRef = IdRef::default();
    assert!(id.id.is_none());
}
