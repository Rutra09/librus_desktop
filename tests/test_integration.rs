//! Integration tests that hit the real Librus API.
//!
//! Run with credentials:
//!   $env:LIBRUS_EMAIL="your@email.com"; $env:LIBRUS_PASSWORD="your_pass"; cargo test --test test_integration -- --ignored --nocapture
//!
//! These tests are #[ignore]d by default so `cargo test` won't fail without credentials.

use chrono::Datelike;
use librus_front::api::client::LibrusClient;

fn get_credentials() -> (String, String) {
    let email = std::env::var("LIBRUS_EMAIL")
        .expect("Set LIBRUS_EMAIL env var (your portal email)");
    let password = std::env::var("LIBRUS_PASSWORD")
        .expect("Set LIBRUS_PASSWORD env var");
    (email, password)
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_login_portal() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await;

    match &client {
        Ok(_) => println!("✅ Login successful!"),
        Err(e) => panic!("❌ Login failed: {e}"),
    }

    let client = client.unwrap();
    let auth = client.get_auth_state().await;
    println!("  Synergia login: {}", auth.synergia_login);
    println!("  Token: {}...", &auth.access_token[..20.min(auth.access_token.len())]);
    println!("  Expires at: {}", auth.expires_at);
    assert!(auth.is_valid());
}

// ---------------------------------------------------------------------------
// Me
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_me() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let me = client.fetch_me().await.unwrap();
    let account = me.me.account.unwrap();
    println!("✅ Me:");
    println!("  Name: {} {}",
        account.first_name.as_deref().unwrap_or("?"),
        account.last_name.as_deref().unwrap_or("?"));
    println!("  Login: {}", account.login.as_deref().unwrap_or("?"));
    println!("  Premium: {:?}", account.is_premium);
    println!("  GroupId: {:?}", account.group_id);
}

// ---------------------------------------------------------------------------
// Grades
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_grades() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let grades = client.fetch_grades().await.unwrap();
    println!("✅ Grades: {} total", grades.len());
    for g in grades.iter().take(5) {
        println!("  [{}] ocena={} sem={} typ={:?}",
            g.id.unwrap_or(0),
            g.grade.as_deref().unwrap_or("?"),
            g.semester.unwrap_or(0),
            g.grade_type());
    }
    if grades.len() > 5 {
        println!("  ... and {} more", grades.len() - 5);
    }
}

#[tokio::test]
#[ignore]
async fn real_fetch_grade_categories() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let cats = client.fetch_grade_categories().await.unwrap();
    println!("✅ Grade categories: {} total", cats.len());
    for c in cats.iter().take(5) {
        println!("  [{}] {} (waga: {:?})",
            c.id.unwrap_or(0),
            c.name.as_deref().unwrap_or("?"),
            c.weight);
    }
}

// ---------------------------------------------------------------------------
// Subjects
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_subjects() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let subjects = client.fetch_subjects().await.unwrap();
    println!("✅ Subjects: {} total", subjects.len());
    for s in &subjects {
        println!("  [{}] {} ({})",
            s.id.unwrap_or(0),
            s.name.as_deref().unwrap_or("?"),
            s.short.as_deref().unwrap_or("?"));
    }
}

// ---------------------------------------------------------------------------
// Teachers
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_teachers() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let teachers = client.fetch_teachers().await.unwrap();
    println!("✅ Teachers: {} total", teachers.len());
    for t in teachers.iter().take(10) {
        println!("  [{}] {}", t.id.unwrap_or(0), t.full_name());
    }
}

// ---------------------------------------------------------------------------
// Timetable
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_timetable() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    // Get current Monday
    let today = chrono::Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday();
    let monday = today - chrono::Duration::days(weekday as i64);
    let week_start = monday.format("%Y-%m-%d").to_string();

    println!("  Fetching timetable for week starting: {week_start}");
    let timetable = client.fetch_timetable(&week_start).await.unwrap();
    let days = LibrusClient::parse_timetable(&timetable);

    println!("✅ Timetable: {} days", days.len());
    for (date, lessons) in &days {
        println!("  📅 {date}: {} lessons", lessons.len());
        for l in lessons {
            let cancelled = if l.is_canceled.unwrap_or(false) { " [ODWOŁANE]" } else { "" };
            let subst = if l.is_substitution_class.unwrap_or(false) { " [ZASTĘPSTWO]" } else { "" };
            println!("    L{}: {}-{} subject={} teacher={}{}{}",
                l.lesson_no.unwrap_or(0),
                l.hour_from.as_deref().unwrap_or("?"),
                l.hour_to.as_deref().unwrap_or("?"),
                l.subject.as_ref().and_then(|s| s.id).unwrap_or(0),
                l.teacher.as_ref().and_then(|t| t.id).unwrap_or(0),
                cancelled, subst);
        }
    }
}

// ---------------------------------------------------------------------------
// Attendance
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_attendances() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let attendances = client.fetch_attendances().await.unwrap();
    println!("✅ Attendances: {} total", attendances.len());
    for a in attendances.iter().take(5) {
        println!("  date={} lesson={} type={}",
            a.date.as_deref().unwrap_or("?"),
            a.lesson_no.unwrap_or(0),
            a.attendance_type.as_ref().and_then(|t| t.id).unwrap_or(0));
    }
}

#[tokio::test]
#[ignore]
async fn real_fetch_attendance_types() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let types = client.fetch_attendance_types().await.unwrap();
    println!("✅ Attendance types: {} total", types.len());
    for t in &types {
        println!("  [{}] {} ({}) presence={}",
            t.id.unwrap_or(0),
            t.name.as_deref().unwrap_or("?"),
            t.short.as_deref().unwrap_or("?"),
            t.is_presence_kind.unwrap_or(false));
    }
}

// ---------------------------------------------------------------------------
// Events / Homework
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_events() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let events = client.fetch_events().await.unwrap();
    println!("✅ Events/Homework: {} total", events.len());
    for e in events.iter().take(5) {
        println!("  [{}] {} — {}",
            e.id.unwrap_or(0),
            e.date.as_deref().unwrap_or("?"),
            e.content.as_deref().unwrap_or("?"));
    }
}

// ---------------------------------------------------------------------------
// Announcements
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_announcements() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    let announcements = client.fetch_announcements().await.unwrap();
    println!("✅ Announcements: {} total", announcements.len());
    for a in announcements.iter().take(5) {
        println!("  [{}] {} (read={})",
            a.id.as_deref().unwrap_or("?"),
            a.subject.as_deref().unwrap_or("?"),
            a.was_read.unwrap_or(false));
    }
}

// ---------------------------------------------------------------------------
// Lucky Number
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_lucky_number() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    match client.fetch_lucky_number().await {
        Ok(Some(ln)) => {
            println!("✅ Lucky number: {} ({})",
                ln.lucky_number.unwrap_or(0),
                ln.lucky_number_day.as_deref().unwrap_or("?"));
        }
        Ok(None) => println!("⚠️ Lucky number not active"),
        Err(e) => println!("⚠️ Lucky number error (may not be active): {e}"),
    }
}

// ---------------------------------------------------------------------------
// School + Class
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_fetch_school_and_class() {
    let (email, password) = get_credentials();
    let client = LibrusClient::login(&email, &password).await.unwrap();

    if let Ok(Some(school)) = client.fetch_school().await {
        println!("✅ School: {} ({})",
            school.name.as_deref().unwrap_or("?"),
            school.town.as_deref().unwrap_or("?"));
    }

    if let Ok(Some(class)) = client.fetch_class().await {
        println!("✅ Class: {}{}",
            class.number.unwrap_or(0),
            class.symbol.as_deref().unwrap_or(""));
    }
}

// ---------------------------------------------------------------------------
// Full sync test — fetches everything
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn real_full_sync() {
    let (email, password) = get_credentials();
    println!("🔑 Logging in as {email}...");
    let client = LibrusClient::login(&email, &password).await.unwrap();
    println!("✅ Logged in!\n");

    // Me
    let me = client.fetch_me().await.unwrap();
    let acc = me.me.account.unwrap();
    println!("👤 {} {} ({})",
        acc.first_name.as_deref().unwrap_or("?"),
        acc.last_name.as_deref().unwrap_or("?"),
        acc.login.as_deref().unwrap_or("?"));

    // School
    if let Ok(Some(school)) = client.fetch_school().await {
        println!("🏫 {} ({})",
            school.name.as_deref().unwrap_or("?"),
            school.town.as_deref().unwrap_or("?"));
    }

    // Subjects
    let subjects = client.fetch_subjects().await.unwrap_or_default();
    println!("📚 {} subjects", subjects.len());

    // Teachers
    let teachers = client.fetch_teachers().await.unwrap_or_default();
    println!("👨‍🏫 {} teachers", teachers.len());

    // Grades
    let grades = client.fetch_grades().await.unwrap_or_default();
    println!("📝 {} grades", grades.len());

    // Timetable
    let today = chrono::Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday();
    let monday = today - chrono::Duration::days(weekday as i64);
    let week_start = monday.format("%Y-%m-%d").to_string();
    let timetable = client.fetch_timetable(&week_start).await.unwrap();
    let days = LibrusClient::parse_timetable(&timetable);
    let total_lessons: usize = days.iter().map(|(_, l)| l.len()).sum();
    println!("📅 {} days, {} lessons this week", days.len(), total_lessons);

    // Attendance
    let attendances = client.fetch_attendances().await.unwrap_or_default();
    println!("✋ {} attendance records", attendances.len());

    // Events
    let events = client.fetch_events().await.unwrap_or_default();
    println!("📋 {} events/homework", events.len());

    // Announcements
    let announcements = client.fetch_announcements().await.unwrap_or_default();
    println!("📢 {} announcements", announcements.len());

    // Lucky number
    match client.fetch_lucky_number().await {
        Ok(Some(ln)) => println!("🍀 Lucky number: {}", ln.lucky_number.unwrap_or(0)),
        _ => println!("🍀 Lucky number: not active"),
    }

    println!("\n✅ Full sync complete!");
}
