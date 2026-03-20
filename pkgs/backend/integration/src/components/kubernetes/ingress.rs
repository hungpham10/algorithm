use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use chrono;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use vector_config_macro::transform;

use vector_runtime::{Component, Event, Identify, Message};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LogEntry {
    pub host: String,
    pub http_status: i32,
    pub request_uri: String,
    pub remote_addr: Option<String>,
    pub http_x_forwarded_for: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SubnetConfig {
    pub name: String,
    pub cidr: String,
}

#[transform(derive(PartialEq))]
pub struct IngressMappingTransform {
    id: String,
    inputs: Vec<String>,
    max_path_depth: usize,
    error_codes: Vec<i32>,
    subnets: Vec<SubnetConfig>,
    domain_service_map: HashMap<String, String>,
    exlusive_domains: HashSet<String>,
}

fn is_valid_segment(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut has_letter = false;
    for c in s.chars() {
        if c.is_ascii_lowercase() {
            has_letter = true;
        } else if !c.is_ascii_digit() && c != '-' {
            return false;
        }
    }
    has_letter
}

fn get_truncated_path(uri: &str, depth: usize) -> String {
    let path_only = uri.split('?').next().unwrap_or("/");
    let segments: Vec<&str> = path_only
        .split('/')
        .filter(|s| is_valid_segment(s))
        .take(depth)
        .collect();

    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

impl_ingress_mapping_transform!(
    async fn run(
        &self,
        _: usize,
        rx: &mut mpsc::Receiver<Message>,
        _: &'life2 [mpsc::Sender<Message>],
        _: &mpsc::Sender<Event>,
    ) -> Result<(), std::io::Error> {
        // Chuyển error_codes vào HashSet để tra cứu O(1)
        let error_set: HashSet<i32> = self.error_codes.iter().cloned().collect();

        let parsed_subnets = self
            .subnets
            .iter()
            .map(|s| {
                let network: IpNet = s.cidr.parse().expect("Invalid CIDR format");
                (s.name.clone(), network)
            })
            .collect::<Vec<(String, IpNet)>>();

        let mut ip_database: HashMap<String, (String, HashSet<(String, String)>)> = HashMap::new();

        while let Some(message) = rx.recv().await {
            if let Ok(log_entry) = serde_json::from_value::<LogEntry>(message.payload.clone()) {
                let real_ip_str = log_entry
                    .remote_addr
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                let call_type = if let Ok(ip) = real_ip_str.parse::<IpAddr>() {
                    match parsed_subnets.iter().find(|(_, net)| net.contains(&ip)) {
                        Some((name, _)) => format!("INT ({})", name),
                        None => "EXT".to_string(),
                    }
                } else {
                    "UNK".to_string()
                };

                let (service_display, is_new) = match self.domain_service_map.get(&log_entry.host) {
                    Some(name) => (name.as_str(), false),
                    None => (log_entry.host.as_str(), true),
                };

                let truncated_path =
                    get_truncated_path(&log_entry.request_uri, self.max_path_depth);
                let entry = ip_database
                    .entry(real_ip_str.clone())
                    .or_insert_with(|| (call_type.clone(), HashSet::new()));

                let (source_label, mappings) = entry;
                let mapping_key = (log_entry.host.clone(), truncated_path.clone());

                if !mappings.contains(&mapping_key) {
                    mappings.insert(mapping_key);
                }

                let now = chrono::Local::now().format("%H:%M:%S");
                let prefix = if is_new { "⭐" } else { "🔥" };

                // @TODO: đẩy dữ liệu qua một luồng
                if error_set.contains(&log_entry.http_status)
                    && !self.exlusive_domains.contains(&log_entry.host)
                {
                    println!(
                        "{} [{}] {} | {:<15} | IP: {:<15} | SVC/DOM: {:<45} | PATH: {}",
                        prefix,
                        now,
                        log_entry.http_status,
                        source_label,
                        real_ip_str,
                        service_display,
                        truncated_path
                    );
                }
            }
        }
        Ok(())
    }
);
