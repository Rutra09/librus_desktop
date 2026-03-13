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
use librus_front::updater;

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
        synergia_cookie: s.synergia_cookie,
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
        synergia_cookie: a.synergia_cookie,
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
    // 6. Event/Homework Categories (For Timetable & Homework)
    if let Ok(cats) = client.fetch_homework_categories().await {
        let mut hw_map = state.homework_categories.lock().await;
        let mut ev_map = state.event_categories.lock().await;
        for c in cats {
            if let Some(id) = c.id {
                let name = c.name.clone().unwrap_or("?".into());
                hw_map.insert(id, c);
                ev_map.insert(id, name);
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

// fn int_to_color(val: i32) -> slint::Color {
//     let r = ((val >> 16) & 0xFF) as u8;
//     let g = ((val >> 8) & 0xFF) as u8;
//     let b = (val & 0xFF) as u8;
//     slint::Color::from_rgb_u8(r, g, b)
// }

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
        let mut events_map: HashMap<(String, i32), Vec<(String, String)>> = HashMap::new();
        if let Ok(events) = events_res {
            let event_cats = state.event_categories.lock().await;
            for e in events {
                if let Some(date) = &e.date {
                    let l_no = e.lesson_no.unwrap_or(0);
                    let cat_id = e.category.as_ref().and_then(|c| c.id).unwrap_or(0);
                    let cat_name = event_cats.get(&cat_id).cloned().unwrap_or("?".to_string());
                    let content = e.content.clone().unwrap_or("".to_string());
                    
                    if !content.is_empty() {
                         events_map.entry((date.clone(), l_no)).or_insert(Vec::new()).push((cat_name, content));
                    }
                }
            }
        }
        
        // Cache locks
        let subjects_map = state.subjects.lock().await;
        let teachers_map = state.teachers.lock().await;
        let classrooms_map = state.classrooms.lock().await;

        // Helper closures
        let resolve_subject = |id_ref: &Option<librus_front::api::models::IdRef>| -> String {
            id_ref.as_ref().map(|s| {
                let id = s.id.unwrap_or(0);
                subjects_map.get(&id).cloned().unwrap_or(format!("Subject {}", id))
            }).unwrap_or("?".into())
        };
        let resolve_teacher = |id_ref: &Option<librus_front::api::models::IdRef>| -> String {
            id_ref.as_ref().map(|t| {
                let id = t.id.unwrap_or(0);
                teachers_map.get(&id).cloned().unwrap_or(format!("Teacher {}", id))
            }).unwrap_or("?".into())
        };
        let resolve_room = |id_ref: &Option<librus_front::api::models::IdRef>| -> String {
            id_ref.as_ref().and_then(|c| c.id).map(|id| {
                classrooms_map.get(&id).and_then(|r| r.name.clone().or(r.symbol.clone()))
                    .unwrap_or(format!("{}", id))
            }).unwrap_or("-".into())
        };

        // Build grid: slot_no -> day_index -> UIGridCell
        // day_index is 0=Mon .. 4=Fri based on parsed date position
        let mut day_headers: Vec<String> = Vec::new();
        let mut date_to_day_idx: HashMap<String, usize> = HashMap::new();

        // Sort dates and assign day indices, filtering empty weekends
        let mut sorted_dates: Vec<String> = parsed_days.iter().map(|(d, _)| d.clone()).collect();
        sorted_dates.sort();
        
        // Filter out Sat/Sun that have no lessons
        sorted_dates.retain(|date_str| {
            if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let weekday = d.weekday();
                if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
                    // Keep only if this day has actual lessons
                    parsed_days.iter()
                        .find(|(ds, _)| ds == date_str)
                        .map(|(_, lessons)| !lessons.is_empty())
                        .unwrap_or(false)
                } else {
                    true // Always keep weekdays
                }
            } else {
                true
            }
        });

        for (i, date_str) in sorted_dates.iter().enumerate() {
            date_to_day_idx.insert(date_str.clone(), i);
            let header = if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let day_name = match d.weekday() {
                    chrono::Weekday::Mon => "Pon",
                    chrono::Weekday::Tue => "Wt",
                    chrono::Weekday::Wed => "Śr",
                    chrono::Weekday::Thu => "Czw",
                    chrono::Weekday::Fri => "Pt",
                    chrono::Weekday::Sat => "Sob",
                    chrono::Weekday::Sun => "Nd",
                };
                format!("{} {}", day_name, d.format("%d.%m"))
            } else {
                format!("Dzień {}", i + 1)
            };
            day_headers.push(header);
        }

        let num_days = day_headers.len();
        // slot_no -> (hour_from, hour_to, [UIGridCell; num_days])
        let mut grid: HashMap<i32, (String, String, Vec<UIGridCell>)> = HashMap::new();

        for (date_str, lessons) in &parsed_days {
            let day_idx = *date_to_day_idx.get(date_str).unwrap_or(&0);

            for l in lessons {
                let lesson_no = l.lesson_no.unwrap_or(0);
                let is_sub = l.is_substitution_class.unwrap_or(false);
                let is_can = l.is_canceled.unwrap_or(false);

                let subject_name = resolve_subject(&l.subject);
                let teacher_name = resolve_teacher(&l.teacher);
                let room_name = resolve_room(&l.classroom);
                let hour_from = l.hour_from.clone().unwrap_or_default();
                let hour_to = l.hour_to.clone().unwrap_or_default();

                // Classify lesson type (Szkolny logic)
                let (lesson_type, change_info) = if is_sub && is_can {
                    // Shifted source: lesson moved away from this slot
                    let info = if let Some(new_date) = &l.new_date {
                        let new_no = l.new_lesson_no.unwrap_or(0);
                        format!("→ {} #{}", new_date, new_no)
                    } else {
                        String::new()
                    };
                    (3i32, info) // TYPE_SHIFTED_SOURCE
                } else if is_sub && !is_can {
                    // Check if it's a change (same slot) or shifted target (different slot)
                    let org_date = l.org_date.as_deref().unwrap_or("");
                    let org_lesson_no = l.org_lesson_no.unwrap_or(lesson_no);
                    
                    if org_date == date_str.as_str() && org_lesson_no == lesson_no {
                        // TYPE_CHANGE: same slot, teacher/subject changed
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
                        let info = if !parts.is_empty() {
                            format!("było: {}", parts.join(" "))
                        } else {
                            String::new()
                        };
                        (2, info) // TYPE_CHANGE
                    } else {
                        // TYPE_SHIFTED_TARGET: lesson moved here from another slot
                        let info = if !org_date.is_empty() {
                            format!("← {} #{}", org_date, org_lesson_no)
                        } else {
                            // Fallback: just show original info
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
                                format!("było: {}", parts.join(" "))
                            } else {
                                String::new()
                            }
                        };
                        (4, info) // TYPE_SHIFTED_TARGET
                    }
                } else if is_can {
                    // TYPE_CANCELLED: plain cancellation
                    (1, String::new())
                } else {
                    // TYPE_NORMAL
                    (0, String::new())
                };

                // Lookup event
                let (event_content, event_category) = if let Some(evs) = events_map.get(&(date_str.clone(), lesson_no)) {
                    let contents = evs.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>().join("\n");
                    let categories = evs.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>().join(", ");
                    (contents, format!("Wydarzenie • {}", categories))
                } else {
                    ("".to_string(), "".to_string())
                };

                let cell = UIGridCell {
                    has_lesson: true,
                    subject: subject_name.into(),
                    teacher: teacher_name.into(),
                    room: room_name.into(),
                    hour_from: hour_from.clone().into(),
                    hour_to: hour_to.clone().into(),
                    lesson_type: lesson_type,
                    event_content: event_content.into(),
                    event_category: event_category.into(),
                    change_info: change_info.into(),
                };

                let entry = grid.entry(lesson_no).or_insert_with(|| {
                    let empty_cells: Vec<UIGridCell> = (0..num_days).map(|_| UIGridCell {
                        has_lesson: false,
                        subject: "".into(),
                        teacher: "".into(),
                        room: "".into(),
                        hour_from: "".into(),
                        hour_to: "".into(),
                        lesson_type: 0,
                        event_content: "".into(),
                        event_category: "".into(),
                        change_info: "".into(),
                    }).collect();
                    (hour_from.clone(), hour_to.clone(), empty_cells)
                });

                // Update hour range if needed
                if entry.0.is_empty() && !hour_from.is_empty() {
                    entry.0 = hour_from;
                    entry.1 = hour_to;
                }

                if day_idx < entry.2.len() {
                    entry.2[day_idx] = cell;
                }
            }
        }
        
        // Sort by slot number and collect raw data (no ModelRc yet – those are not Send)
        let mut slot_numbers: Vec<i32> = grid.keys().cloned().collect();
        slot_numbers.sort();

        let mut raw_rows: Vec<(i32, String, String, Vec<UIGridCell>)> = Vec::new();
        for slot_no in slot_numbers {
            if let Some((hour_from, hour_to, cells)) = grid.remove(&slot_no) {
                raw_rows.push((slot_no, hour_from, hour_to, cells));
            }
        }

        // Build day headers as plain strings (Send)
        let ui_day_headers: Vec<slint::SharedString> = day_headers.into_iter().map(|s| s.into()).collect();

        // Determine today's column index
        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_col: i32 = sorted_dates.iter()
            .position(|d| d == &today_str)
            .map(|p| p as i32)
            .unwrap_or(-1);

        // Update UI – construct ModelRc on the UI thread
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = window_weak.upgrade() {
                let ui_rows: Vec<UIGridRow> = raw_rows.into_iter().map(|(no, hf, ht, cells)| {
                    UIGridRow {
                        no,
                        hour_from: hf.into(),
                        hour_to: ht.into(),
                        cells: std::rc::Rc::new(slint::VecModel::from(cells)).into(),
                    }
                }).collect();

                let rows_model = std::rc::Rc::new(slint::VecModel::from(ui_rows));
                let headers_model = std::rc::Rc::new(slint::VecModel::from(ui_day_headers));
                window.set_timetable_grid_rows(rows_model.into());
                window.set_timetable_day_headers(headers_model.into());
                window.set_timetable_week_range(week_start_str.into());
                window.set_timetable_today_col(today_col);
            }
        });
    }
}

/// Silently prefetch adjacent weeks to warm cache for smooth navigation
fn prefetch_adjacent_weeks(client: LibrusClient, week_start_str: &str) {
    if let Ok(base_date) = NaiveDate::parse_from_str(week_start_str, "%Y-%m-%d") {
        for offset in &[-14i64, -7, 7, 14] {
            let adj_date = base_date + Duration::days(*offset);
            let adj_str = adj_date.format("%Y-%m-%d").to_string();
            let client_clone = client.clone();
            tokio::spawn(async move {
                let _ = client_clone.fetch_timetable(&adj_str).await;
            });
        }
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
            let topic = "".to_string(); // Not available in Attendance or Lesson?
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
async fn fetch_homework_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>, state: AppState) {
    // 1. Fetch Categories (if empty)
    {
        let mut cats = state.homework_categories.lock().await;
        if cats.is_empty() {
            if let Ok(new_cats) = client.fetch_homework_categories().await {
                for c in new_cats {
                    if let Some(id) = c.id {
                        cats.insert(id, c);
                    }
                }
            }
        }
    }
    
    // 2. Fetch Homework
    // Calculate start of the school year (Sept 1st)
    let today = chrono::Local::now().date_naive();
    let current_year = today.year();
    let start_year = if today.month() >= 9 {
        current_year
    } else {
        current_year - 1
    };
    
    let from = format!("{}-09-01", start_year);
    let to = (today + chrono::Duration::days(30)).format("%Y-%m-%d").to_string();

    // Use Synergia Scraping for Homework
    match client.fetch_homework_via_synergia(Some(&from), Some(&to)).await {
        Ok(homeworks) => {
            println!("Fetched {} homework entries via Synergia scraping.", homeworks.len());
            let subjects_map = state.subjects.lock().await;
            // let cats_map = state.homework_categories.lock().await;
            let teachers_map = state.teachers.lock().await;
            
            let mut ui_homeworks = Vec::new();
            
            for hw in homeworks {
                 // For scraped data, we check transient fields first
                 
                 let subject_name: slint::SharedString = if let Some(name) = &hw.scraped_subject {
                     name.clone().into()
                 } else {
                     let subj_id = hw.subject.as_ref().and_then(|s| s.id).unwrap_or(0);
                     subjects_map.get(&subj_id).cloned().unwrap_or("?".into()).into()
                 };

                 let category: slint::SharedString = if let Some(cat) = &hw.scraped_category {
                     cat.clone().into()
                 } else {
                     let cat_id = hw.category.as_ref().and_then(|c| c.id).unwrap_or(0);
                     // cats_map is not locked here currently, assumed default or fixed
                     if cat_id == -1 {
                         "Zadanie domowe".into()
                     } else {
                         // cats_map.get(&cat_id)...
                         "Zadanie domowe".into()
                     }
                 };
                 
                 let author: slint::SharedString = if let Some(teacher) = &hw.scraped_teacher {
                     teacher.clone().into()
                 } else {
                     let author_id = hw.created_by.as_ref().and_then(|u| u.id).unwrap_or(0);
                     teachers_map.get(&author_id).cloned().unwrap_or("?".into()).into()
                 };
                 
                 let date = hw.date.clone().unwrap_or("-".into());
                 
                 ui_homeworks.push(UIHomework {
                     id: hw.id.unwrap_or(0) as i32,
                     subject: subject_name,
                     content: hw.content.unwrap_or("".into()).into(),
                     date: date.into(),
                     category: category,
                     author: author,
                 });
            }
            
            // Sort by date desc
            ui_homeworks.sort_by(|a, b| b.date.cmp(&a.date));
            
            let _ = slint::invoke_from_event_loop(move || {
                 let model = std::rc::Rc::new(slint::VecModel::from(ui_homeworks));
                 if let Some(window) = window_weak.upgrade() {
                     window.set_homework_list(model.into());
                 }
            });
        }
        Err(e) => {
            eprintln!("Failed to fetch homework via Synergia: {:?}", e);
            // Fallback to API if sync fails? 
            // For now just error is enough debug info.
        }
    }
}

async fn refresh_all_data(client: LibrusClient, window_weak: slint::Weak<MainWindow>, state: AppState, week_start: String) {
    // 1. Metadata (fast, sync-ish)
    fetch_metadata(client.clone(), state.clone()).await;
    
    // 2. Data fetching (concurrent)
    let t1 = fetch_dashboard_data(client.clone(), window_weak.clone());
    let t2 = fetch_grades_data(client.clone(), window_weak.clone(), state.clone());
    let t3 = fetch_timetable_data(client.clone(), window_weak.clone(), week_start, state.clone());
    let t4 = fetch_announcements_data(client.clone(), window_weak.clone(), state.clone());
    let t5 = fetch_attendances_data(client.clone(), window_weak.clone(), state.clone());
    let t6 = fetch_messages_data(client.clone(), window_weak.clone());
    let t7 = fetch_homework_data(client.clone(), window_weak.clone(), state.clone());
    
    // Wait for all
    tokio::join!(t1, t2, t3, t4, t5, t6, t7);
    
    println!("Refresh complete.");
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

    // Check for updates
    let mw_weak = main_window.as_weak();
    tokio::spawn(async move {
        if let Ok(Some(version)) = updater::check_for_updates().await {
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = mw_weak.upgrade() {
                    window.set_update_version(version.into());
                    window.set_show_update_dialog(true);
                }
            });
        }
    });

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
        
        main_window.set_active_page(1);
        
        let client_clone = client.clone();
        let window_weak = main_window.as_weak();
        let week_start_clone = current_week_start.clone();
        tokio::spawn(async move {
            // fetch_metadata(client_clone.clone(), app_state_clone_for_meta.clone()).await;
            // fetch_dashboard_data(client_clone.clone(), window_weak.clone()).await;
            // fetch_attendances_data(client_clone, window_weak, app_state_clone_for_meta).await;
            refresh_all_data(client_clone, window_weak, app_state_clone_for_meta, week_start_clone.lock().await.clone()).await;

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
                     window.set_timetable_grid_rows(std::rc::Rc::new(slint::VecModel::from(vec![] as Vec<UIGridRow>)).into());
                     window.set_timetable_day_headers(std::rc::Rc::new(slint::VecModel::from(vec![] as Vec<slint::SharedString>)).into());
                 }
             });
        });
    });

    // Timetable Callbacks



    // removed on_request_timetable

    // Grades Callbacks
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    // removed on_request_grades

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


    // removed on_request_announcements

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

                    fetch_timetable_data(client_clone.clone(), main_window_weak.clone(), new_date_str.clone(), state_copy).await;
                    prefetch_adjacent_weeks(client_clone, &new_date_str);
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

                    fetch_timetable_data(client_clone.clone(), main_window_weak.clone(), new_date_str.clone(), state_copy).await;
                    prefetch_adjacent_weeks(client_clone, &new_date_str);
                }
            }
        });
    });


    // removed on_request_attendances


    // removed on_request_messages

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


    // removed on_request_homework



    // Refresh Callback
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    let week_start_refresh = current_week_start.clone();
    
    main_window.on_request_refresh(move || {
        let window = main_window_weak.clone();
        let state_arc = app_state_clone.clone();
        let week_arc = week_start_refresh.clone();
        
        tokio::spawn(async move {
            let state = state_arc.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);
                
                let date = week_arc.lock().await.clone();
                println!("Manual refresh initiated...");
                refresh_all_data(client_clone, window, state_copy, date).await;
            }
        });
    });

    // Auto-Refresh Loop (5 minutes)
    let main_window_weak = main_window.as_weak();
    let app_state_clone = app_state.clone();
    let week_start_refresh = current_week_start.clone();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // 5 minutes
            
            let state = app_state_clone.lock().await;
            if let Some(client) = &state.client {
                let client_clone = client.clone();
                let state_copy = state.clone();
                drop(state);
                
                // Check if window is still alive
                if main_window_weak.upgrade().is_none() {
                    break;
                }

                let date = week_start_refresh.lock().await.clone();
                println!("Auto-refresh initiated...");
                refresh_all_data(client_clone, main_window_weak.clone(), state_copy, date).await;
            } else {
                 drop(state);
            }
        }
    });

    // Remove skip callback - no longer needed with inline form


    // Updater callbacks
    let mw_weak_updater = main_window.as_weak();
    main_window.on_close_update_dialog(move || {
        if let Some(window) = mw_weak_updater.upgrade() {
            window.set_show_update_dialog(false);
        }
    });

    let mw_weak_installer = main_window.as_weak();
    main_window.on_request_install_update(move || {
        let mw_weak_1 = mw_weak_installer.clone();
        let mw_weak_2 = mw_weak_installer.clone();
        tokio::spawn(async move {
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = mw_weak_1.upgrade() {
                    window.set_is_updating(true);
                }
            });
            match updater::install_update().await {
                Ok(_) => {
                    let mw_weak_ok = mw_weak_2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = mw_weak_ok.upgrade() {
                            window.set_update_status("Zaktualizowano pomyślnie. Zrestartuj aplikację.".into());
                            window.set_is_updating(false); // Done visually
                        }
                    });
                }
                Err(e) => {
                    let err_msg = format!("Błąd pobierania: {}", e);
                    let mw_weak_err = mw_weak_2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = mw_weak_err.upgrade() {
                            window.set_update_status(err_msg.into());
                            window.set_is_updating(false);
                        }
                    });
                }
            }
        });
    });

    main_window.run()?;
    Ok(())
}
