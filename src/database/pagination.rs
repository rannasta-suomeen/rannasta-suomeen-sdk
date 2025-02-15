use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PageContext<T> {
    pub rows: Vec<T>,
    pub total_rows: i64,
    pub next_offset: i64,
    pub prev_offset: i64,
    pub page_list: Vec<(String, i64)>,
    pub message: Option<String>,
}

impl<T> PageContext<T> {
    pub fn from_rows(rows: Vec<T>, total_rows: i64, page_size: i64, current_offset: i64) -> Self {
        if rows.len() <= 0 {
            return Self::no_rows();
        }
        let next_offset = (current_offset + page_size).min(total_rows - (total_rows % page_size));
        let prev_offset = (current_offset - page_size).max(0);

        let page_count = ((total_rows / page_size) as f64).ceil() as usize;
        let page_count = page_count + if total_rows <= page_size { 0 } else { 1 };

        let page_list = (0..page_count)
            .map(|n| {
                let page = if n == ((current_offset / page_size) as f64).floor() as usize {
                    String::from("...")
                } else {
                    format!("{}", n + 1)
                };

                let offset = ((n as i64) * page_size).min(total_rows - (total_rows % page_size));

                (page, offset)
            })
            .collect();

        Self {
            rows,
            total_rows,
            next_offset,
            prev_offset,
            page_list,
            message: Some(format!(
                "{} - {} / {}",
                current_offset,
                (current_offset + page_size).min(total_rows),
                total_rows
            )),
        }
    }

    pub fn no_rows() -> Self {
        Self {
            rows: vec![],
            total_rows: 0,
            next_offset: 0,
            prev_offset: 0,
            page_list: vec![(String::from("1"), 0)],
            message: Some(String::from("No results")),
        }
    }
}
