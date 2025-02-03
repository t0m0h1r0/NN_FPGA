//! 共有メモリの実装

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
    /// 新しい共有メモリを作成
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

    /// メモリにデータを書き込み
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

    /// メモリからデータを読み出し
    pub fn read(&self, unit_id: usize) -> Option<&Vec<FpgaValue>> {
        if unit_id < self.size && self.entries[unit_id].valid {
            Some(&self.entries[unit_id].data)
        } else {
            None
        }
    }

    /// エントリを無効化
    pub fn invalidate(&mut self, unit_id: usize) {
        if unit_id < self.size {
            self.entries[unit_id].valid = false;
        }
    }

    /// エントリへの可変参照を取得
    pub fn get_entries_mut(&mut self) -> &mut [SharedMemoryEntry] {
        &mut self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataConversionType;

    #[test]
    fn test_shared_memory_operations() {
        let mut memory = SharedMemory::new(4);
        
        // テストデータ作成
        let test_data = vec![
            FpgaValue::from_f32(1.0, DataConversionType::Full),
            FpgaValue::from_f32(2.0, DataConversionType::Full),
        ];

        // 書き込みテスト
        assert!(memory.write(0, test_data.clone()));
        assert!(!memory.write(4, test_data.clone())); // 範囲外

        // 読み出しテスト
        let read_data = memory.read(0).unwrap();
        assert_eq!(read_data.len(), 2);
        assert!(memory.read(4).is_none()); // 範囲外

        // 無効化テスト
        memory.invalidate(0);
        assert!(memory.read(0).is_none());
    }
}