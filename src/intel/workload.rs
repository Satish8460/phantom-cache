#[derive(Debug, Clone)]
pub struct WorkloadProfile {
    pub name: String,
    pub target_rate_per_sec: f64,
    pub zipf_skew: f64,
    pub catalog_size: usize,
    pub read_ratio: f32,
}

impl WorkloadProfile {
    pub fn travel_hotel() -> Self {
        Self {
            name: "Travel/Hotel".to_string(),
            target_rate_per_sec: 15000.0,
            zipf_skew: 0.95,
            catalog_size: 10000,
            read_ratio: 0.95,
        }
    }

    pub fn ecommerce() -> Self {
        Self {
            name: "E-commerce".to_string(),
            target_rate_per_sec: 25000.0,
            zipf_skew: 1.1,
            catalog_size: 50000,
            read_ratio: 0.9,
        }
    }

    pub fn ticketing() -> Self {
        Self {
            name: "Ticketing".to_string(),
            target_rate_per_sec: 50000.0,
            zipf_skew: 1.35,
            catalog_size: 1000,
            read_ratio: 0.99,
        }
    }

    pub fn general_api() -> Self {
        Self {
            name: "General API".to_string(),
            target_rate_per_sec: 10000.0,
            zipf_skew: 0.7,
            catalog_size: 100000,
            read_ratio: 0.8,
        }
    }
}
