use chrono::Duration;
use std::collections::VecDeque;
use prettytable::{Table, Row, Cell, format};

// 结构体用于记录查询信息
#[derive(Debug, Clone)]
pub struct QueryRecord {
    pub text: String,
    pub duration: Duration,
}

// 管理查询记录的结构体
#[derive(Default)]
pub struct QueryTracker {
    pub total_cnts: usize,
    pub start_time: String,
    pub records: VecDeque<QueryRecord>,
}

impl QueryTracker {
    pub fn new(start_time: String ) -> QueryTracker {
        Self {
            total_cnts: 0,
            start_time,
            ..Default::default()
        }
    }

    // 记录查询信息
    pub fn record_query(&mut self, text: String, duration: Duration) {
        let record = QueryRecord { text, duration };

        // 保留最近10次查询记录
        if self.records.len() >= 10 {
            self.records.pop_front();
        }

        self.records.push_back(record);
        self.total_cnts += 1;
    }

    // 获取查询记录
    pub fn get_total_cnts(&self) -> usize {
        self.total_cnts
    }

    // 获取查询记录
    pub fn get_records(&self) -> Vec<QueryRecord> {
        self.records.iter().cloned().collect()
    }

    pub fn to_table_string(&self) -> String {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

        table.add_row(Row::new(vec![
            Cell::new("Index"),
            Cell::new("Duration (s)"),
            Cell::new("Text"),
        ]));

        for(index, record) in self.records.iter().enumerate() {
            table.add_row(Row::new(vec![
                Cell::new(&index.to_string()),
                Cell::new(&(record.duration.num_milliseconds() as f64 / 1000.0).to_string()),
                Cell::new(&record.text),
            ]));
        }

        format!("Start at: {}\nTotal Query Times:{}\n\nLast 10 Queries:\n{}", self.start_time, self.total_cnts, table)
    }
}