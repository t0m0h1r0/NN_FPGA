//! VLIWインストラクションセットの定義

/// VLIWコマンドの列挙型
#[derive(Debug, Clone, Copy)]
pub enum VliwCommand {
    Nop,
    LoadV0,
    LoadV1,
    LoadM0,
    StoreV0,
    StoreV1,
    StoreM0,
    ZeroV0,
    ZeroV1,
    ZeroM0,
    PushV0,
    PopV1,
    PopV0, // 【新規追加】
    MatrixVectorMultiply,
    VectorAdd01,
    VectorSub01,
    VectorReLU,
    VectorTanh,
    VectorSquare,
}

/// VLIWインストラクション
#[derive(Debug, Clone)]
pub struct VliwInstruction {
    pub op1: VliwCommand,
    pub op2: VliwCommand,
    pub op3: VliwCommand,
    pub op4: VliwCommand,
}

impl VliwInstruction {
    /// NOPインストラクションを生成
    pub fn nop() -> Self {
        Self {
            op1: VliwCommand::Nop,
            op2: VliwCommand::Nop,
            op3: VliwCommand::Nop,
            op4: VliwCommand::Nop,
        }
    }

    /// 単一の命令を含むインストラクションを生成
    pub fn single(op: VliwCommand) -> Self {
        Self {
            op1: op,
            op2: VliwCommand::Nop,
            op3: VliwCommand::Nop,
            op4: VliwCommand::Nop,
        }
    }
}

/// VLIWインストラクションのビルダー
pub struct InstructionBuilder {
    inst: VliwInstruction,
    current_slot: usize,
}

impl InstructionBuilder {
    /// 新しいビルダーを作成
    pub fn new() -> Self {
        Self {
            inst: VliwInstruction::nop(),
            current_slot: 0,
        }
    }

    /// 命令を追加
    pub fn add_op(&mut self, op: VliwCommand) -> &mut Self {
        if self.current_slot < 4 {
            match self.current_slot {
                0 => self.inst.op1 = op,
                1 => self.inst.op2 = op,
                2 => self.inst.op3 = op,
                3 => self.inst.op4 = op,
                _ => {}
            }
            self.current_slot += 1;
        }
        self
    }

    /// インストラクションを生成
    pub fn build(self) -> VliwInstruction {
        self.inst
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_builder() {
        let inst = InstructionBuilder::new()
            .add_op(VliwCommand::LoadV0)
            .add_op(VliwCommand::MatrixVectorMultiply)
            .add_op(VliwCommand::StoreV0)
            .build();

        assert!(matches!(inst.op1, VliwCommand::LoadV0));
        assert!(matches!(inst.op2, VliwCommand::MatrixVectorMultiply));
        assert!(matches!(inst.op3, VliwCommand::StoreV0));
        assert!(matches!(inst.op4, VliwCommand::Nop));
    }

    #[test]
    fn test_single_instruction() {
        let inst = VliwInstruction::single(VliwCommand::MatrixVectorMultiply);
        assert!(matches!(inst.op1, VliwCommand::MatrixVectorMultiply));
        assert!(matches!(inst.op2, VliwCommand::Nop));
        assert!(matches!(inst.op3, VliwCommand::Nop));
        assert!(matches!(inst.op4, VliwCommand::Nop));
    }

    // PopV0のテストを追加
    #[test]
    fn test_pop_v0_instruction() {
        let inst = VliwInstruction::single(VliwCommand::PopV0);
        assert!(matches!(inst.op1, VliwCommand::PopV0));
        assert!(matches!(inst.op2, VliwCommand::Nop));
        assert!(matches!(inst.op3, VliwCommand::Nop));
        assert!(matches!(inst.op4, VliwCommand::Nop));
    }
}