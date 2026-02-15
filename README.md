# Librus Front

A desktop client for the Librus Synergia electronic school register, built with Rust and Slint.

## Features

- Dashboard: Overview of lucky number, next lesson, and recent announcements.
- Timetable: Weekly view of scheduled lessons.
- Grades: Detailed list of grades with average calculations and grade simulation.
- Attendance: History of absences, lates, and presences with percentage statistics.
- Messages: Full message access including attachments and detail viewing.

## Technical Stack

- Language: Rust
- UI Framework: Slint (using Skia renderer)
- Networking: Reqwest with persistent cookie management
- Data Parsing: Serde (JSON) and roxmltree (XML)

## Setup

### Prerequisites

- Rust toolchain (cargo, rustc)
- C++ compiler (required by Slint)

### Building and Running

1. Clone the repository.
2. Build the project:
   ```bash
   cargo build
   ```
3. Run the application:
   ```bash
   cargo run
   ```

Note: The application automatically initializes with the Skia backend for optimal rendering quality.

## License

This project is intended for personal use.
