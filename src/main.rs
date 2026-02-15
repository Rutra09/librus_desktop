#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use slint::ComponentHandle;
use std::sync::Arc;
// Trigger rebuild
use tokio::sync::Mutex;
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::{HashMap, HashSet};

use librus_front::app_state::AppState;
use librus_front::api::client::LibrusClient;
use librus_front::api::auth::AuthState;
use librus_front::api::models::Lesson; // Import Lesson
use librus_front::session;

slint::include_modules!();

fn session_to_auth(s: session::Session) -> AuthState {
    AuthState {
        access_token: s.access_token,
        expires_at: s.expires_at,
        refresh_token: s.refresh_token,
        portal_access_token: s.portal_access_token,
        portal_refresh_token: s.portal_refresh_token,
        portal_expires_at: s.portal_expires_at,
        user_id: s.user_id,
        synergia_login: s.synergia_login,
        portal_email: s.portal_email,
        portal_password: s.portal_password,
        synergia_username: s.synergia_username,
        synergia_password: s.synergia_password,
        messages_session_id: s.messages_session_id,
        messages_session_expiry: s.messages_session_expiry,
    }
}

fn auth_to_session(a: AuthState) -> session::Session {
    session::Session {
        access_token: a.access_token,
        expires_at: a.expires_at,
        refresh_token: a.refresh_token,
        portal_access_token: a.portal_access_token,
        portal_refresh_token: a.portal_refresh_token,
        portal_expires_at: a.portal_expires_at,
        user_id: a.user_id,
        synergia_login: a.synergia_login,
        portal_email: a.portal_email,
        portal_password: a.portal_password,
        synergia_username: a.synergia_username,
        synergia_password: a.synergia_password,
        messages_session_id: a.messages_session_id,
        messages_session_expiry: a.messages_session_expiry,
    }
}

async fn fetch_metadata(client: LibrusClient, state: AppState) {
    // 1. Subjects
    if let Ok(subjects) = client.fetch_subjects().await {
        let mut map = state.subjects.lock().await;
        for s in subjects {
            if let Some(id) = s.id {
                map.insert(id, s.name.unwrap_or("?".into()));
            }
        }
    }
    // 2. Teachers
    if let Ok(teachers) = client.fetch_teachers().await {
        let mut map = state.teachers.lock().await;
        for t in teachers {
            if let Some(id) = t.id {
                let first = t.first_name.unwrap_or_default();
                let last = t.last_name.unwrap_or_default();
                let name = format!("{} {}", first, last);
                map.insert(id, name);
            }
        }
    }
    // 3. Grade Categories
    if let Ok(cats) = client.fetch_grade_categories().await {
        let mut map = state.grade_categories.lock().await;
        for c in cats {
            if let Some(id) = c.id {
                map.insert(id, c);
            }
        }
    }
    // 4. Attendance Types
    if let Ok(types) = client.fetch_attendance_types().await {
        let mut map = state.attendance_types.lock().await;
        for t in types {
            if let Some(id) = t.id {
                map.insert(id, t);
            }
        }
    }
    // 5. Classrooms
    if let Ok(rooms) = client.fetch_classrooms().await {
        let mut map = state.classrooms.lock().await;
        for r in rooms {
            if let Some(id) = r.id {
                map.insert(id, r);
            }
        }
    }
}

async fn fetch_dashboard_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>) {
    let me_res = client.fetch_me().await;
    let lucky_res = client.fetch_lucky_number().await;

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(window) = window_weak.upgrade() {
            if let Ok(me) = me_res {
                let first = me.me.user.as_ref().and_then(|u| u.first_name.as_deref()).unwrap_or("");
                let last = me.me.user.as_ref().and_then(|u| u.last_name.as_deref()).unwrap_or("");
                let name = format!("{} {}", first, last);
                window.set_user_name(name.into());
            }
            if let Ok(Some(lucky)) = lucky_res {
                 if let Some(num) = lucky.lucky_number {
                     window.set_lucky_number(format!("{}", num).into());
                 } else {
                     window.set_lucky_number("-".into());
                 }
                 window.set_lucky_number_day(lucky.lucky_number_day.unwrap_or_default().into());
            }
        }
    });
}

fn get_current_monday() -> NaiveDate {
    let today = chrono::Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday();
    today - Duration::days(weekday as i64)
}

fn hex_to_color(hex: &str) -> slint::Color {
    if hex.len() == 6 {
         let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
         let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
         let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
         return slint::Color::from_rgb_u8(r, g, b);
    }
    slint::Color::from_rgb_u8(100, 100, 100)
}

fn int_to_color(val: i32) -> slint::Color {
    let r = ((val >> 16) & 0xFF) as u8;
    let g = ((val >> 8) & 0xFF) as u8;
    let b = (val & 0xFF) as u8;
    slint::Color::from_rgb_u8(r, g, b)
}

// Helper to parse grade value
fn parse_grade_value(val: &str) -> Option<f32> {
    let lower = val.to_lowercase();
    if lower == "np" || lower == "nb" || lower == "zw" || lower == "?" || lower == "-" || lower == "" {
        return None;
    }
    
    // Check modifiers
    let mut modifier = 0.0;
    let mut base_str = lower.clone();
    
    if lower.contains('+') {
        modifier = 0.5;
        base_str = base_str.replace('+', "");
    } else if lower.contains('-') {
        modifier = -0.25; // Standard Polish school system often uses .75 for minus? e.g. 4- = 3.75. Or 3.5?
        // Let's assume standard: 4- is usually treated as 3.75 or 3.5. 
        // Librus often treats "+" as .5 and "-" as .75 of previous? 
        // Let's go with .75 for X- (which means X - 0.25).
        base_str = base_str.replace('-', "");
    }
    
    if let Ok(base) = base_str.parse::<f32>() {
        Some(base + modifier)
    } else {
        None
    }
}

async fn fetch_grades_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>, state: AppState) {
    if let Ok(grades_res) = client.fetch_grades().await {
        {
            let mut g_lock = state.grades.lock().await;
            *g_lock = grades_res;
        }
        update_grades_ui(window_weak, state).await;
    }
}

async fn update_grades_ui(window_weak: slint::Weak<MainWindow>, state: AppState) {
    let categories_map = state.grade_categories.lock().await;
    let subjects_map = state.subjects.lock().await;
    let teachers_map = state.teachers.lock().await;
    let grades_list = state.grades.lock().await;

    let mut grades_s1: HashMap<i64, Vec<UIGrade>> = HashMap::new();
    let mut grades_s2: HashMap<i64, Vec<UIGrade>> = HashMap::new();
    let mut sums_s1: HashMap<i64, (f32, f32)> = HashMap::new();
    let mut sums_s2: HashMap<i64, (f32, f32)> = HashMap::new();

    for g in grades_list.iter() {
            let subj_id = g.subject.as_ref().and_then(|s| s.id).unwrap_or(0);
            let cat_id = g.category.as_ref().and_then(|c| c.id).unwrap_or(0);
            
            let cat_opt = categories_map.get(&cat_id);
            let cat_name = cat_opt.and_then(|c| c.name.clone()).unwrap_or("?".into());
            let weight = cat_opt.and_then(|c| c.weight).unwrap_or(0.0);
            
            let teacher_id = g.added_by.as_ref().and_then(|u| u.id).unwrap_or(0);
            let teacher_name = teachers_map.get(&teacher_id).cloned().unwrap_or("?".into());

            let val_str = g.grade.clone().unwrap_or("?".into());
            let semester = g.semester.unwrap_or(0); // 1 or 2 usually
            
            // Calculate value for average
            if let Some(val) = parse_grade_value(&val_str) {
                if weight > 0.0 {
                    let entry = if semester == 1 { 
                    sums_s1.entry(subj_id).or_insert((0.0, 0.0)) 
                    } else { 
                    sums_s2.entry(subj_id).or_insert((0.0, 0.0)) 
                    };
                    entry.0 += val * weight;
                    entry.1 += weight;
                }
            }

            // Color logic
            let is_simulated = cat_id < 0; // Negative ID used for simulation
            let color = if is_simulated {
                 slint::Color::from_rgb_u8(100, 100, 255) // distinct blue/purple for simulated
            } else if let Some(v) = parse_grade_value(&val_str) {
                if v >= 5.0 { slint::Color::from_rgb_u8(34, 197, 94) }
                else if v >= 4.0 { slint::Color::from_rgb_u8(59, 130, 246) }
                else if v >= 3.0 { slint::Color::from_rgb_u8(234, 179, 8) }
                else if v >= 2.0 { slint::Color::from_rgb_u8(249, 115, 22) }
                else { slint::Color::from_rgb_u8(239, 68, 68) }
            } else {
                slint::Color::from_rgb_u8(107, 114, 128)
            };
            
            let combined_desc = format!("{} ({})", cat_name, teacher_name);
            
            // Format weight (check if integer)
            let weight_str = if (weight % 1.0).abs() < 0.01 {
                format!("{:.0}", weight)
            } else {
                format!("{:.1}", weight)
            };

            let ui_grade = UIGrade {
                value: val_str.into(),
                color: color.into(),
                desc: combined_desc.into(), 
                date: g.add_date.clone().unwrap_or("-".into()).into(),
                weight: weight_str.into(),
            };
            
            // If simulated, assume semester 2 (current)
            let sem_final = if semester == 0 { 2 } else { semester };

            if sem_final == 1 {
                grades_s1.entry(subj_id).or_insert(Vec::new()).push(ui_grade);
            } else {
                grades_s2.entry(subj_id).or_insert(Vec::new()).push(ui_grade);
            }
    }

    let mut all_subjects: HashSet<i64> = HashSet::new();
    all_subjects.extend(grades_s1.keys());
    all_subjects.extend(grades_s2.keys());

    let mut ui_list_raw = Vec::new();
    for subj_id in all_subjects {
            let subj_name = subjects_map.get(&subj_id).cloned().unwrap_or(format!("Subject {}", subj_id));
            
            // Helper for average
            let get_avg = |sums: &HashMap<i64, (f32, f32)>| -> String {
                if let Some((w_sum, w_total)) = sums.get(&subj_id) {
                    if *w_total > 0.0 { format!("{:.2}", w_sum / w_total) } else { "-".to_string() }
                } else { "-".to_string() }
            };

            let avg_s1 = get_avg(&sums_s1);
            let avg_s2 = get_avg(&sums_s2);
            
            let list_s1 = grades_s1.remove(&subj_id).unwrap_or_default();
            let list_s2 = grades_s2.remove(&subj_id).unwrap_or_default();

            // Default to S2
            let avg_current = avg_s2.clone();
            let list_current = list_s2.clone();

            ui_list_raw.push((subj_name, avg_current, list_current, avg_s1, list_s1, avg_s2, list_s2));
    }
    
    // Sort by subject name
    ui_list_raw.sort_by(|a, b| a.0.cmp(&b.0));

    // Update UI
    let _ = slint::invoke_from_event_loop(move || {
            let mut final_list = Vec::new();
            for (name, avg, grades, avg1, grades1, avg2, grades2) in ui_list_raw {
                final_list.push(UISubjectGrades {
                    subject_name: name.into(),
                    average: avg.into(),
                    grades: std::rc::Rc::new(slint::VecModel::from(grades)).into(),
                    average_s1: avg1.into(),
                    grades_s1: std::rc::Rc::new(slint::VecModel::from(grades1)).into(),
                    average_s2: avg2.into(),
                    grades_s2: std::rc::Rc::new(slint::VecModel::from(grades2)).into(),
                });
            }
            let model = std::rc::Rc::new(slint::VecModel::from(final_list));
            
            if let Some(window) = window_weak.upgrade() {
                window.set_grades_list(model.into());
            }
    });
}
 

async fn fetch_timetable_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>, week_start_str: String, state: AppState) {
    let timetable_json_res = client.fetch_timetable(&week_start_str).await;
    let events_res = client.fetch_events().await;

    if let Ok(json) = timetable_json_res {
        let parsed_days = LibrusClient::parse_timetable(&json);
        
        // Process events
        let mut events_map: HashMap<(String, i32), Vec<String>> = HashMap::new();
        if let Ok(events) = events_res {
            let event_cats = state.event_categories.lock().await; // Lock once
            for e in events {
                if let Some(date) = &e.date {
                    // Filter by range? For now just map all, map lookup handles it.
                    let l_no = e.lesson_no.unwrap_or(0);
                    let cat_id = e.category.as_ref().and_then(|c| c.id).unwrap_or(0);
                    let cat_name = event_cats.get(&cat_id).cloned().unwrap_or("?".to_string());
                    let content = e.content.clone().unwrap_or("".to_string());
                    
                    if !content.is_empty() {
                         let entry = format!("[{}] {}", cat_name, content);
                         events_map.entry((date.clone(), l_no)).or_insert(Vec::new()).push(entry);
                    }
                }
            }
        }
        
        let mut raw_days = Vec::new();
        
        // Cache locks
        let subjects_map = state.subjects.lock().await;
        let teachers_map = state.teachers.lock().await;
        let classrooms_map = state.classrooms.lock().await;

        for (date_str, lessons) in parsed_days {
            let mut ui_lessons = Vec::new();
            for l in lessons {
                let subject_name = l.subject.as_ref().map(|s| {
                    let id = s.id.unwrap_or(0);
                    subjects_map.get(&id).cloned().unwrap_or(format!("Subject {}", id))
                }).unwrap_or("?".into());

                let teacher_name = l.teacher.as_ref().map(|t| {
                    let id = t.id.unwrap_or(0);
                    teachers_map.get(&id).cloned().unwrap_or(format!("Teacher {}", id))
                }).unwrap_or("?".into());
                
                let room_name = l.classroom.as_ref().and_then(|c| c.id).map(|id| {
                    classrooms_map.get(&id).and_then(|r| r.name.clone().or(r.symbol.clone()))
                        .unwrap_or(format!("{}", id))
                }).unwrap_or("-".into());

                // Lookup event
                let lesson_no = l.lesson_no.unwrap_or(0);
                let event_content = if let Some(evs) = events_map.get(&(date_str.clone(), lesson_no)) {
                    evs.join("\n")
                } else {
                    "".to_string()
                };
                let event_category = "".to_string(); // Not used directly, content has it

                // Substitution Details
                let mut substitution_desc = String::new();
                if l.is_substitution_class.unwrap_or(false) {
                    let mut parts = Vec::new();
                    if let Some(oid) = l.org_subject.as_ref().and_then(|x| x.id) {
                         if let Some(name) = subjects_map.get(&oid) {
                             parts.push(name.clone());
                         }
                    }
                    if let Some(tid) = l.org_teacher.as_ref().and_then(|x| x.id) {
                        if let Some(t) = teachers_map.get(&tid) {
                            parts.push(format!("({})", t));
                        }
                    }
                    if !parts.is_empty() {
                        substitution_desc = parts.join(" ");
                    }
                }

                // Push Lesson
                ui_lessons.push(UILesson {
                    id: l.id.unwrap_or(0) as i32,
                    no: l.lesson_no.unwrap_or(0),
                    subject: subject_name.into(),
                    teacher: teacher_name.into(),
                    room: room_name.into(),
                    color: slint::Color::from_argb_encoded(0xff_ffffff).into(), // Default Brush
                    is_substitution: l.is_substitution_class.unwrap_or(false),
                    is_canceled: l.is_canceled.unwrap_or(false),
                    hour_from: l.hour_from.clone().unwrap_or_default().into(),
                    hour_to: l.hour_to.clone().unwrap_or_default().into(),
                    event_content: event_content.into(),
                    event_category: event_category.into(),
                    substitution_desc: substitution_desc.into(),
                });
            }
            
            let day_name = if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                 d.weekday().to_string()
            } else {
                "Day".to_string()
            };
            
            raw_days.push((date_str, day_name, ui_lessons));
        }
        
        // Update UI
        let _ = slint::invoke_from_event_loop(move || {
            // Construct ModelRc here (on UI thread)
            let mut ui_days = Vec::new();
            for (date, name, lessons) in raw_days {
                ui_days.push(UIDay {
                    date: date.into(),
                    name: name.into(),
                    lessons: std::rc::Rc::new(slint::VecModel::from(lessons)).into(),
                });
            }
            let days_model = std::rc::Rc::new(slint::VecModel::from(ui_days));

            if let Some(window) = window_weak.upgrade() {
                window.set_timetable_schedule(days_model.into());
                window.set_timetable_week_range(week_start_str.into());
            }
        });
    }
}

async fn fetch_announcements_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>, state: AppState) {
    if let Ok(notices) = client.fetch_announcements().await {
        let teachers_map = state.teachers.lock().await;

        let mut ui_notices = Vec::new();
        for n in notices {
            let author_id = n.added_by.as_ref().and_then(|id| id.id).unwrap_or(0);
            let author = teachers_map.get(&author_id).cloned().unwrap_or("?".to_string());
            
            ui_notices.push(UIAnnouncement {
                subject: n.subject.unwrap_or("No Subject".into()).into(),
                content: n.content.unwrap_or("".into()).into(),
                author: author.into(),
                date: n.creation_date.unwrap_or(n.start_date.unwrap_or("-".into())).into(),
            });
        }
        
        // Sort by date desc
        ui_notices.sort_by(|a, b| b.date.cmp(&a.date));

        let _ = slint::invoke_from_event_loop(move || {
             let model = std::rc::Rc::new(slint::VecModel::from(ui_notices));
             if let Some(window) = window_weak.upgrade() {
                 window.set_announcements_list(model.into());
             }
        });
    }
}

async fn fetch_attendances_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>, state: AppState) {
    if let Ok(atts) = client.fetch_attendances().await {
        let types_map = state.attendance_types.lock().await;
        // subjects_map is used if lesson fetching fails or for fallback
        let subjects_map = state.subjects.lock().await; 
        let classrooms_map = state.classrooms.lock().await;

        // 1. Collect Lesson IDs
        let mut lesson_ids: Vec<i64> = atts.iter()
            .filter_map(|a| a.lesson.as_ref().and_then(|l| l.id))
            .collect();
        lesson_ids.sort();
        lesson_ids.dedup();

        // 2. Fetch Lessons in Chunks
        let mut lesson_details: HashMap<i64, Lesson> = HashMap::new();
        for chunk in lesson_ids.chunks(20) { // Fetch 20 at a time
            if let Ok(lessons) = client.fetch_lessons(chunk.to_vec()).await {
                for l in lessons {
                    if let Some(id) = l.id {
                        lesson_details.insert(id, l);
                    }
                }
            }
        }

        let mut ui_atts = Vec::new();
        let mut present_count = 0;
        let mut total_count = 0;
        
        for a in atts {
            let date = a.date.clone().unwrap_or("-".into());
            let lesson_no = a.lesson_no.unwrap_or(0);
            
            // Resolve Type
            let type_id = a.attendance_type.as_ref().and_then(|t| t.id).unwrap_or(0);
            let type_entry = types_map.get(&type_id);
            let type_name = type_entry.and_then(|t| t.name.clone()).unwrap_or("?".into());
            let type_short = type_entry.and_then(|t| t.short.clone()).unwrap_or("?".into());
            let color_hex = type_entry.and_then(|t| t.color_rgb.clone()).unwrap_or("888888".into());
            let color = hex_to_color(&color_hex);
            
            // Resolve Stats
            if let Some(t) = type_entry {
                if t.is_presence_kind.unwrap_or(false) {
                    present_count += 1;
                }
                total_count += 1;
            }
            
            // Resolve Subject & Topic & Classroom
            let mut subject_name = "-".to_string();
            let mut topic = "".to_string(); // Not available in Attendance or Lesson?
            // Wait, Lesson has Topic? No, Lesson has Subject, Teacher, Class.
            
            if let Some(l_id) = a.lesson.as_ref().and_then(|l| l.id) {
                if let Some(details) = lesson_details.get(&l_id) {
                    // Get Subject
                   if let Some(s_id) = details.subject.as_ref().and_then(|s| s.id) {
                        subject_name = subjects_map.get(&s_id).cloned().unwrap_or(format!("Subject {}", s_id));
                   }
                   // Append Classroom
                   if let Some(c_id) = details.classroom.as_ref().and_then(|c| c.id) {
                       if let Some(c_obj) = classrooms_map.get(&c_id) {
                           let c_name = c_obj.name.as_ref().or(c_obj.symbol.as_ref()).cloned().unwrap_or("?".into());
                           subject_name = format!("{} [{}]", subject_name, c_name);
                       }
                   }
                }
            }
            
             ui_atts.push(UIAttendance {
                 id: format!("{}", a.id.as_ref().map(|v| v.to_string()).unwrap_or("0".into())).into(),
                 date: date.into(),
                 type_name: type_name.into(),
                 subject: subject_name.into(),
                 lesson_no: lesson_no,
                 color: color.into(),
                 symbol: type_short.into(),
                 topic: topic.into(),
             });
        }
        
        // Sort by date desc
        ui_atts.sort_by(|a, b| b.date.cmp(&a.date));

        let freq = if total_count > 0 {
            format!("{:.1}%", (present_count as f32 / total_count as f32) * 100.0)
        } else {
            "0%".into()
        };
        
        // Calculate Stats
        let mut subject_stats: HashMap<String, (i32, i32, i32, i32, i32)> = HashMap::new(); // Name -> (Pres, Unexc, Exc, Late, Total)
        
        for a in &ui_atts {
             let subj = a.subject.to_string();
             let type_short = a.symbol.to_string().to_lowercase();
             let type_name = a.type_name.to_string().to_lowercase();
             
             let entry = subject_stats.entry(subj).or_insert((0, 0, 0, 0, 0));
             entry.4 += 1; // Total
             
             // Logic: Check specific traits first
             
             // 1. Excused (Usprawiedliwione / Zwolnienie)
             if type_short == "u" || type_short == "zw" || type_name.contains("uspraw") || type_name.contains("zwoln") {
                 entry.2 += 1; // Excused
             } 
             // 2. Unexcused Absence (Nieobecność - must check this BEFORE "obecn")
             else if type_short == "nb" || type_name.contains("nieobecn") {
                 entry.1 += 1; // Unexcused
             } 
             // 3. Late (Spóźnienie)
             else if type_short == "sp" || type_name.contains("spóźn") {
                 entry.3 += 1; // Late
                 entry.0 += 1; // Late counts as present in most stats
             }
             // 4. Present (Obecność)
             else if type_short == "ob" || type_name.contains("obecn") || type_name.contains("wycieczka") || type_name.contains("delegacja") {
                 entry.0 += 1; // Pres
             }
        }
        
        let mut ui_stats = Vec::new();
        for (name, (pres, unexc, exc, late, total)) in subject_stats {
             let freq_val = if total > 0 { (pres as f32 / total as f32) * 100.0 } else { 0.0 };
             let freq_str = format!("{:.0}%", freq_val);
             
             ui_stats.push(UIAttendanceSubject {
                 id: 0,
                 name: name.into(),
                 presence_count: pres,
                 absence_unexcused_count: unexc,
                 absence_excused_count: exc,
                 late_count: late,
                 total_count: total,
                 frequency: freq_str.into(),
             });
        }
        ui_stats.sort_by(|a, b| a.name.cmp(&b.name));

        let _ = slint::invoke_from_event_loop(move || {
             let model = std::rc::Rc::new(slint::VecModel::from(ui_atts));
             let stats_model = std::rc::Rc::new(slint::VecModel::from(ui_stats));
             if let Some(window) = window_weak.upgrade() {
                 window.set_attendances_list(model.into());
                 window.set_attendance_stats(stats_model.into());
                 window.set_attendance_frequency(freq.into());
             }
        });
    }
}

async fn fetch_messages_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>) {
    use librus_front::api::models::MessageType;
    
    println!("Fetching messages...");
    match client.fetch_messages().await {
        Ok(messages) => {
            println!("Received {} messages", messages.len());
            let mut ui_messages = Vec::new();
            
            for msg in messages {
                // Parse date to simpler format (just date part)
                let date_str = msg.send_date
                    .split('T')
                    .next()
                    .unwrap_or(&msg.send_date)
                    .to_string();
                
                println!("Message: {} from {}", msg.subject, msg.sender_name);
                
                ui_messages.push(UIMessage {
                    id: msg.id as i32,
                    subject: msg.subject.into(),
                    sender: msg.sender_name.into(),
                    date: date_str.into(),
                    read: msg.read_date.is_some(),
                    has_attachments: msg.has_attachments,
                });
            }

            // reverse order
            ui_messages.reverse();
            
            let _ = slint::invoke_from_event_loop(move || {
                let model = std::rc::Rc::new(slint::VecModel::from(ui_messages));
                if let Some(window) = window_weak.upgrade() {
                    window.set_messages_list(model.into());
                }
            });
        }
        Err(e) => {
            eprintln!("Failed to fetch messages: {:?}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Force Skia backend if not already set
    if std::env::var("SLINT_BACKEND").is_err() {
        std::env::set_var("SLINT_BACKEND", "skia");
    }

    // Initialize UI
    let main_window = MainWindow::new()?;
    let app_state = Arc::new(Mutex::new(AppState::new()));
    
    // State for timetable
    let current_week_start: Arc<Mutex<String>> = Arc::new(Mutex::new(get_current_monday().format("%Y-%m-%d").to_string()));

    // Try to load session
    if let Ok(Some(session)) = session::load_session() {
        let auth = session_to_auth(session);
        
        // Check if Synergia credentials are set
        let has_credentials = auth.synergia_username.is_some() && auth.synergia_password.is_some();
        main_window.set_has_synergia_credentials(has_credentials);
        
        let client = LibrusClient::new(auth);
        
        let mut app_state_lock = app_state.lock().await;
        app_state_lock.set_client(client.clone());
        let app_state_clone_for_meta = app_state_lock.clone();
        drop(app_state_lock);
        
        main_window.set_active_page(5);
        
        let client_clone = client.clone();
        let window_weak = main_window.as_weak();
        tokio::spawn(async move {
            fetch_metadata(client_clone.clone(), app_state_clone_for_meta.clone()).await;
            fetch_dashboard_data(client_clone.clone(), window_weak.clone()).await;
            fetch_attendances_data(client_clone, window_weak, app_state_clone_for_meta).await;
        });
    }

    // Login Callback
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    
    main_window.on_request_login(move |email, password| {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        let email = email.to_string();
        let password = password.to_string();

        tokio::spawn(async move {
            let window_weak_start = main_window_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = window_weak_start.upgrade() {
                    window.set_is_logging_in(true);
                    window.set_login_error("".into());
                }
            });

            match LibrusClient::login(&email, &password).await {
                Ok(client) => {
                    let auth = client.get_auth_state().await;
                    let session_data = auth_to_session(auth);
                    let _ = session::save_session(&session_data);

                    let mut state = app_state_clone.lock().await;
                    state.set_client(client.clone());
                    let app_state_copy = state.clone();
                    drop(state);

                    let window_weak_success = main_window_weak.clone();
                    let client_clone = client.clone();
                    
                    // Fetch metadata then dashboard
                    fetch_metadata(client_clone.clone(), app_state_copy).await;
                    fetch_dashboard_data(client_clone, window_weak_success.clone()).await;

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = window_weak_success.upgrade() {
                            window.set_is_logging_in(false);
                            window.set_active_page(1);
                        }
                    });
                }
                Err(e) => {
                    let msg = format!("Login failed: {}", e);
                    let window_weak_fail = main_window_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = window_weak_fail.upgrade() {
                            window.set_is_logging_in(false);
                            window.set_login_error(msg.into());
                        }
                    });
                }
            }
        });
    });

    // Logout
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_request_logout(move || {
        let _ = session::clear_session();
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        
        tokio::spawn(async move {
            let mut state = app_state_clone.lock().await;
            state.client = None;
            drop(state);
            
             let _ = slint::invoke_from_event_loop(move || {
                 if let Some(window) = main_window_weak.upgrade() {
                     window.set_active_page(0);
                     window.set_user_name("...".into());
                     window.set_lucky_number("-".into());
                     window.set_timetable_schedule(std::rc::Rc::new(slint::VecModel::from(vec![])).into());
                 }
             });
        });
    });

    // Timetable Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    let week_start_mutex = current_week_start.clone();

    main_window.on_request_timetable(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        let week_start_mutex = week_start_mutex.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state); 

                // Get date
                let d = week_start_mutex.lock().await.clone();
                fetch_timetable_data(client_clone, main_window_weak.clone(), d, state_copy).await;

                // Navigate
                let _ = slint::invoke_from_event_loop(move || {
                     if let Some(window) = main_window_weak.upgrade() {
                         window.set_active_page(2);
                     }
                });
            }
        });
    });

    // Grades Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_request_grades(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);

                fetch_grades_data(client_clone, main_window_weak.clone(), state_copy).await;

                let _ = slint::invoke_from_event_loop(move || {
                     if let Some(window) = main_window_weak.upgrade() {
                         window.set_active_page(3);
                     }
                });
            }
        });
    });

    // Simulated Grade Callback
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_add_simulated_grade(move |subject_idx, value: slint::SharedString, weight| {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        let val_str = value.to_string();
        
        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            let state_copy = state.clone();
            
            // 1. Identify Subject
            let subj_id = {
                let subjects_map = state.subjects.lock().await;
                let mut subjects_vec: Vec<_> = subjects_map.iter().collect();
                subjects_vec.sort_by(|a, b| a.1.cmp(b.1));
                
                if (subject_idx as usize) < subjects_vec.len() {
                    *subjects_vec[subject_idx as usize].0
                } else {
                    0
                }
            };
            
            if subj_id != 0 {
                // 2. Create Fake Grade
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as i64;
                let neg_id = -timestamp; // Unique negative ID
                
                // 3. Register Fake Category
                {
                    let mut cats = state.grade_categories.lock().await;
                    cats.insert(neg_id, librus_front::api::models::GradeCategory {
                        id: Some(neg_id),
                        name: Some("Symulacja".into()),
                        weight: Some(weight),
                        color: None,
                    });
                }
               
                // 4. Add Grade
                {
                    let mut grades = state.grades.lock().await;
                    grades.push(librus_front::api::models::Grade {
                        id: Some(neg_id),
                        grade: Some(val_str),
                        subject: Some(librus_front::api::models::IdRef{id: Some(subj_id)}),
                        category: Some(librus_front::api::models::IdRef{id: Some(neg_id)}),
                        semester: Some(0), 
                        add_date: Some("Teraz".into()),
                        is_constituent: None, is_semester: None, is_semester_proposition: None, 
                        is_final: None, is_final_proposition: None, 
                        added_by: None, comments: None, improvement: None,
                    });
                }
                
                // 5. Update UI
                drop(state);
                update_grades_ui(main_window_weak, state_copy).await;
            }
        });
    });

// ... main ...

    // Announcements Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_request_announcements(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);

                fetch_announcements_data(client_clone, main_window_weak.clone(), state_copy).await;

                let _ = slint::invoke_from_event_loop(move || {
                     if let Some(window) = main_window_weak.upgrade() {
                         window.set_active_page(4);
                     }
                });
            }
        });
    });

    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    let week_start_mutex = current_week_start.clone();
    main_window.on_request_prev_week(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        let week_start_mutex = week_start_mutex.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);

                let mut date_lock = week_start_mutex.lock().await;
                if let Ok(d) = NaiveDate::parse_from_str(&date_lock, "%Y-%m-%d") {
                    let new_date = d - Duration::days(7);
                    let new_date_str = new_date.format("%Y-%m-%d").to_string();
                    date_lock.clear();
                    date_lock.push_str(&new_date_str);
                    drop(date_lock); // unlock

                    fetch_timetable_data(client_clone, main_window_weak.clone(), new_date_str, state_copy).await;
                }
            }
        });
    });

    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    let week_start_mutex = current_week_start.clone();
    main_window.on_request_next_week(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        let week_start_mutex = week_start_mutex.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);

                let mut date_lock = week_start_mutex.lock().await;
                if let Ok(d) = NaiveDate::parse_from_str(&date_lock, "%Y-%m-%d") {
                    let new_date = d + Duration::days(7);
                    let new_date_str = new_date.format("%Y-%m-%d").to_string();
                    date_lock.clear();
                    date_lock.push_str(&new_date_str);
                    drop(date_lock);

                    fetch_timetable_data(client_clone, main_window_weak.clone(), new_date_str, state_copy).await;
                }
            }
        });
    });

    // Attendance Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_request_attendances(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);

                fetch_attendances_data(client_clone, main_window_weak.clone(), state_copy).await;

                let _ = slint::invoke_from_event_loop(move || {
                     if let Some(window) = main_window_weak.upgrade() {
                         window.set_active_page(5);
                     }
                });
            }
        });
    });

    // Messages Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_request_messages(move || {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                drop(state);

                fetch_messages_data(client_clone, main_window_weak.clone()).await;

                let _ = slint::invoke_from_event_loop(move || {
                     if let Some(window) = main_window_weak.upgrade() {
                         window.set_active_page(6);
                     }
                });
            }
        });
    });

    // Message Details Callback
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_request_message_details(move |id| {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();

        tokio::spawn(async move {
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                drop(state);

                match client_clone.fetch_message_content(id).await {
                    Ok(content) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(window) = main_window_weak.upgrade() {
                                window.set_selected_message_body(content.into());
                                window.set_showing_details(true);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch message details: {:?}", e);
                    }
                }
            }
        });
    });

    // Synergia Credentials Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    main_window.on_submit_synergia_credentials(move |username, password| {
        let main_window_weak = main_window_weak.clone();
        let app_state_clone = app_state_clone.clone();
        let username = username.to_string();
        let password = password.to_string();

        tokio::spawn(async move {
            let mut state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let mut auth = client.get_auth_state().await;
                
                // Store Synergia credentials
                auth.synergia_username = Some(username);
                auth.synergia_password = Some(password);
                
                // Update client auth
                let updated_client = LibrusClient::new(auth.clone());
                state.set_client(updated_client.clone());
                
                // Save session
                let session = auth_to_session(auth);
                if let Err(e) = session::save_session(&session) {
                    eprintln!("Failed to save session with Synergia credentials: {:?}", e);
                }
                
                drop(state);
                
                // Update UI and reload messages
                let window_weak_for_update = main_window_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = window_weak_for_update.upgrade() {
                        window.set_has_synergia_credentials(true);
                    }
                });
                
                // Fetch messages
                fetch_messages_data(updated_client, main_window_weak.clone()).await;
                
                println!("Synergia credentials saved and messages loaded!");
            }
        });
    });

    // Remove skip callback - no longer needed with inline form

    main_window.run()?;
    Ok(())
}
