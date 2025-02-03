use crate::types::FpgaValue;

/// 共有メモリのエントリ
#[derive(Debug, Clone)]
pub struct SharedMemoryEntry {
    pub data: Vec<FpgaValue>,
    pub valid: bool,
}

/// 共有メモリの実装
pub struct SharedMemory {
    entries: Vec<SharedMemoryEntry>,
    size: usize,
}

impl SharedMemory {
    pub fn new(num_units: usize) -> Self {
        let entries = (0..num_units)
            .map(|_| SharedMemoryEntry {
                data: Vec::new(),
                valid: false,
            })
            .collect();

        Self {
            entries,
            size: num_units,
        }
    }

    pub fn write(&mut self, unit_id: usize, data: Vec<FpgaValue>) -> bool {
        if unit_id < self.size {
            self.entries[unit_id] = SharedMemoryEntry {
                data,
                valid: true,
            };
            true
        } else {
            false
        }
    }

    pub fn read(&self, unit_id: usize) -> Option<&Vec<FpgaValue>> {
        if unit_id < self.size && self.entries[unit_id].valid {
            Some(&self.entries[unit_id].data)
        } else {
            None
        }
    }

    pub fn invalidate(&mut self, unit_id: usize) {
        if unit_id < self.size {
            self.entries[unit_id].valid = false;
        }
    }
    
    pub fn get_entries_mut(&mut self) -> &mut [SharedMemoryEntry] {
        &mut self.entries
    }
}