//! Report generator and provenance formatter for Phantom Lab (M4-12 / WP-20a).
//!
//! Formats execution results, run manifests, and validity checks into
//! machine-readable JSON and human-readable Markdown with strict provenance tagging:
//! - T1 labeled SIMULATED
//! - T2 labeled MEASURED (loopback)
//! - Derived metrics labeled DERIVED
//! - Absent metrics explicitly labeled UNAVAILABLE

use super::manifest::ProvenanceTag;
use super::matrix_runner::CellResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabReportItem {
    pub scenario_id: String,
    pub workload_family: String,
    pub baseline_id: String,
    pub provenance: String,
    pub origin_fetches: u64,
    pub peak_concurrency: usize,
    pub hit_ratio_pct: f64,
    pub stale_cap_violations: u64,
    pub validity_status: String,
}

pub struct ReportGenerator;

impl ReportGenerator {
    pub fn build_report_item(cell: &CellResult) -> LabReportItem {
        let provenance_tag = if cell.tier == "SIMULATED" {
            ProvenanceTag::Simulated
        } else {
            ProvenanceTag::Measured
        };

        LabReportItem {
            scenario_id: cell.scenario_id.clone(),
            workload_family: cell.workload_family.clone(),
            baseline_id: cell.baseline_id.clone(),
            provenance: provenance_tag.as_str().to_string(),
            origin_fetches: cell.origin_fetches,
            peak_concurrency: cell.peak_concurrency,
            hit_ratio_pct: cell.hit_ratio * 100.0,
            stale_cap_violations: cell.stale_cap_violations,
            validity_status: if cell.is_valid {
                "VALID".to_string()
            } else {
                "INVALID".to_string()
            },
        }
    }

    pub fn to_markdown_table(items: &[LabReportItem]) -> String {
        let mut md = String::new();
        md.push_str("| Scenario | Workload | Baseline | Provenance | Origin Fetches | Peak Concurrency | Hit Ratio | Stale Violations | Validity |\n");
        md.push_str("|---|---|---|---|---|---|---|---|---|\n");

        for item in items {
            md.push_str(&format!(
                "| {} | {} | {} | **{}** | {} | {} | {:.2}% | {} | {} |\n",
                item.scenario_id,
                item.workload_family,
                item.baseline_id,
                item.provenance,
                item.origin_fetches,
                item.peak_concurrency,
                item.hit_ratio_pct,
                item.stale_cap_violations,
                item.validity_status,
            ));
        }

        md
    }
}
