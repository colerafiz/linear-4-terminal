use regex::Regex;
use serde_json::{json, Value};

#[derive(Debug)]
pub struct FilterQuery {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Debug)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    In,
    HasAssignee,
    NoAssignee,
    HasLabel,
    NoLabel,
}

pub fn parse_filter_query(query: &str) -> Result<Vec<FilterQuery>, String> {
    let mut filters = Vec::new();
    
    // Handle special cases first
    if query.contains("has-assignee") {
        filters.push(FilterQuery {
            field: "assignee".to_string(),
            operator: FilterOperator::HasAssignee,
            value: String::new(),
        });
    }
    
    if query.contains("no-assignee") {
        filters.push(FilterQuery {
            field: "assignee".to_string(),
            operator: FilterOperator::NoAssignee,
            value: String::new(),
        });
    }
    
    // Handle has-label:name patterns
    let has_label_re = Regex::new(r"has-label:(\S+)").unwrap();
    for cap in has_label_re.captures_iter(query) {
        filters.push(FilterQuery {
            field: "label".to_string(),
            operator: FilterOperator::HasLabel,
            value: cap[1].to_string(),
        });
    }
    
    if query.contains("no-label") {
        filters.push(FilterQuery {
            field: "label".to_string(),
            operator: FilterOperator::NoLabel,
            value: String::new(),
        });
    }
    
    // Parse field:operator:value patterns
    let re = Regex::new(r"(\w+):(!=|>=|<=|>|<|~|in:|)([^AND\s]+)").unwrap();
    
    for cap in re.captures_iter(query) {
        let field = cap[1].to_string();
        let op_str = &cap[2];
        let value = cap[3].to_string();
        
        // Skip special fields we already handled
        if field == "has" || field == "no" || field == "label" {
            continue;
        }
        
        let operator = match op_str {
            "!=" => FilterOperator::NotEquals,
            ">" | ">=" => FilterOperator::GreaterThan,
            "<" | "<=" => FilterOperator::LessThan,
            "~" => FilterOperator::Contains,
            "in:" => FilterOperator::In,
            _ => FilterOperator::Equals,
        };
        
        filters.push(FilterQuery {
            field,
            operator,
            value,
        });
    }
    
    if filters.is_empty() {
        return Err("No valid filters found in query".to_string());
    }
    
    Ok(filters)
}

pub fn build_graphql_filter(filters: Vec<FilterQuery>) -> Value {
    let mut filter_obj = json!({});
    
    for filter in filters {
        match (filter.field.as_str(), &filter.operator) {
            ("assignee", FilterOperator::Equals) => {
                filter_obj["assignee"] = json!({ "email": { "eq": filter.value } });
            }
            ("assignee", FilterOperator::HasAssignee) => {
                filter_obj["assignee"] = json!({ "null": false });
            }
            ("assignee", FilterOperator::NoAssignee) => {
                filter_obj["assignee"] = json!({ "null": true });
            }
            ("state", FilterOperator::Equals) => {
                filter_obj["state"] = json!({ "name": { "eq": filter.value } });
            }
            ("priority", FilterOperator::GreaterThan) => {
                if let Ok(priority) = filter.value.parse::<u8>() {
                    filter_obj["priority"] = json!({ "gte": priority });
                }
            }
            ("priority", FilterOperator::LessThan) => {
                if let Ok(priority) = filter.value.parse::<u8>() {
                    filter_obj["priority"] = json!({ "lte": priority });
                }
            }
            ("priority", FilterOperator::Equals) => {
                if let Ok(priority) = filter.value.parse::<u8>() {
                    filter_obj["priority"] = json!({ "eq": priority });
                }
            }
            ("title", FilterOperator::Contains) => {
                filter_obj["title"] = json!({ "containsIgnoreCase": filter.value });
            }
            ("description", FilterOperator::Contains) => {
                filter_obj["description"] = json!({ "containsIgnoreCase": filter.value });
            }
            ("created", FilterOperator::GreaterThan) => {
                if let Some(date) = parse_relative_date(&filter.value) {
                    filter_obj["createdAt"] = json!({ "gte": date });
                }
            }
            ("created", FilterOperator::LessThan) => {
                if let Some(date) = parse_relative_date(&filter.value) {
                    filter_obj["createdAt"] = json!({ "lte": date });
                }
            }
            ("updated", FilterOperator::GreaterThan) => {
                if let Some(date) = parse_relative_date(&filter.value) {
                    filter_obj["updatedAt"] = json!({ "gte": date });
                }
            }
            ("updated", FilterOperator::LessThan) => {
                if let Some(date) = parse_relative_date(&filter.value) {
                    filter_obj["updatedAt"] = json!({ "lte": date });
                }
            }
            ("label", FilterOperator::HasLabel) => {
                filter_obj["labels"] = json!({ 
                    "some": { 
                        "name": { "eq": filter.value } 
                    } 
                });
            }
            ("label", FilterOperator::NoLabel) => {
                filter_obj["labels"] = json!({ "every": { "id": { "null": true } } });
            }
            _ => {}
        }
    }
    
    filter_obj
}

pub fn parse_relative_date(input: &str) -> Option<String> {
    use chrono::{Duration, Utc};
    
    let re = Regex::new(r"(\d+)(day|week|month|hour)s?").unwrap();
    if let Some(captures) = re.captures(input) {
        let amount = captures[1].parse::<i64>().ok()?;
        let unit = &captures[2];
        
        let duration = match unit {
            "hour" => Duration::hours(amount),
            "day" => Duration::days(amount),
            "week" => Duration::weeks(amount),
            "month" => Duration::days(amount * 30), // Approximation
            _ => return None,
        };
        
        let date = Utc::now() - duration;
        return Some(date.to_rfc3339());
    }
    
    None
}