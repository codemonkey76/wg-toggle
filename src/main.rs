use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io;
use std::process::Command;

const STATE_FILE: &str = "/tmp/wg-current";

fn get_wireguard_connections() -> io::Result<Vec<String>> {
    let output = Command::new("nmcli")
        .args(["-g", "NAME,TYPE", "connection", "show"])
        .output()?;
    let list = String::from_utf8_lossy(&output.stdout);
    let mut result = Vec::new();

    for line in list.lines() {
        if let Some((name, "wireguard")) = line.split_once(':') {
            result.push(name.to_string());
        }
    }

    result.sort();
    Ok(result)
}

fn read_state(default: &str) -> String {
    fs::read_to_string(STATE_FILE).unwrap_or_else(|_| default.to_string())
}

fn write_state(current: &str) {
    let _ = fs::write(STATE_FILE, current);
}

fn rotate_current(current: &str, list: &[String], reverse: bool) -> String {
    let mut queue: VecDeque<_> = list.iter().collect();
    while queue.front().map(|s| s.as_str()) != Some(current) {
        let front = queue.pop_front().unwrap();
        queue.push_back(front);
    }
    if reverse {
        let back = queue.pop_back().unwrap();
        queue.push_front(back);
    } else {
        let front = queue.pop_front().unwrap();
        queue.push_back(front);
    }
    queue.front().unwrap().to_string()
}

fn is_active(conn: &str) -> bool {
    let output = Command::new("nmcli")
        .args(["connection", "show", "--active"])
        .output()
        .unwrap_or_else(|_| panic!("Failed to query active connections"));

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.contains(conn))
}

fn toggle_connection(conn: &str) {
    if is_active(conn) {
        let _ = Command::new("nmcli")
            .args(["connection", "down", conn])
            .output();
        println!("{{\"text\": \"{}\", \"class\": \"inactive\"}}", conn);
    } else {
        let _ = Command::new("nmcli")
            .args(["connection", "up", conn])
            .output();
        println!("{{\"text\": \"{}\", \"class\": \"active\"}}", conn);
    }
}

fn status_output(conn: &str) {
    if is_active(conn) {
        println!("{{\"text\": \"🛡️  {}\", \"class\": \"active\"}}", conn);
    } else {
        println!("{{\"text\": \"🚫  {}\", \"class\": \"inactive\"}}", conn);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let conn_list = get_wireguard_connections().unwrap_or_else(|_| vec![]);

    if conn_list.is_empty() {
        println!("{{\"text\": \"No VPNs\", \"class\": \"inactive\"}}");
        return;
    }

    let current = read_state(&conn_list[0]);

    let next_conn = match args.get(1).map(|s| s.as_str()) {
        Some("next") => {
            let rotated = rotate_current(&current, &conn_list, false);
            write_state(&rotated);
            status_output(&rotated);
            return;
        }
        Some("previous") => {
            let rotated = rotate_current(&current, &conn_list, true);
            write_state(&rotated);
            status_output(&rotated);
            return;
        }
        Some("--status") => {
            status_output(&current);
            return;
        }
        _ => current,
    };

    toggle_connection(&next_conn);
}
