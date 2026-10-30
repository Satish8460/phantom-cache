use std::hash::{Hash, Hasher};

pub struct CountMinSketch {
    depth: usize,
    width: usize,
    table: Vec<Vec<u32>>,
}

impl CountMinSketch {
    pub fn new(depth: usize, width: usize) -> Self {
        Self {
            depth,
            width,
            table: vec![vec![0; width]; depth],
        }
    }

    fn hash(&self, item: u64, i: usize) -> usize {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        i.hash(&mut hasher);
        (hasher.finish() as usize) % self.width
    }

    pub fn increment(&mut self, item: u64) {
        for i in 0..self.depth {
            let col = self.hash(item, i);
            self.table[i][col] = self.table[i][col].saturating_add(1);
        }
    }

    pub fn estimate(&self, item: u64) -> u32 {
        let mut min_val = u32::MAX;
        for i in 0..self.depth {
            let col = self.hash(item, i);
            min_val = min_val.min(self.table[i][col]);
        }
        min_val
    }

    pub fn clear(&mut self) {
        for row in &mut self.table {
            row.fill(0);
        }
    }
}
