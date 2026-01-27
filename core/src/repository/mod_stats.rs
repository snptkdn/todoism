use crate::model::stats::MonthlyStats;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileStatsRepository {
    base_dir: PathBuf,
}

impl FileStatsRepository {
    pub fn new(base_dir: Option<PathBuf>) -> Result<Self> {
        let path = match base_dir {
            Some(p) => p,
            None => {
                let mut p = dirs::home_dir().expect("Could not find home directory");
                p.push(".todoism");
                p.push("stats");
                p
            }
        };
        fs::create_dir_all(&path)?;
        Ok(Self { base_dir: path })
    }

    pub fn get_stats(&self, year: i32, month: u32) -> Result<MonthlyStats> {
        let filename = format!("stats_{:04}_{:02}.json", year, month);
        let path = self.base_dir.join(filename);

        if path.exists() {
            let content = fs::read_to_string(path)?;
            let stats: MonthlyStats = serde_json::from_str(&content)?;
            Ok(stats)
        } else {
            Ok(MonthlyStats::new(year, month))
        }
    }

    pub fn save_stats(&self, stats: &MonthlyStats) -> Result<()> {
        let filename = format!("stats_{:04}_{:02}.json", stats.year, stats.month);
        let path = self.base_dir.join(filename);
        let content = serde_json::to_string_pretty(stats)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    pub fn list_stats(&self) -> Result<Vec<MonthlyStats>> {
        let mut stats_list = Vec::new();
        if self.base_dir.exists() {
            for entry in fs::read_dir(&self.base_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = fs::read_to_string(path)?;
                    if let Ok(stats) = serde_json::from_str::<MonthlyStats>(&content) {
                        stats_list.push(stats);
                    }
                }
            }
        }
        Ok(stats_list)
    }
}
